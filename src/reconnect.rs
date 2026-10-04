use std::time::Duration;

use crate::structs::config::{reconnect_backoff, DEVICE_BLOCKED_DELAY, HEALTHY_SESSION};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionExit {
    Closed,
    DeviceBlocked,
    Errored,
    NoTtwid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptEnd {
    Healthy,
    Failed,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionAction {
    Keep,
    Rotate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Judgement {
    pub end: AttemptEnd,
    pub session: SessionAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Retry { attempt: u32, delay: Duration },
    GiveUp { attempt: u32 },
}

pub fn judge(exit: SessionExit, lived: Duration) -> Judgement {
    let healthy = lived >= HEALTHY_SESSION;
    let end = if healthy { AttemptEnd::Healthy } else { AttemptEnd::Failed };
    match exit {
        SessionExit::Closed => Judgement { end, session: SessionAction::Keep },
        SessionExit::DeviceBlocked => Judgement {
            end: AttemptEnd::Blocked,
            session: SessionAction::Rotate,
        },
        SessionExit::Errored if healthy => Judgement { end, session: SessionAction::Keep },
        SessionExit::Errored => Judgement { end, session: SessionAction::Rotate },
        SessionExit::NoTtwid => Judgement {
            end: AttemptEnd::Failed,
            session: SessionAction::Rotate,
        },
    }
}

#[derive(Clone, Debug)]
pub struct ReconnectBudget {
    attempt: u32,
    max_retries: u32,
}

impl ReconnectBudget {
    pub fn new(max_retries: u32) -> Self {
        Self { attempt: 0, max_retries }
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn record(&mut self, end: AttemptEnd) -> Verdict {
        let next = match end {
            AttemptEnd::Healthy => 1,
            AttemptEnd::Failed | AttemptEnd::Blocked => match self.attempt.checked_add(1) {
                Some(next) => next,
                None => return Verdict::GiveUp { attempt: self.attempt },
            },
        };
        self.attempt = next;
        if next > self.max_retries {
            return Verdict::GiveUp { attempt: next };
        }
        let delay = match end {
            AttemptEnd::Blocked => DEVICE_BLOCKED_DELAY,
            AttemptEnd::Healthy | AttemptEnd::Failed => reconnect_backoff(next),
        };
        Verdict::Retry { attempt: next, delay }
    }
}
