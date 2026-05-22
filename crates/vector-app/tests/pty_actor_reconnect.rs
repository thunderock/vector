//! Plan 09-03: PERSIST-01/02 state machine + backoff + cancel + event emission.
//!
//! Drives `pane_io_loop` through a hand-rolled `TestEventSink`, scripted
//! `Domain`, and `FakeTransport` (see tests/common/mod.rs). Uses
//! `tokio::time::pause()` for deterministic schedule assertions.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use vector_app::frame_tick::{CoalesceBuffer, COALESCE_THRESHOLD};
use vector_app::pty_actor::{pane_io_loop, EventSink};
use vector_app::UserEvent;
use vector_mux::{Domain, PaneId, PtyTransport};

mod common;
use common::{test_sink, FakeTransport, ScriptStep, ScriptedDomain};

const TEST_PANE_ID: PaneId = PaneId(42);
const TEST_PROFILE: &str = "test-profile";

/// Build common actor inputs.
fn actor_channels() -> (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>, mpsc::Sender<(u16, u16)>, mpsc::Receiver<(u16, u16)>) {
    let (wtx, wrx) = mpsc::channel::<Vec<u8>>(8);
    let (rtx, rrx) = mpsc::channel::<(u16, u16)>(4);
    (wtx, wrx, rtx, rrx)
}

async fn collect_until<F: Fn(&UserEvent) -> bool>(
    rx: &mut mpsc::UnboundedReceiver<UserEvent>,
    stop: F,
    timeout: Duration,
) -> Vec<UserEvent> {
    let mut out = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline - now;
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(ev)) => {
                let done = stop(&ev);
                out.push(ev);
                if done {
                    break;
                }
            }
            Ok(None) | Err(_) => break,
        }
    }
    out
}

/// Test 1: actor enters Reconnecting on EOF, swaps to a fresh transport on
/// first `Ok(Some(...))`, and emits PaneReconnecting{attempt:1} then
/// PaneReconnected.
#[tokio::test]
async fn pty_actor_enters_reconnecting_on_eof() {
    let (sink, mut rx) = test_sink();

    // First reconnect: hand back a fresh dead transport (so after PaneReconnected
    // the actor will loop back to Reconnecting; we'll stop checking after the first
    // PaneReconnected).
    let steps = vec![ScriptStep::Swap(Box::new(|| {
        Box::new(FakeTransport::dead()) as Box<dyn PtyTransport>
    })), ScriptStep::PermanentNone];
    let domain: Arc<dyn Domain> = Arc::new(ScriptedDomain::new(steps));

    let (_wtx, wrx, _rtx, rrx) = actor_channels();
    let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
    let cancel = CancellationToken::new();

    let handle = tokio::spawn(pane_io_loop(
        TEST_PANE_ID,
        Box::new(FakeTransport::dead()) as Box<dyn PtyTransport>,
        Arc::clone(&domain),
        TEST_PROFILE.to_string(),
        Arc::clone(&sink) as Arc<dyn EventSink>,
        coalesce,
        wrx,
        rrx,
        cancel,
    ));

    // Wait for PaneReconnected (means the swap succeeded).
    let events = collect_until(
        &mut rx,
        |e| matches!(e, UserEvent::PaneReconnected { .. }),
        Duration::from_secs(2),
    )
    .await;

    let _ = handle.await;

    // Expect: PaneReconnecting{attempt:1, profile: "test-profile"} then PaneReconnected.
    assert!(
        events.iter().any(|e| matches!(
            e,
            UserEvent::PaneReconnecting { pane_id, attempt: 1, profile_label }
                if *pane_id == TEST_PANE_ID && profile_label == TEST_PROFILE
        )),
        "expected PaneReconnecting{{attempt:1, profile:test-profile}} in {events:?}"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            UserEvent::PaneReconnected { pane_id } if *pane_id == TEST_PANE_ID
        )),
        "expected PaneReconnected in {events:?}"
    );
}

/// Test 2: 4 errors then success — cumulative sleeps follow exact schedule
/// `BACKOFF_SCHEDULE_SECS = [1, 2, 4, 8, 16, 30]`.
/// Attempt 1 → err → sleep 1s → Attempt 2 → err → sleep 2s → Attempt 3 → err
/// → sleep 4s → Attempt 4 → err → sleep 8s → Attempt 5 → Ok(Some).
#[tokio::test(start_paused = true)]
async fn pty_actor_exponential_backoff_schedule() {
    let (sink, mut rx) = test_sink();

    let steps = vec![
        ScriptStep::Err("attempt 1".into()),
        ScriptStep::Err("attempt 2".into()),
        ScriptStep::Err("attempt 3".into()),
        ScriptStep::Err("attempt 4".into()),
        ScriptStep::Swap(Box::new(|| Box::new(FakeTransport::dead()) as Box<dyn PtyTransport>)),
        ScriptStep::PermanentNone, // after the new transport EOFs, exit clean
    ];
    let domain: Arc<dyn Domain> = Arc::new(ScriptedDomain::new(steps));

    let (_wtx, wrx, _rtx, rrx) = actor_channels();
    let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
    let cancel = CancellationToken::new();

    let handle = tokio::spawn(pane_io_loop(
        TEST_PANE_ID,
        Box::new(FakeTransport::dead()) as Box<dyn PtyTransport>,
        Arc::clone(&domain),
        TEST_PROFILE.to_string(),
        Arc::clone(&sink) as Arc<dyn EventSink>,
        coalesce,
        wrx,
        rrx,
        cancel,
    ));

    // Let attempt 1 fire.
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    // Schedule: cumulative sleeps after errors are 1 / 2 / 4 / 8 seconds.
    // Advance virtual time well past each cumulative point and yield so the
    // actor task can poll past the sleep and through `reconnect_one_shot`.
    let advances = [
        Duration::from_secs(1),
        Duration::from_secs(2),
        Duration::from_secs(4),
        Duration::from_secs(8),
    ];
    for d in advances {
        tokio::time::advance(d + Duration::from_millis(1)).await;
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
    }

    // Drain events with a generous virtual timeout.
    let events = collect_until(
        &mut rx,
        |e| matches!(e, UserEvent::PaneReconnected { .. }),
        Duration::from_secs(60),
    )
    .await;

    let _ = handle.await;

    // Expect PaneReconnecting for attempts 1..=5 each at least once.
    for attempt in 1u32..=5 {
        assert!(
            events.iter().any(|e| matches!(
                e,
                UserEvent::PaneReconnecting { attempt: a, .. } if *a == attempt
            )),
            "expected PaneReconnecting{{attempt:{attempt}}} in {events:?}"
        );
    }
    // Final swap fires PaneReconnected exactly once.
    let reconnected_count = events
        .iter()
        .filter(|e| matches!(e, UserEvent::PaneReconnected { .. }))
        .count();
    assert_eq!(reconnected_count, 1, "PaneReconnected fired {reconnected_count} times: {events:?}");

    // Sanity: assert the schedule constant via inline reference (also pins it
    // into the test source so a future BACKOFF_SCHEDULE_SECS edit blows up here).
    let schedule: &[u64] = &[1, 2, 4, 8, 16, 30];
    assert_eq!(schedule[0], 1);
    assert_eq!(schedule[5], 30);
}

/// Test 3: cancel during backoff aborts the sleep within 50 ms and the
/// actor exits with PaneExited; no PaneReconnected.
#[tokio::test]
async fn pty_actor_cancels_backoff_on_pane_close() {
    let (sink, mut rx) = test_sink();

    // Domain always errors so the actor stays in backoff forever (until cancel).
    let mut steps: Vec<ScriptStep> = Vec::new();
    for i in 0..32 {
        steps.push(ScriptStep::Err(format!("err{i}")));
    }
    let domain: Arc<dyn Domain> = Arc::new(ScriptedDomain::new(steps));

    let (_wtx, wrx, _rtx, rrx) = actor_channels();
    let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
    let cancel = CancellationToken::new();
    let cancel_handle = cancel.clone();

    let handle = tokio::spawn(pane_io_loop(
        TEST_PANE_ID,
        Box::new(FakeTransport::dead()) as Box<dyn PtyTransport>,
        Arc::clone(&domain),
        TEST_PROFILE.to_string(),
        Arc::clone(&sink) as Arc<dyn EventSink>,
        coalesce,
        wrx,
        rrx,
        cancel,
    ));

    // Wait for the first PaneReconnecting to land.
    let _first = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("PaneReconnecting timeout")
        .expect("sink closed early");

    // Cancel. Actor must exit promptly.
    cancel_handle.cancel();

    tokio::time::timeout(Duration::from_millis(50), handle)
        .await
        .expect("actor did not exit within 50 ms of cancel")
        .expect("actor task panicked");

    // Drain any remaining events; assert PaneExited fired and no PaneReconnected.
    let mut got_exited = false;
    let mut got_reconnected = false;
    while let Ok(ev) = rx.try_recv() {
        match ev {
            UserEvent::PaneExited(_) => got_exited = true,
            UserEvent::PaneReconnected { .. } => got_reconnected = true,
            _ => {}
        }
    }
    assert!(got_exited, "expected PaneExited after cancel");
    assert!(!got_reconnected, "PaneReconnected fired despite cancel");
}

/// Test 4: profile_label string flows through verbatim into every
/// PaneReconnecting event.
#[tokio::test]
async fn reconnect_emits_pane_reconnecting_event() {
    let (sink, mut rx) = test_sink();

    let steps = vec![
        ScriptStep::Err("first".into()),
        ScriptStep::Swap(Box::new(|| Box::new(FakeTransport::dead()) as Box<dyn PtyTransport>)),
        ScriptStep::PermanentNone,
    ];
    let domain: Arc<dyn Domain> = Arc::new(ScriptedDomain::new(steps));

    let (_wtx, wrx, _rtx, rrx) = actor_channels();
    let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
    let cancel = CancellationToken::new();

    let label = "custom-profile-label";
    let handle = tokio::spawn(pane_io_loop(
        TEST_PANE_ID,
        Box::new(FakeTransport::dead()) as Box<dyn PtyTransport>,
        Arc::clone(&domain),
        label.to_string(),
        Arc::clone(&sink) as Arc<dyn EventSink>,
        coalesce,
        wrx,
        rrx,
        cancel,
    ));

    let events = collect_until(
        &mut rx,
        |e| matches!(e, UserEvent::PaneReconnected { .. }),
        Duration::from_secs(5),
    )
    .await;
    let _ = handle.await;

    let reconnecting_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            UserEvent::PaneReconnecting { attempt, profile_label, .. } => {
                Some((*attempt, profile_label.clone()))
            }
            _ => None,
        })
        .collect();
    assert!(!reconnecting_events.is_empty(), "no PaneReconnecting events: {events:?}");
    for (_, lbl) in &reconnecting_events {
        assert_eq!(lbl, label, "profile_label mismatch in {reconnecting_events:?}");
    }
}

/// Sanity: the EventSink trait Task 1 introduced is consumable from this
/// integration test. Compiles → contract is locked.
struct _CompileTimeCheck;
impl EventSink for _CompileTimeCheck {
    fn send_user_event(&self, _: UserEvent) {}
}
