//! Main application window: phase pill, big countdown, cycle dots and
//! transport controls.

use adw::prelude::*;
use gtk::gio;

use crate::timer::{Phase, State, Timer};

const CSS: &str = "
.timer-label {
    font-size: 72px;
    font-weight: 300;
    font-feature-settings: 'tnum';
}
.phase-pill {
    padding: 4px 14px;
    border-radius: 999px;
    font-weight: bold;
    background: alpha(currentColor, 0.1);
}
.phase-focus { color: @red_3; }
.phase-break { color: @green_4; }
.phase-long-break { color: @blue_3; }
.cycle-dot {
    font-size: 12px;
    color: alpha(currentColor, 0.25);
}
.cycle-dot.done { color: @accent_color; }
.big-circular {
    min-width: 64px;
    min-height: 64px;
}
";

pub fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub struct MainView {
    pub window: adw::ApplicationWindow,
    pub toggle_button: gtk::Button,
    pub skip_button: gtk::Button,
    pub reset_button: gtk::Button,
    phase_label: gtk::Label,
    time_label: gtk::Label,
    dots_box: gtk::Box,
}

impl MainView {
    pub fn new(app: &adw::Application) -> Self {
        let phase_label = gtk::Label::builder()
            .label(Phase::Focus.label())
            .css_classes(["phase-pill", "phase-focus"])
            .halign(gtk::Align::Center)
            .build();

        let time_label = gtk::Label::builder()
            .label("25:00")
            .css_classes(["timer-label"])
            .build();

        let dots_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .halign(gtk::Align::Center)
            .build();

        let reset_button = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .css_classes(["circular"])
            .valign(gtk::Align::Center)
            .tooltip_text("Reset")
            .build();

        let toggle_button = gtk::Button::builder()
            .icon_name("media-playback-start-symbolic")
            .css_classes(["circular", "suggested-action", "big-circular"])
            .tooltip_text("Start")
            .build();

        let skip_button = gtk::Button::builder()
            .icon_name("media-skip-forward-symbolic")
            .css_classes(["circular"])
            .valign(gtk::Align::Center)
            .tooltip_text("Skip")
            .build();

        let controls = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(18)
            .halign(gtk::Align::Center)
            .build();
        controls.append(&reset_button);
        controls.append(&toggle_button);
        controls.append(&skip_button);

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .valign(gtk::Align::Center)
            .vexpand(true)
            .margin_top(12)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();
        content.append(&phase_label);
        content.append(&time_label);
        content.append(&dots_box);
        content.append(&controls);

        let menu = gio::Menu::new();
        menu.append(Some("_Statistics"), Some("app.stats"));
        menu.append(Some("_Preferences"), Some("app.preferences"));
        menu.append(Some("_About Blue Time"), Some("app.about"));
        menu.append(Some("_Quit"), Some("app.quit"));

        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .build();

        let header = adw::HeaderBar::new();
        header.pack_end(&menu_button);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.set_content(Some(&content));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Blue Time")
            .default_width(360)
            .default_height(400)
            .content(&toolbar_view)
            .build();
        // Closing the window keeps the timer running in the background;
        // Quit (menu or indicator) is what actually exits.
        window.set_hide_on_close(true);

        Self {
            window,
            toggle_button,
            skip_button,
            reset_button,
            phase_label,
            time_label,
            dots_box,
        }
    }

    pub fn refresh(&self, timer: &Timer) {
        let remaining = timer.remaining();
        self.time_label
            .set_label(&format!("{:02}:{:02}", remaining / 60, remaining % 60));

        let phase = timer.phase();
        self.phase_label.set_label(phase.label());
        let phase_class = match phase {
            Phase::Focus => "phase-focus",
            Phase::ShortBreak => "phase-break",
            Phase::LongBreak => "phase-long-break",
        };
        self.phase_label
            .set_css_classes(&["phase-pill", phase_class]);

        let (icon, tip) = match timer.state() {
            State::Running => ("media-playback-pause-symbolic", "Pause"),
            State::Paused => ("media-playback-start-symbolic", "Resume"),
            State::Idle => ("media-playback-start-symbolic", "Start"),
        };
        self.toggle_button.set_icon_name(icon);
        self.toggle_button.set_tooltip_text(Some(tip));

        self.refresh_dots(timer);
    }

    fn refresh_dots(&self, timer: &Timer) {
        while let Some(child) = self.dots_box.first_child() {
            self.dots_box.remove(&child);
        }
        let total = timer.durations().sessions_until_long_break;
        let done = timer.cycle_count();
        for i in 0..total {
            let dot = gtk::Label::new(Some("●"));
            dot.add_css_class("cycle-dot");
            if i < done {
                dot.add_css_class("done");
            }
            self.dots_box.append(&dot);
        }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

pub fn show_about(parent: &adw::ApplicationWindow) {
    let about = adw::AboutDialog::builder()
        .application_name("Blue Time")
        .application_icon("io.github.vlucas14sp.BlueTime")
        .version(env!("CARGO_PKG_VERSION"))
        .developer_name("vlucas14sp")
        .license_type(gtk::License::MitX11)
        .website("https://github.com/vlucas14sp/blue-time")
        .issue_url("https://github.com/vlucas14sp/blue-time/issues")
        .build();
    about.present(Some(parent));
}
