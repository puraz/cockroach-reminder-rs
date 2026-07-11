//! Break timer state machine. Ported from `src/main/timer.js`.
//!
//! The original drove itself with `setInterval`; here `tick()` is called once per
//! second from the iced subscription and returns a [`Transition`] the app reacts to.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Running,
    Break,
    Paused,
}

/// What happened on a `tick()` that the application needs to react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// Work time elapsed -> a break just started (summon the cockroaches).
    EnteredBreak,
    /// Break time elapsed -> work timer restarted (dismiss the cockroaches).
    EnteredRunning,
}

pub struct Timer {
    pub phase: Phase,
    pub remaining_ms: i64,
    pub interval_minutes: u32,
    pub duration_seconds: u32,
}

impl Timer {
    pub fn new(interval_minutes: u32, duration_seconds: u32) -> Self {
        Self {
            phase: Phase::Idle,
            remaining_ms: 0,
            interval_minutes,
            duration_seconds,
        }
    }

    pub fn start(&mut self) {
        self.remaining_ms = self.interval_minutes as i64 * 60 * 1000;
        self.phase = Phase::Running;
    }

    pub fn pause(&mut self) {
        if self.phase == Phase::Running {
            self.phase = Phase::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Running;
        }
    }

    pub fn trigger_break(&mut self) {
        self.phase = Phase::Break;
        self.remaining_ms = self.duration_seconds as i64 * 1000;
    }

    pub fn stop(&mut self) {
        self.phase = Phase::Idle;
        self.remaining_ms = 0;
    }

    /// Advance the timer by one second. Returns a [`Transition`] when a phase boundary is crossed.
    pub fn tick(&mut self) -> Option<Transition> {
        if self.phase != Phase::Running && self.phase != Phase::Break {
            return None;
        }
        self.remaining_ms -= 1000;
        if self.remaining_ms <= 0 {
            match self.phase {
                Phase::Running => {
                    self.trigger_break();
                    Some(Transition::EnteredBreak)
                }
                Phase::Break => {
                    self.start();
                    Some(Transition::EnteredRunning)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Remaining time as `(minutes, seconds)`.
    pub fn remaining(&self) -> (i64, i64) {
        let total = self.remaining_ms.max(0);
        (total / 60000, (total % 60000) / 1000)
    }

    /// Remaining time formatted as `MM:SS`.
    pub fn formatted(&self) -> String {
        let (m, s) = self.remaining();
        format!("{:02}:{:02}", m, s)
    }

    pub fn update_interval(&mut self, minutes: u32) {
        self.interval_minutes = minutes;
        // If currently running (not in break), reset timer with new interval.
        if self.phase == Phase::Running {
            self.start();
        }
    }

    pub fn update_duration(&mut self, seconds: u32) {
        self.duration_seconds = seconds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_idle() {
        let t = Timer::new(25, 15);
        assert_eq!(t.phase, Phase::Idle);
        assert_eq!(t.remaining_ms, 0);
    }

    #[test]
    fn start_transitions_to_running() {
        let mut t = Timer::new(25, 15);
        t.start();
        assert_eq!(t.phase, Phase::Running);
        assert_eq!(t.remaining_ms, 25 * 60 * 1000);
    }

    #[test]
    fn pause_and_resume_cycle() {
        let mut t = Timer::new(25, 15);
        t.start();
        t.pause();
        assert_eq!(t.phase, Phase::Paused);
        t.resume();
        assert_eq!(t.phase, Phase::Running);
    }

    #[test]
    fn pause_idle_is_noop() {
        let mut t = Timer::new(25, 15);
        t.pause();
        assert_eq!(t.phase, Phase::Idle);
    }

    #[test]
    fn resume_running_is_noop() {
        let mut t = Timer::new(25, 15);
        t.start();
        t.resume();
        assert_eq!(t.phase, Phase::Running);
    }

    #[test]
    fn tick_idle_returns_none_and_does_not_change_state() {
        let mut t = Timer::new(25, 15);
        assert!(t.tick().is_none());
        assert_eq!(t.phase, Phase::Idle);
    }

    #[test]
    fn tick_paused_does_not_decrement() {
        let mut t = Timer::new(25, 15);
        t.start();
        let before = t.remaining_ms;
        t.pause();
        assert!(t.tick().is_none());
        assert_eq!(t.remaining_ms, before);
    }

    #[test]
    fn tick_decrements_remaining_ms_by_1000() {
        let mut t = Timer::new(1, 15);
        t.start();
        let before = t.remaining_ms;
        assert!(t.tick().is_none());
        assert_eq!(t.remaining_ms, before - 1000);
    }

    #[test]
    fn tick_enters_break_when_running_expires() {
        let mut t = Timer::new(1, 15);
        t.start();
        t.remaining_ms = 500;
        assert_eq!(t.tick(), Some(Transition::EnteredBreak));
        assert_eq!(t.phase, Phase::Break);
        assert_eq!(t.remaining_ms, 15_000);
    }

    #[test]
    fn tick_enters_running_when_break_expires() {
        let mut t = Timer::new(30, 1);
        t.trigger_break();
        t.remaining_ms = 500;
        assert_eq!(t.tick(), Some(Transition::EnteredRunning));
        assert_eq!(t.phase, Phase::Running);
        assert_eq!(t.remaining_ms, 30 * 60 * 1000);
    }

    #[test]
    fn tick_exact_boundary_running() {
        let mut t = Timer::new(1, 15);
        t.start();
        t.remaining_ms = 1000;
        assert_eq!(t.tick(), Some(Transition::EnteredBreak));
    }

    #[test]
    fn tick_exact_boundary_break() {
        let mut t = Timer::new(25, 1);
        t.trigger_break();
        t.remaining_ms = 1000;
        assert_eq!(t.tick(), Some(Transition::EnteredRunning));
    }

    #[test]
    fn remaining_clamps_negative_to_zero() {
        let t = Timer {
            remaining_ms: -5000,
            ..Timer::new(25, 15)
        };
        assert_eq!(t.remaining(), (0, 0));
    }

    #[test]
    fn remaining_decomposes_correctly() {
        let t = Timer {
            remaining_ms: 2 * 60_000 + 37_000,
            ..Timer::new(25, 15)
        };
        assert_eq!(t.remaining(), (2, 37));
    }

    #[test]
    fn remaining_zero() {
        let t = Timer {
            remaining_ms: 0,
            ..Timer::new(25, 15)
        };
        assert_eq!(t.remaining(), (0, 0));
    }

    #[test]
    fn formatted_returns_mm_ss() {
        let t = Timer {
            remaining_ms: 5 * 60_000 + 4_000,
            ..Timer::new(25, 15)
        };
        assert_eq!(t.formatted(), "05:04");
    }

    #[test]
    fn formatted_zero() {
        let t = Timer {
            remaining_ms: 0,
            ..Timer::new(25, 15)
        };
        assert_eq!(t.formatted(), "00:00");
    }

    #[test]
    fn stop_resets_to_idle() {
        let mut t = Timer::new(25, 15);
        t.start();
        t.stop();
        assert_eq!(t.phase, Phase::Idle);
        assert_eq!(t.remaining_ms, 0);
    }

    #[test]
    fn stop_idle_is_idempotent() {
        let mut t = Timer::new(25, 15);
        t.stop();
        assert_eq!(t.phase, Phase::Idle);
    }

    #[test]
    fn trigger_break_sets_break_phase_and_remaining() {
        let mut t = Timer::new(25, 30);
        t.trigger_break();
        assert_eq!(t.phase, Phase::Break);
        assert_eq!(t.remaining_ms, 30_000);
    }

    #[test]
    fn update_interval_changes_field() {
        let mut t = Timer::new(25, 15);
        t.update_interval(45);
        assert_eq!(t.interval_minutes, 45);
    }

    #[test]
    fn update_interval_resets_running_timer() {
        let mut t = Timer::new(25, 15);
        t.start();
        t.remaining_ms = 60_000;
        t.update_interval(30);
        assert_eq!(t.remaining_ms, 30 * 60 * 1000);
    }

    #[test]
    fn update_interval_does_not_reset_non_running() {
        let mut t = Timer::new(25, 15);
        t.trigger_break();
        t.remaining_ms = 5_000;
        t.update_interval(30);
        assert_eq!(t.remaining_ms, 5_000);
        assert_eq!(t.phase, Phase::Break);
    }

    #[test]
    fn update_duration_changes_field() {
        let mut t = Timer::new(25, 15);
        t.update_duration(60);
        assert_eq!(t.duration_seconds, 60);
    }

    #[test]
    fn full_idle_running_break_running_cycle() {
        let mut t = Timer::new(1, 1);
        assert_eq!(t.phase, Phase::Idle);
        t.start();
        assert_eq!(t.phase, Phase::Running);
        t.remaining_ms = 1000;
        assert_eq!(t.tick(), Some(Transition::EnteredBreak));
        assert_eq!(t.phase, Phase::Break);
        t.remaining_ms = 1000;
        assert_eq!(t.tick(), Some(Transition::EnteredRunning));
        assert_eq!(t.phase, Phase::Running);
        assert!(t.remaining_ms > 0);
    }

    #[test]
    fn multiple_ticks_near_boundary() {
        let mut t = Timer::new(1, 15);
        t.start();
        t.remaining_ms = 2500;
        assert!(t.tick().is_none()); // 1500 left
        assert!(t.tick().is_none()); // 500 left
        assert_eq!(t.tick(), Some(Transition::EnteredBreak)); // -500 -> triggered
    }

    #[test]
    fn pause_after_trigger_break_stays_in_break() {
        let mut t = Timer::new(25, 15);
        t.trigger_break();
        t.pause();
        assert_eq!(t.phase, Phase::Break); // can't pause break
    }
}
