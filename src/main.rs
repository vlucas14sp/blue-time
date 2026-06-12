mod autostart;
mod config;
mod icon;
mod stats;
mod timer;
mod tray;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use adw::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

use config::Config;
use stats::Stats;
use timer::{Phase, Tick, Timer};
use ui::window::MainView;

const APP_ID: &str = "io.github.vlucas14sp.BlueTime";
const GETTEXT_DOMAIN: &str = "blue-time";

/// Set by `--hidden`: start in the background (indicator only).
static START_HIDDEN: AtomicBool = AtomicBool::new(false);

struct App {
    gtk_app: adw::Application,
    view: MainView,
    timer: Timer,
    config: Rc<RefCell<Config>>,
    stats: Stats,
    tray: ksni::blocking::Handle<tray::Indicator>,
    /// Keeps the sound alive while it plays.
    sound: Option<gtk::MediaFile>,
    /// Keeps the application alive while the window is hidden.
    _hold: gio::ApplicationHoldGuard,
}

impl App {
    fn refresh(&mut self) {
        self.view.refresh(&self.timer);
        let timer = self.timer.clone();
        self.tray.update(move |indicator| indicator.sync(&timer));
    }

    fn handle_command(&mut self, command: tray::Command) {
        match command {
            tray::Command::Toggle => self.timer.toggle(),
            tray::Command::Skip => self.timer.skip(),
            tray::Command::Reset => self.timer.reset(),
            tray::Command::ShowWindow => self.view.present(),
            tray::Command::Quit => self.gtk_app.quit(),
        }
        self.refresh();
    }

    fn on_tick(&mut self) {
        if let Tick::Finished(finished) = self.timer.tick() {
            self.on_phase_finished(finished);
        }
        self.refresh();
    }

    fn on_phase_finished(&mut self, finished: Phase) {
        let config = self.config.borrow().clone();
        if finished == Phase::Focus {
            self.stats.record_focus(config.durations().focus);
        }

        let next = self.timer.phase();
        let next_minutes = self.timer.remaining() / 60;
        let (title, body) = match finished {
            Phase::Focus => (
                gettext("Focus session complete"),
                gettext("Time for a %s (%d min)")
                    .replace("%s", &gettext(next.label()).to_lowercase())
                    .replace("%d", &next_minutes.to_string()),
            ),
            _ => (
                gettext("Break is over"),
                gettext("Time to focus (%d min)").replace("%d", &next_minutes.to_string()),
            ),
        };
        ui::window::notify(&self.gtk_app, &title, &body);

        if config.play_sound {
            self.play_sound();
        }

        let auto = match finished {
            Phase::Focus => config.auto_start_breaks,
            _ => config.auto_start_focus,
        };
        if auto {
            self.timer.start();
        }
    }

    fn play_sound(&mut self) {
        const SOUND: &str = "/usr/share/sounds/freedesktop/stereo/complete.oga";
        if std::path::Path::new(SOUND).exists() {
            let media = gtk::MediaFile::for_filename(SOUND);
            media.play();
            self.sound = Some(media);
        } else if let Some(display) = gtk::gdk::Display::default() {
            display.beep();
        }
    }
}

fn build_app(gtk_app: &adw::Application) -> Rc<RefCell<App>> {
    let config = Rc::new(RefCell::new(Config::load()));
    let timer = Timer::new(config.borrow().durations());
    let view = MainView::new(gtk_app);

    let (tx, rx) = async_channel::unbounded();
    let indicator = tray::Indicator::new(&timer, tx);
    use ksni::blocking::TrayMethods;
    let tray = indicator.spawn().expect("spawn tray service");

    let app = Rc::new(RefCell::new(App {
        gtk_app: gtk_app.clone(),
        view,
        timer,
        config,
        stats: Stats::load(),
        tray,
        sound: None,
        _hold: gtk_app.hold(),
    }));

    // Commands coming from the top bar indicator.
    {
        let a = app.clone();
        glib::spawn_future_local(async move {
            while let Ok(command) = rx.recv().await {
                a.borrow_mut().handle_command(command);
            }
        });
    }

    // Window controls.
    {
        let a = app.clone();
        app.borrow().view.toggle_button.connect_clicked(move |_| {
            let mut a = a.borrow_mut();
            a.timer.toggle();
            a.refresh();
        });
        let a = app.clone();
        app.borrow().view.skip_button.connect_clicked(move |_| {
            let mut a = a.borrow_mut();
            a.timer.skip();
            a.refresh();
        });
        let a = app.clone();
        app.borrow().view.reset_button.connect_clicked(move |_| {
            let mut a = a.borrow_mut();
            a.timer.reset();
            a.refresh();
        });
    }

    // One-second heartbeat.
    {
        let a = app.clone();
        glib::timeout_add_seconds_local(1, move || {
            a.borrow_mut().on_tick();
            glib::ControlFlow::Continue
        });
    }

    // App actions (menu).
    let quit = gio::ActionEntry::builder("quit")
        .activate(|gtk_app: &adw::Application, _, _| gtk_app.quit())
        .build();
    let preferences = {
        let a = app.clone();
        gio::ActionEntry::builder("preferences")
            .activate(move |_, _, _| {
                let (window, cfg) = {
                    let app = a.borrow();
                    (app.view.window.clone(), app.config.clone())
                };
                let a = a.clone();
                ui::prefs::present(
                    &window,
                    cfg,
                    Rc::new(move |config: &Config| {
                        let mut app = a.borrow_mut();
                        app.timer.set_durations(config.durations());
                        app.refresh();
                    }),
                );
            })
            .build()
    };
    let stats_action = {
        let a = app.clone();
        gio::ActionEntry::builder("stats")
            .activate(move |_, _, _| {
                let app = a.borrow();
                ui::stats_dialog::present(&app.view.window, &app.stats);
            })
            .build()
    };
    let about = {
        let a = app.clone();
        gio::ActionEntry::builder("about")
            .activate(move |_, _, _| {
                ui::window::show_about(&a.borrow().view.window);
            })
            .build()
    };
    gtk_app.add_action_entries([quit, preferences, stats_action, about]);
    gtk_app.set_accels_for_action("app.quit", &["<primary>q"]);

    app.borrow_mut().refresh();
    app
}

/// Initialize gettext, looking for catalogs relative to the installed
/// binary (`<prefix>/share/locale`) with a system-wide fallback.
fn init_i18n() {
    use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};

    setlocale(LocaleCategory::LcAll, "");
    let localedir = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.parent()?.join("share/locale")))
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(|| "/usr/share/locale".into());
    let _ = bindtextdomain(GETTEXT_DOMAIN, localedir);
    let _ = bind_textdomain_codeset(GETTEXT_DOMAIN, "UTF-8");
    let _ = textdomain(GETTEXT_DOMAIN);
}

fn main() -> glib::ExitCode {
    init_i18n();
    let gtk_app = adw::Application::builder().application_id(APP_ID).build();

    gtk_app.add_main_option(
        "hidden",
        glib::Char::from(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        &gettext("Start in the background (indicator only)"),
        None,
    );
    gtk_app.connect_handle_local_options(|_, options| {
        if options.contains("hidden") {
            START_HIDDEN.store(true, Ordering::Relaxed);
        }
        std::ops::ControlFlow::Continue(()) // continue normal startup
    });

    gtk_app.connect_startup(|_| ui::window::load_css());

    let state: Rc<RefCell<Option<Rc<RefCell<App>>>>> = Rc::new(RefCell::new(None));
    gtk_app.connect_activate(move |gtk_app| {
        let mut state = state.borrow_mut();
        match state.as_ref() {
            Some(app) => app.borrow().view.present(),
            None => {
                let app = build_app(gtk_app);
                if !START_HIDDEN.swap(false, Ordering::Relaxed) {
                    app.borrow().view.present();
                }
                *state = Some(app);
            }
        }
    });

    gtk_app.run()
}
