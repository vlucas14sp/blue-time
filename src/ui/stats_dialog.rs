//! Statistics dialog: today's totals and a 7-day history.

use adw::prelude::*;
use gettextrs::{gettext, ngettext};
use gtk::glib;

use crate::stats::Stats;

pub fn present(parent: &adw::ApplicationWindow, stats: &Stats) {
    let today = stats.completed_today();
    let focus_secs = stats.focus_seconds_today();

    let headline = gtk::Label::builder()
        .label(format!("{today}"))
        .css_classes(["timer-label"])
        .build();
    let subtitle = gtk::Label::builder()
        .label(format!(
            "{} — {}",
            ngettext(
                "pomodoro completed today",
                "pomodoros completed today",
                today as u32
            ),
            gettext("%s of focus").replace("%s", &format_duration(focus_secs))
        ))
        .css_classes(["dim-label"])
        .build();

    let history = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    for (day_start, count) in stats.daily_counts(7) {
        let row = adw::ActionRow::builder()
            .title(day_label(day_start))
            .build();
        let count_label = gtk::Label::builder()
            .label(format!("{count}"))
            .css_classes(if count > 0 { ["accent"] } else { ["dim-label"] })
            .build();
        row.add_suffix(&count_label);
        history.append(&row);
    }

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    content.append(&headline);
    content.append(&subtitle);
    content.append(&history);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&content));

    let dialog = adw::Dialog::builder()
        .title(gettext("Statistics"))
        .content_width(340)
        .child(&toolbar_view)
        .build();
    dialog.present(Some(parent));
}

fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}min")
    } else {
        format!("{minutes}min")
    }
}

fn day_label(unix_day_start: u64) -> String {
    glib::DateTime::from_unix_local(unix_day_start as i64)
        .map(|dt| {
            dt.format("%a, %b %e")
                .map(|s| s.to_string())
                .unwrap_or_default()
        })
        .unwrap_or_default()
}
