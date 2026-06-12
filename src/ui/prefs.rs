//! Preferences dialog: session durations, behavior toggles and autostart.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use crate::autostart;
use crate::config::Config;

/// Builds and presents the preferences dialog. `on_change` runs with the
/// updated config every time the user edits a setting.
pub fn present(
    parent: &adw::ApplicationWindow,
    config: Rc<RefCell<Config>>,
    on_change: Rc<dyn Fn(&Config)>,
) {
    let dialog = adw::PreferencesDialog::builder().title("Preferences").build();
    let page = adw::PreferencesPage::new();

    let durations = adw::PreferencesGroup::builder().title("Durations").build();

    let cfg = config.borrow().clone();
    let focus = spin_row("Focus", "Minutes per focus session", 1.0, 180.0, cfg.focus_minutes);
    let short_break = spin_row("Short Break", "Minutes per short break", 1.0, 60.0, cfg.short_break_minutes);
    let long_break = spin_row("Long Break", "Minutes per long break", 1.0, 120.0, cfg.long_break_minutes);
    let cadence = spin_row(
        "Sessions Until Long Break",
        "Focus sessions before a long break",
        1.0,
        12.0,
        cfg.sessions_until_long_break,
    );
    durations.add(&focus);
    durations.add(&short_break);
    durations.add(&long_break);
    durations.add(&cadence);

    let behavior = adw::PreferencesGroup::builder().title("Behavior").build();
    let auto_breaks = switch_row(
        "Auto-Start Breaks",
        "Begin breaks as soon as a focus session ends",
        cfg.auto_start_breaks,
    );
    let auto_focus = switch_row(
        "Auto-Start Focus",
        "Begin the next focus session as soon as a break ends",
        cfg.auto_start_focus,
    );
    let sound = switch_row("Sound", "Play a sound when a session ends", cfg.play_sound);
    behavior.add(&auto_breaks);
    behavior.add(&auto_focus);
    behavior.add(&sound);

    let system = adw::PreferencesGroup::builder().title("System").build();
    let autostart_row = switch_row(
        "Start on Login",
        "Launch Blue Time in the background when you log in",
        autostart::is_enabled(),
    );
    system.add(&autostart_row);

    page.add(&durations);
    page.add(&behavior);
    page.add(&system);
    dialog.add(&page);

    let apply = {
        let config = config.clone();
        let focus = focus.clone();
        let short_break = short_break.clone();
        let long_break = long_break.clone();
        let cadence = cadence.clone();
        let auto_breaks = auto_breaks.clone();
        let auto_focus = auto_focus.clone();
        let sound = sound.clone();
        Rc::new(move || {
            let mut cfg = config.borrow_mut();
            cfg.focus_minutes = focus.value() as u32;
            cfg.short_break_minutes = short_break.value() as u32;
            cfg.long_break_minutes = long_break.value() as u32;
            cfg.sessions_until_long_break = cadence.value() as u32;
            cfg.auto_start_breaks = auto_breaks.is_active();
            cfg.auto_start_focus = auto_focus.is_active();
            cfg.play_sound = sound.is_active();
            cfg.save();
            let snapshot = cfg.clone();
            drop(cfg);
            on_change(&snapshot);
        })
    };

    for row in [&focus, &short_break, &long_break, &cadence] {
        let apply = apply.clone();
        row.connect_value_notify(move |_| apply());
    }
    for row in [&auto_breaks, &auto_focus, &sound] {
        let apply = apply.clone();
        row.connect_active_notify(move |_| apply());
    }
    autostart_row.connect_active_notify(|row| {
        autostart::set_enabled(row.is_active());
    });

    dialog.present(Some(parent));
}

fn spin_row(title: &str, subtitle: &str, min: f64, max: f64, value: u32) -> adw::SpinRow {
    let row = adw::SpinRow::with_range(min, max, 1.0);
    row.set_title(title);
    row.set_subtitle(subtitle);
    row.set_value(f64::from(value));
    row
}

fn switch_row(title: &str, subtitle: &str, active: bool) -> adw::SwitchRow {
    adw::SwitchRow::builder()
        .title(title)
        .subtitle(subtitle)
        .active(active)
        .build()
}
