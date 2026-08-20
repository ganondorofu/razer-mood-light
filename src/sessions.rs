use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Generating,
    Waiting,
    Compacting,
}

pub struct Colors {
    pub generating: u32,
    pub idle: u32,
    pub waiting: u32,
    pub compacting: u32,
}

/// Tracks the busy/idle state of each concurrent Claude Code session by ID,
/// so one session finishing doesn't paint over another that's still working.
/// The displayed color is picked by priority across all sessions: a pending
/// question anywhere outranks compaction, which outranks plain generation.
pub struct Sessions {
    states: Mutex<HashMap<String, State>>,
}

impl Sessions {
    pub fn new() -> Self {
        Sessions {
            states: Mutex::new(HashMap::new()),
        }
    }

    pub fn set(&self, session: String, state: State) {
        self.states.lock().unwrap().insert(session, state);
    }

    pub fn clear(&self, session: &str) {
        self.states.lock().unwrap().remove(session);
    }

    pub fn overall_color(&self, colors: &Colors) -> u32 {
        let states = self.states.lock().unwrap();
        if states.values().any(|s| *s == State::Waiting) {
            colors.waiting
        } else if states.values().any(|s| *s == State::Compacting) {
            colors.compacting
        } else if states.values().any(|s| *s == State::Generating) {
            colors.generating
        } else {
            colors.idle
        }
    }
}
