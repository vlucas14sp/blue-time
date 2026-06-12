mod autostart;
mod config;
mod icon;
mod timer;
mod tray;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use config::Config;
use timer::{Tick, Timer};
use ui::window::MainView;

const APP_ID: &str = "io.github.vlucas14sp.BlueTime";

struct App {
    gtk_app: adw::Application,
    view: MainView,
    timer: Timer,
    config: Rc<RefCell<Config>>,
    tray: ksni::blocking::Handle<tray::Indicator>,
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
        if let Tick::Finished(_finished) = self.timer.tick() {
            // Notifications, sound and auto-start will hook in here.
        }
        self.refresh();
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
        tray,
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
    let about = {
        let a = app.clone();
        gio::ActionEntry::builder("about")
            .activate(move |_, _, _| {
                ui::window::show_about(&a.borrow().view.window);
            })
            .build()
    };
    gtk_app.add_action_entries([quit, preferences, about]);
    gtk_app.set_accels_for_action("app.quit", &["<primary>q"]);

    app.borrow_mut().refresh();
    app
}

fn main() -> glib::ExitCode {
    let gtk_app = adw::Application::builder().application_id(APP_ID).build();

    gtk_app.connect_startup(|_| ui::window::load_css());

    let state: Rc<RefCell<Option<Rc<RefCell<App>>>>> = Rc::new(RefCell::new(None));
    gtk_app.connect_activate(move |gtk_app| {
        let mut state = state.borrow_mut();
        match state.as_ref() {
            Some(app) => app.borrow().view.present(),
            None => {
                let app = build_app(gtk_app);
                app.borrow().view.present();
                *state = Some(app);
            }
        }
    });

    gtk_app.run()
}
