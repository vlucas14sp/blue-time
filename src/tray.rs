//! Top bar status indicator (StatusNotifierItem via ksni).
//!
//! The tray service runs on its own thread; menu actions are forwarded to
//! the GTK main loop through an async channel.

use gettextrs::gettext;

use crate::timer::{Phase, State, Timer};

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Toggle,
    Skip,
    Reset,
    ShowWindow,
    Quit,
}

pub struct Indicator {
    pub icons: Vec<ksni::Icon>,
    pub status_text: String,
    pub state: State,
    pub tx: async_channel::Sender<Command>,
}

impl Indicator {
    pub fn new(timer: &Timer, tx: async_channel::Sender<Command>) -> Self {
        Self {
            icons: crate::icon::render(timer),
            status_text: status_text(timer),
            state: timer.state(),
            tx,
        }
    }

    /// Refresh everything shown in the bar from the current timer state.
    pub fn sync(&mut self, timer: &Timer) {
        self.icons = crate::icon::render(timer);
        self.status_text = status_text(timer);
        self.state = timer.state();
    }

    fn send(&self, command: Command) {
        let _ = self.tx.send_blocking(command);
    }
}

pub fn status_text(timer: &Timer) -> String {
    let remaining = timer.remaining();
    let text = format!(
        "{} — {:02}:{:02}",
        gettext(timer.phase().label()),
        remaining / 60,
        remaining % 60
    );
    match timer.state() {
        State::Paused => format!("{text} ({})", gettext("paused")),
        State::Idle if timer.phase() == Phase::Focus && timer.cycle_count() == 0 => {
            "Blue Time".into()
        }
        _ => text,
    }
}

impl ksni::Tray for Indicator {
    fn id(&self) -> String {
        "blue-time".into()
    }

    fn title(&self) -> String {
        self.status_text.clone()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icons.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.status_text.clone(),
            ..Default::default()
        }
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(Command::ShowWindow);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let toggle_label = match self.state {
            State::Running => gettext("Pause"),
            State::Paused => gettext("Resume"),
            State::Idle => gettext("Start"),
        };
        vec![
            StandardItem {
                label: self.status_text.clone(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: toggle_label,
                icon_name: match self.state {
                    State::Running => "media-playback-pause-symbolic".into(),
                    _ => "media-playback-start-symbolic".into(),
                },
                activate: Box::new(|this: &mut Self| this.send(Command::Toggle)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: gettext("Skip"),
                icon_name: "media-skip-forward-symbolic".into(),
                activate: Box::new(|this: &mut Self| this.send(Command::Skip)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: gettext("Reset"),
                icon_name: "view-refresh-symbolic".into(),
                activate: Box::new(|this: &mut Self| this.send(Command::Reset)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: gettext("Show Window"),
                activate: Box::new(|this: &mut Self| this.send(Command::ShowWindow)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: gettext("Quit"),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|this: &mut Self| this.send(Command::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}
