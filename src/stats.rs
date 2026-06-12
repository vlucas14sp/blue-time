//! Completed-session history, persisted as JSON under the XDG data directory.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gtk::glib;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unix timestamp (seconds) of when the session finished.
    pub finished_at: u64,
    /// Length of the completed session in seconds.
    pub duration: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    /// Completed focus sessions, oldest first.
    pub sessions: Vec<Session>,
}

impl Stats {
    fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("io.github", "vlucas14sp", "blue-time")
            .map(|dirs| dirs.data_dir().join("stats.json"))
    }

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

    /// Record a focus session that just finished.
    pub fn record_focus(&mut self, duration: u32) {
        self.sessions.push(Session {
            finished_at: now(),
            duration,
        });
        self.save();
    }

    /// Number of focus sessions completed since local midnight.
    pub fn completed_today(&self) -> usize {
        let midnight = local_midnight();
        self.sessions
            .iter()
            .filter(|s| s.finished_at >= midnight)
            .count()
    }

    /// Total focus seconds since local midnight.
    pub fn focus_seconds_today(&self) -> u64 {
        let midnight = local_midnight();
        self.sessions
            .iter()
            .filter(|s| s.finished_at >= midnight)
            .map(|s| u64::from(s.duration))
            .sum()
    }

    /// Per-day `(unix_day_start, completed_count)` for the most recent
    /// `days` local days, oldest first. Days with no sessions are included.
    pub fn daily_counts(&self, days: u32) -> Vec<(u64, usize)> {
        let midnight = local_midnight();
        (0..u64::from(days))
            .rev()
            .map(|back| {
                let start = midnight.saturating_sub(back * 86_400);
                let end = start + 86_400;
                let count = self
                    .sessions
                    .iter()
                    .filter(|s| s.finished_at >= start && s.finished_at < end)
                    .count();
                (start, count)
            })
            .collect()
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Unix timestamp of the most recent local midnight.
fn local_midnight() -> u64 {
    let now = now();
    let offset = utc_offset_seconds();
    let local = now.saturating_add_signed(offset);
    (local - local % 86_400).saturating_add_signed(-offset)
}

/// Current UTC offset in seconds, read from glibc via the `tm_gmtoff`
/// behaviour of `date`; falls back to UTC on failure.
fn utc_offset_seconds() -> i64 {
    // glib is already a dependency of the UI; use its local time support.
    let dt = glib::DateTime::now_local();
    dt.map(|d| d.utc_offset().as_seconds()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_today_only() {
        let mut stats = Stats::default();
        stats.sessions.push(Session {
            finished_at: now(),
            duration: 1500,
        });
        stats.sessions.push(Session {
            finished_at: now().saturating_sub(3 * 86_400),
            duration: 1500,
        });
        assert_eq!(stats.completed_today(), 1);
        assert_eq!(stats.focus_seconds_today(), 1500);
    }

    #[test]
    fn daily_counts_cover_requested_range() {
        let mut stats = Stats::default();
        stats.sessions.push(Session {
            finished_at: now(),
            duration: 1500,
        });
        let counts = stats.daily_counts(7);
        assert_eq!(counts.len(), 7);
        assert_eq!(counts.last().unwrap().1, 1);
        assert!(counts.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
