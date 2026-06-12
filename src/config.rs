//! Application settings, persisted as JSON under the XDG config directory.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::timer::Durations;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Focus session length in minutes.
    pub focus_minutes: u32,
    /// Short break length in minutes.
    pub short_break_minutes: u32,
    /// Long break length in minutes.
    pub long_break_minutes: u32,
    /// Number of focus sessions before a long break.
    pub sessions_until_long_break: u32,
    /// Automatically start breaks when a focus session ends.
    pub auto_start_breaks: bool,
    /// Automatically start the next focus session when a break ends.
    pub auto_start_focus: bool,
    /// Play a sound when a session ends.
    pub play_sound: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            sessions_until_long_break: 4,
            auto_start_breaks: false,
            auto_start_focus: false,
            play_sound: true,
        }
    }
}

impl Config {
    pub fn durations(&self) -> Durations {
        Durations {
            focus: self.focus_minutes * 60,
            short_break: self.short_break_minutes * 60,
            long_break: self.long_break_minutes * 60,
            sessions_until_long_break: self.sessions_until_long_break,
        }
    }

    fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("io.github", "vlucas14sp", "blue-time")
            .map(|dirs| dirs.config_dir().join("config.json"))
    }

    /// Load the saved config, falling back to defaults if missing or invalid.
    pub fn load() -> Self {
        Self::path()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_classic_pomodoro() {
        let c = Config::default();
        assert_eq!(c.focus_minutes, 25);
        assert_eq!(c.short_break_minutes, 5);
        assert_eq!(c.long_break_minutes, 15);
        assert_eq!(c.sessions_until_long_break, 4);
        assert!(c.play_sound);
        assert!(!c.auto_start_breaks);
    }

    #[test]
    fn durations_are_in_seconds() {
        let d = Config::default().durations();
        assert_eq!(d.focus, 25 * 60);
        assert_eq!(d.short_break, 5 * 60);
        assert_eq!(d.long_break, 15 * 60);
    }

    #[test]
    fn unknown_or_missing_fields_fall_back() {
        let parsed: Config = serde_json::from_str(r#"{"focus_minutes": 50}"#).unwrap();
        assert_eq!(parsed.focus_minutes, 50);
        assert_eq!(parsed.short_break_minutes, 5);

        let roundtrip: Config =
            serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
        assert_eq!(roundtrip, parsed);
    }
}
