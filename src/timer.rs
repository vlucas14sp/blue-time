//! Pure Pomodoro timer state machine, independent of any UI.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Focus => "Focus",
            Phase::ShortBreak => "Short Break",
            Phase::LongBreak => "Long Break",
        }
    }

    pub fn is_break(self) -> bool {
        !matches!(self, Phase::Focus)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Running,
    Paused,
}

/// Session durations in seconds plus the long-break cadence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Durations {
    pub focus: u32,
    pub short_break: u32,
    pub long_break: u32,
    pub sessions_until_long_break: u32,
}

impl Default for Durations {
    fn default() -> Self {
        Self {
            focus: 25 * 60,
            short_break: 5 * 60,
            long_break: 15 * 60,
            sessions_until_long_break: 4,
        }
    }
}

impl Durations {
    fn for_phase(&self, phase: Phase) -> u32 {
        match phase {
            Phase::Focus => self.focus,
            Phase::ShortBreak => self.short_break,
            Phase::LongBreak => self.long_break,
        }
    }
}

/// What happened as a result of a `tick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    /// Time is still counting down.
    Counting,
    /// The phase that just finished; the timer has advanced to the next
    /// phase and is now idle (the caller decides whether to auto-start).
    Finished(Phase),
}

#[derive(Debug, Clone)]
pub struct Timer {
    durations: Durations,
    phase: Phase,
    state: State,
    remaining: u32,
    /// Focus sessions completed in the current long-break cycle.
    cycle_count: u32,
}

impl Timer {
    pub fn new(durations: Durations) -> Self {
        Self {
            durations,
            phase: Phase::Focus,
            state: State::Idle,
            remaining: durations.focus,
            cycle_count: 0,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn remaining(&self) -> u32 {
        self.remaining
    }

    pub fn durations(&self) -> Durations {
        self.durations
    }

    /// Focus sessions completed in the current cycle (0-based, wraps at
    /// `sessions_until_long_break`).
    pub fn cycle_count(&self) -> u32 {
        self.cycle_count
    }

    /// Fraction of the current phase already elapsed, in `0.0..=1.0`.
    pub fn progress(&self) -> f64 {
        let total = self.durations.for_phase(self.phase).max(1);
        1.0 - f64::from(self.remaining) / f64::from(total)
    }

    pub fn start(&mut self) {
        if self.remaining == 0 {
            self.remaining = self.durations.for_phase(self.phase);
        }
        self.state = State::Running;
    }

    pub fn pause(&mut self) {
        if self.state == State::Running {
            self.state = State::Paused;
        }
    }

    /// Start when idle, otherwise flip between running and paused.
    pub fn toggle(&mut self) {
        match self.state {
            State::Running => self.state = State::Paused,
            State::Idle | State::Paused => self.start(),
        }
    }

    /// Reset the current phase to its full duration and stop counting.
    pub fn reset(&mut self) {
        self.state = State::Idle;
        self.remaining = self.durations.for_phase(self.phase);
    }

    /// Abandon the current phase and move to the next one (idle).
    pub fn skip(&mut self) {
        self.advance();
    }

    /// Advance one second. Call this once per second while running.
    pub fn tick(&mut self) -> Tick {
        if self.state != State::Running {
            return Tick::Counting;
        }
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining > 0 {
            return Tick::Counting;
        }
        let finished = self.phase;
        self.advance();
        Tick::Finished(finished)
    }

    /// Apply new durations. When idle the current phase is refreshed to the
    /// new full duration; a running or paused countdown is left untouched.
    pub fn set_durations(&mut self, durations: Durations) {
        self.durations = durations;
        if self.state == State::Idle {
            self.remaining = self.durations.for_phase(self.phase);
        }
    }

    fn advance(&mut self) {
        self.phase = match self.phase {
            Phase::Focus => {
                self.cycle_count += 1;
                if self.cycle_count >= self.durations.sessions_until_long_break {
                    Phase::LongBreak
                } else {
                    Phase::ShortBreak
                }
            }
            Phase::ShortBreak => Phase::Focus,
            Phase::LongBreak => {
                self.cycle_count = 0;
                Phase::Focus
            }
        };
        self.state = State::Idle;
        self.remaining = self.durations.for_phase(self.phase);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short() -> Durations {
        Durations {
            focus: 3,
            short_break: 2,
            long_break: 4,
            sessions_until_long_break: 2,
        }
    }

    fn run_to_completion(timer: &mut Timer) -> Phase {
        timer.start();
        loop {
            if let Tick::Finished(phase) = timer.tick() {
                return phase;
            }
        }
    }

    #[test]
    fn starts_idle_on_focus() {
        let t = Timer::new(short());
        assert_eq!(t.phase(), Phase::Focus);
        assert_eq!(t.state(), State::Idle);
        assert_eq!(t.remaining(), 3);
    }

    #[test]
    fn tick_does_nothing_unless_running() {
        let mut t = Timer::new(short());
        assert_eq!(t.tick(), Tick::Counting);
        assert_eq!(t.remaining(), 3);
    }

    #[test]
    fn counts_down_and_finishes_focus() {
        let mut t = Timer::new(short());
        t.start();
        assert_eq!(t.tick(), Tick::Counting);
        assert_eq!(t.tick(), Tick::Counting);
        assert_eq!(t.tick(), Tick::Finished(Phase::Focus));
        assert_eq!(t.phase(), Phase::ShortBreak);
        assert_eq!(t.state(), State::Idle);
        assert_eq!(t.remaining(), 2);
    }

    #[test]
    fn long_break_after_configured_sessions() {
        let mut t = Timer::new(short());
        assert_eq!(run_to_completion(&mut t), Phase::Focus);
        assert_eq!(t.phase(), Phase::ShortBreak);
        assert_eq!(run_to_completion(&mut t), Phase::ShortBreak);
        assert_eq!(run_to_completion(&mut t), Phase::Focus);
        assert_eq!(t.phase(), Phase::LongBreak);
        assert_eq!(run_to_completion(&mut t), Phase::LongBreak);
        assert_eq!(t.phase(), Phase::Focus);
        assert_eq!(t.cycle_count(), 0);
    }

    #[test]
    fn pause_and_resume() {
        let mut t = Timer::new(short());
        t.start();
        t.tick();
        t.pause();
        assert_eq!(t.state(), State::Paused);
        assert_eq!(t.tick(), Tick::Counting);
        assert_eq!(t.remaining(), 2);
        t.toggle();
        assert_eq!(t.state(), State::Running);
    }

    #[test]
    fn reset_restores_full_phase() {
        let mut t = Timer::new(short());
        t.start();
        t.tick();
        t.reset();
        assert_eq!(t.state(), State::Idle);
        assert_eq!(t.remaining(), 3);
        assert_eq!(t.phase(), Phase::Focus);
    }

    #[test]
    fn skip_advances_cycle() {
        let mut t = Timer::new(short());
        t.skip();
        assert_eq!(t.phase(), Phase::ShortBreak);
        assert_eq!(t.cycle_count(), 1);
        t.skip();
        assert_eq!(t.phase(), Phase::Focus);
    }

    #[test]
    fn set_durations_refreshes_idle_phase() {
        let mut t = Timer::new(short());
        let mut d = short();
        d.focus = 10;
        t.set_durations(d);
        assert_eq!(t.remaining(), 10);

        t.start();
        t.tick();
        d.focus = 20;
        t.set_durations(d);
        assert_eq!(t.remaining(), 9);
    }

    #[test]
    fn progress_moves_from_zero_to_one() {
        let mut t = Timer::new(short());
        assert!(t.progress().abs() < f64::EPSILON);
        t.start();
        t.tick();
        assert!((t.progress() - 1.0 / 3.0).abs() < 1e-9);
    }
}
