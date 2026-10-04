use std::time::Duration;

use piratetok_live_rs::reconnect::{judge, AttemptEnd, Judgement, ReconnectBudget, SessionAction, SessionExit, Verdict};

const SHORT: Duration = Duration::from_secs(3);
const LONG: Duration = Duration::from_secs(600);

fn j(end: AttemptEnd, session: SessionAction) -> Judgement {
    Judgement { end, session }
}

#[test]
fn judge_keeps_or_rotates_the_ttwid_session() {
    assert_eq!(judge(SessionExit::Closed, SHORT), j(AttemptEnd::Failed, SessionAction::Keep));
    assert_eq!(judge(SessionExit::Closed, LONG), j(AttemptEnd::Healthy, SessionAction::Keep));
    assert_eq!(judge(SessionExit::DeviceBlocked, SHORT), j(AttemptEnd::Blocked, SessionAction::Rotate));
    assert_eq!(judge(SessionExit::DeviceBlocked, LONG), j(AttemptEnd::Blocked, SessionAction::Rotate));
    assert_eq!(judge(SessionExit::Errored, SHORT), j(AttemptEnd::Failed, SessionAction::Rotate));
    assert_eq!(judge(SessionExit::Errored, LONG), j(AttemptEnd::Healthy, SessionAction::Keep));
    assert_eq!(judge(SessionExit::NoTtwid, Duration::ZERO), j(AttemptEnd::Failed, SessionAction::Rotate));
}

#[test]
fn consecutive_failures_back_off_then_give_up() {
    let mut budget = ReconnectBudget::new(5);
    let delays: Vec<Verdict> = (0..6).map(|_| budget.record(AttemptEnd::Failed)).collect();
    let secs = |s| Duration::from_secs(s);
    assert_eq!(
        delays,
        vec![
            Verdict::Retry { attempt: 1, delay: secs(2) },
            Verdict::Retry { attempt: 2, delay: secs(4) },
            Verdict::Retry { attempt: 3, delay: secs(8) },
            Verdict::Retry { attempt: 4, delay: secs(16) },
            Verdict::Retry { attempt: 5, delay: secs(30) },
            Verdict::GiveUp { attempt: 6 },
        ]
    );
}

#[test]
fn healthy_session_resets_the_budget() {
    let mut budget = ReconnectBudget::new(3);
    for _ in 0..3 {
        assert!(matches!(budget.record(AttemptEnd::Failed), Verdict::Retry { .. }));
    }
    assert_eq!(budget.attempt(), 3);
    assert_eq!(
        budget.record(AttemptEnd::Healthy),
        Verdict::Retry {
            attempt: 1,
            delay: Duration::from_secs(2)
        }
    );
    assert_eq!(budget.attempt(), 1);
    assert!(matches!(budget.record(AttemptEnd::Failed), Verdict::Retry { attempt: 2, .. }));
    assert!(matches!(budget.record(AttemptEnd::Failed), Verdict::Retry { attempt: 3, .. }));
    assert_eq!(budget.record(AttemptEnd::Failed), Verdict::GiveUp { attempt: 4 });
}

#[test]
fn device_blocked_uses_short_delay_but_still_counts() {
    let mut budget = ReconnectBudget::new(2);
    assert_eq!(
        budget.record(AttemptEnd::Blocked),
        Verdict::Retry {
            attempt: 1,
            delay: Duration::from_secs(2)
        }
    );
    assert_eq!(
        budget.record(AttemptEnd::Blocked),
        Verdict::Retry {
            attempt: 2,
            delay: Duration::from_secs(2)
        }
    );
    assert_eq!(budget.record(AttemptEnd::Blocked), Verdict::GiveUp { attempt: 3 });
}

#[test]
fn zero_retries_means_one_attempt_only() {
    let mut budget = ReconnectBudget::new(0);
    assert_eq!(budget.record(AttemptEnd::Healthy), Verdict::GiveUp { attempt: 1 });
}

/// Replays the client loop's decisions over a scripted run and counts ttwid
/// fetches: reuse means a fetch only on start and after each Rotate.
#[test]
fn scripted_run_reuses_ttwid_across_reconnects() {
    let script = [
        (SessionExit::NoTtwid, Duration::ZERO),
        (SessionExit::Closed, LONG),
        (SessionExit::Closed, LONG),
        (SessionExit::Errored, LONG),
        (SessionExit::DeviceBlocked, SHORT),
        (SessionExit::Closed, LONG),
    ];
    let mut budget = ReconnectBudget::new(5);
    let mut have_session = false;
    let mut ttwid_fetches = 0;
    for (exit, lived) in script {
        if !have_session {
            ttwid_fetches += 1;
        }
        let judgement = judge(exit, lived);
        have_session = judgement.session == SessionAction::Keep && exit != SessionExit::NoTtwid;
        assert!(matches!(budget.record(judgement.end), Verdict::Retry { .. }), "{exit:?}");
    }
    assert_eq!(ttwid_fetches, 3, "start (failed), retry after NoTtwid, after DEVICE_BLOCKED");
    assert_eq!(budget.attempt(), 1, "last healthy session reset the budget");
}
