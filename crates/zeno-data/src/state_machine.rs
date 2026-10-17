use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::error::DataError;
use crate::value::DataValue;

/// Fired action representation during state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateAction {
    pub from_state: String,
    pub to_state: String,
    pub action_name: String,
}

/// Transition rule from a source state to a destination state with associated action name.
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub action_name: String,
}

/// Data change trigger rule firing a state transition on data path modifications.
#[derive(Debug, Clone)]
pub struct DataTriggerRule {
    pub data_path: String,
    pub target_state: String,
    pub throttle: Duration,
    pub last_triggered: Option<Instant>,
}

/// State machine governing animation state sequences and reactive transitions.
pub struct StateMachine {
    current_state: String,
    transitions: Vec<Transition>,
    data_triggers: Vec<DataTriggerRule>,
    action_history: Vec<StateAction>,
    transition_counter: AtomicUsize,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new("Idle")
    }
}

impl StateMachine {
    /// Constructs a new state machine starting in `initial_state`.
    pub fn new(initial_state: impl Into<String>) -> Self {
        Self {
            current_state: initial_state.into(),
            transitions: Vec::new(),
            data_triggers: Vec::new(),
            action_history: Vec::new(),
            transition_counter: AtomicUsize::new(0),
        }
    }

    /// Returns the currently active state.
    pub fn current_state(&self) -> &str {
        &self.current_state
    }

    /// Adds a valid transition between two states with an associated action name.
    pub fn add_transition(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        action_name: impl Into<String>,
    ) {
        self.transitions.push(Transition {
            from: from.into(),
            to: to.into(),
            action_name: action_name.into(),
        });
    }

    /// Adds a data-change trigger rule that transitions to `target_state` when `data_path` changes.
    pub fn add_data_trigger(
        &mut self,
        data_path: impl Into<String>,
        target_state: impl Into<String>,
        throttle: Duration,
    ) {
        self.data_triggers.push(DataTriggerRule {
            data_path: data_path.into(),
            target_state: target_state.into(),
            throttle,
            last_triggered: None,
        });
    }

    /// Manually triggers a transition to `target_state`.
    /// Fires the transition action and returns the executed `StateAction`.
    pub fn transition_to(&mut self, target_state: &str) -> Result<StateAction, DataError> {
        let transition_idx = self
            .transitions
            .iter()
            .position(|t| t.from == self.current_state && t.to == target_state)
            .ok_or_else(|| {
                DataError::StateMachine(format!(
                    "Invalid transition from '{}' to '{}'",
                    self.current_state, target_state
                ))
            })?;

        let trans = self.transitions[transition_idx].clone();
        let action = StateAction {
            from_state: self.current_state.clone(),
            to_state: trans.to.clone(),
            action_name: trans.action_name.clone(),
        };

        self.current_state = trans.to;
        self.action_history.push(action.clone());
        self.transition_counter.fetch_add(1, Ordering::Relaxed);

        Ok(action)
    }

    /// Evaluates incoming data modification against data triggers and executes transition if matched.
    pub fn on_data_change(&mut self, path: &str, _value: &DataValue) -> Option<StateAction> {
        let now = Instant::now();
        let mut target_state = None;

        for trigger in &mut self.data_triggers {
            if trigger.data_path == path {
                if let Some(last) = trigger.last_triggered {
                    if now.duration_since(last) < trigger.throttle {
                        continue;
                    }
                }

                trigger.last_triggered = Some(now);
                target_state = Some(trigger.target_state.clone());
                break;
            }
        }

        if let Some(target) = target_state {
            self.transition_to(&target).ok()
        } else {
            None
        }
    }

    /// Retrieves full history of all fired transition actions.
    pub fn action_history(&self) -> &[StateAction] {
        &self.action_history
    }

    /// Number of transitions completed.
    pub fn transition_count(&self) -> usize {
        self.transition_counter.load(Ordering::Relaxed)
    }

    /// Clears action history.
    pub fn clear_history(&mut self) {
        self.action_history.clear();
    }
}
