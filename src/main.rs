mod config;
mod timer;
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
    view: MainView,
    timer: Timer,
}

impl App {
    fn refresh(&mut self) {
        self.view.refresh(&self.timer);
    }

    fn on_tick(&mut self) {
        if let Tick::Finished(_finished) = self.timer.tick() {
            // Notifications, sound and auto-start will hook in here.
        }
        self.refresh();
    }
}

fn build_app(gtk_app: &adw::Application) -> Rc<RefCell<App>> {
    let config = Config::load();
    let timer = Timer::new(config.durations());
    let view = MainView::new(gtk_app);

    let app = Rc::new(RefCell::new(App { view, timer }));

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
    let about = {
        let a = app.clone();
        gio::ActionEntry::builder("about")
            .activate(move |_, _, _| {
                ui::window::show_about(&a.borrow().view.window);
            })
            .build()
    };
    gtk_app.add_action_entries([quit, about]);
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
