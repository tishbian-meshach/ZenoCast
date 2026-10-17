use uuid::Uuid;
use zeno_core::{InterruptBehavior, Lifecycle, LifecyclePhase, TimeRange};
use zeno_engine::{LifecycleController, LifecycleManager};

#[test]
fn test_overlay_in_transitions_to_loop_and_out_cuts_to_hidden() {
    let lifecycle = Lifecycle {
        in_range: TimeRange::new(0.0, 1.0),
        loop_range: TimeRange::new(1.0, 3.0),
        out_range: TimeRange::new(3.0, 4.0),
        loop_count: None,
        hide_during_in: InterruptBehavior::Immediately,
        show_during_out: InterruptBehavior::Immediately,
    };

    let mut controller = LifecycleController::new(Uuid::new_v4(), "Lower Third", lifecycle);
    assert_eq!(controller.phase, LifecyclePhase::Hidden);
    assert!(!controller.visible);
    assert_eq!(controller.effective_alpha(), 0.0);

    // 1. Fire Play In
    controller.play_in();
    assert_eq!(controller.phase, LifecyclePhase::In);
    assert!(controller.visible);
    assert_eq!(controller.effective_alpha(), 0.0);

    // Step through In (0.5s -> alpha = 0.5)
    controller.step(0.5);
    assert_eq!(controller.phase, LifecyclePhase::In);
    assert!((controller.effective_alpha() - 0.5).abs() < 1e-4);

    // Complete In (remaining 0.5s -> total 1.0s)
    controller.step(0.5);
    // Transitions to Loop on completion
    assert_eq!(controller.phase, LifecyclePhase::Loop);
    assert_eq!(controller.current_time, 1.0);
    assert_eq!(controller.effective_alpha(), 1.0);

    // Step through Loop for 2 seconds (1.0 to 3.0)
    controller.step(2.0);
    assert_eq!(controller.phase, LifecyclePhase::Loop);
    assert_eq!(controller.loops_completed, 1);
    assert_eq!(controller.effective_alpha(), 1.0);

    // 2. Fire Play Out
    controller.play_out();
    assert_eq!(controller.phase, LifecyclePhase::Out);
    assert_eq!(controller.current_time, 3.0);
    assert_eq!(controller.effective_alpha(), 1.0);

    // Step half way through Out (0.5s of 1.0s)
    controller.step(0.5);
    assert_eq!(controller.phase, LifecyclePhase::Out);
    assert!((controller.effective_alpha() - 0.5).abs() < 1e-4);

    // Complete Out
    controller.step(0.5);
    // Cuts cleanly to Hidden
    assert_eq!(controller.phase, LifecyclePhase::Hidden);
    assert!(!controller.visible);
    assert_eq!(controller.effective_alpha(), 0.0);
}

#[test]
fn test_triggering_out_mid_in_transition_has_continuous_transforms_no_pop() {
    let lifecycle = Lifecycle {
        in_range: TimeRange::new(0.0, 1.0),
        loop_range: TimeRange::new(1.0, 3.0),
        out_range: TimeRange::new(3.0, 4.0),
        loop_count: None,
        hide_during_in: InterruptBehavior::Immediately,
        show_during_out: InterruptBehavior::Immediately,
    };

    let mut controller = LifecycleController::new(Uuid::new_v4(), "Banner", lifecycle);
    controller.play_in();

    // Advance to t = 0.3 of a 1.0s In transition
    controller.step(0.3);
    assert_eq!(controller.phase, LifecyclePhase::In);
    let alpha_before_interrupt = controller.effective_alpha();
    assert!((alpha_before_interrupt - 0.3).abs() < 1e-4);

    // Trigger Out immediately while In is running
    controller.play_out();
    assert_eq!(controller.phase, LifecyclePhase::Out);

    // Immediate next frame evaluation (t=0 of Out): MUST MATCH exactly (no 1-frame pop)
    let alpha_after_interrupt = controller.effective_alpha();
    assert_eq!(
        alpha_before_interrupt, alpha_after_interrupt,
        "Discontinuity detected! 1-frame pop: before={:?}, after={:?}",
        alpha_before_interrupt, alpha_after_interrupt
    );

    // Step 0.5s into Out (50% progress of Out): alpha should smoothly decay from 0.3 to 0.15
    controller.step(0.5);
    let mid_out_alpha = controller.effective_alpha();
    assert!(
        (mid_out_alpha - 0.15).abs() < 1e-4,
        "Expected smooth decay without pop: got {:?}",
        mid_out_alpha
    );

    // Step remaining 0.5s: cleanly hidden
    controller.step(0.5);
    assert_eq!(controller.phase, LifecyclePhase::Hidden);
    assert_eq!(controller.effective_alpha(), 0.0);
}

#[test]
fn test_multiple_simultaneous_overlays_independent_lifecycles() {
    let mut manager = LifecycleManager::new();

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    let lc1 = Lifecycle::default();
    let lc2 = Lifecycle::default();
    let lc3 = Lifecycle::default();

    manager.register(LifecycleController::new(id1, "Scoreboard", lc1));
    manager.register(LifecycleController::new(id2, "Lower Third", lc2));
    manager.register(LifecycleController::new(id3, "Sponsor Bug", lc3));

    // Fire In on #1 (Scoreboard) and #3 (Sponsor Bug), leave #2 (Lower Third) Hidden
    manager.find_mut(id1).unwrap().play_in();
    manager.find_mut(id3).unwrap().play_in();

    // Step 0.5s
    manager.step_all(0.5);

    assert_eq!(manager.find(id1).unwrap().phase, LifecyclePhase::In);
    assert_eq!(manager.find(id2).unwrap().phase, LifecyclePhase::Hidden);
    assert_eq!(manager.find(id3).unwrap().phase, LifecyclePhase::In);

    // Complete In (step 0.5s)
    manager.step_all(0.5);
    assert_eq!(manager.find(id1).unwrap().phase, LifecyclePhase::Loop);
    assert_eq!(manager.find(id2).unwrap().phase, LifecyclePhase::Hidden);
    assert_eq!(manager.find(id3).unwrap().phase, LifecyclePhase::Loop);

    // Now fire In on #2, and fire Out on #1
    manager.find_mut(id2).unwrap().play_in();
    manager.find_mut(id1).unwrap().play_out();

    // Step 0.5s
    manager.step_all(0.5);
    assert_eq!(manager.find(id1).unwrap().phase, LifecyclePhase::Out);
    assert_eq!(manager.find(id2).unwrap().phase, LifecyclePhase::In);
    assert_eq!(manager.find(id3).unwrap().phase, LifecyclePhase::Loop);

    // Verify all 3 overlays have distinct, independent states and alphas
    assert!((manager.find(id1).unwrap().effective_alpha() - 0.5).abs() < 1e-4);
    assert!((manager.find(id2).unwrap().effective_alpha() - 0.5).abs() < 1e-4);
    assert_eq!(manager.find(id3).unwrap().effective_alpha(), 1.0);
}

#[test]
fn test_ping_pong_loop_playback() {
    let lifecycle = Lifecycle {
        in_range: TimeRange::new(0.0, 1.0),
        loop_range: TimeRange::new(1.0, 2.0),
        out_range: TimeRange::new(2.0, 3.0),
        loop_count: None,
        hide_during_in: InterruptBehavior::Immediately,
        show_during_out: InterruptBehavior::Immediately,
    };

    let mut controller = LifecycleController::new(Uuid::new_v4(), "Ticker", lifecycle);
    controller.ping_pong = true;
    controller.play_in();

    // Finish In
    controller.step(1.0);
    assert_eq!(controller.phase, LifecyclePhase::Loop);
    assert_eq!(controller.current_time, 1.0);

    // Forward loop to 2.0
    controller.step(1.0);
    assert_eq!(controller.current_time, 2.0);
    assert!(controller.ping_pong_reverse);

    // Reverse loop back towards 1.0
    controller.step(0.5);
    assert_eq!(controller.current_time, 1.5);
    assert!(controller.ping_pong_reverse);

    controller.step(0.5);
    assert_eq!(controller.current_time, 1.0);
    assert!(!controller.ping_pong_reverse);
    assert_eq!(controller.loops_completed, 1);
}
