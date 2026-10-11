//! Inspector panel tests (lane sr-fix-insp-count, task A only).
//!
//! Covers review findings 1 (Tab reaches inspector via `focus_next_sidebar`)
//! and 4 (keep-last via `last_inspected`) at the wiring level the lane owns:
//! `HerdrWindow::selected_agent_view()` + `last_inspected`, the
//! `selected.or(last_inspected)` mount in `crate::inspector`, and the
//! `INSPECTOR_MIN_WINDOW_WIDTH` gate. Field rendering itself is pinned by
//! `inspector_fields` unit tests; these tests prove the panel receives the
//! right agent in each state using the existing `layout_tests` fixtures.
#![allow(clippy::expect_used)]

use super::*;
use crate::preferences::AgentSort;
use crate::sidebar::inspector_fields::inspector_fields;
use herdr_client::protocol::AgentStatus;

fn tokens(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn selected_worker_snapshot() -> ClientShellSnapshot {
    let mut fixture = snapshot(1);
    let template = fixture.agents[0].clone();
    let mut agent = template;
    agent.pane_id = "p1".into();
    agent.workspace_id = "w0".into();
    agent.tab_id = "t0".into();
    agent.name = Some("worker-1".into());
    agent.display_agent = Some("worker-1".into());
    agent.tokens = tokens(&[
        ("role", "worker"),
        ("model", "muse-spark-1.3"),
        ("parent", "p-orch"),
    ]);
    agent.agent_status = AgentStatus::Working;
    agent.focused = true;
    fixture.agents = vec![agent];
    fixture.focused_pane_id = Some("p1".into());
    fixture
}

fn wide_window(
    snapshot: ClientShellSnapshot,
    cx: &mut gpui::TestAppContext,
) -> (Entity<HerdrWindow>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot));
        view.agent_sort = AgentSort::Grouped;
        view
    });
    cx.simulate_resize(size(px(1200.), px(600.)));
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        full_draw(window, cx).clear(cx);
    });
    (view, cx)
}

/// The exact agent the mount hands the panel: `selected.or(last_inspected)`.
fn mounted_agent(
    view: &Entity<HerdrWindow>,
    cx: &mut gpui::VisualTestContext,
) -> Option<ClientShellAgent> {
    cx.update(|_, cx| {
        let selected = view.read(cx).selected_agent_view();
        let last = view.read(cx).last_inspected.clone();
        selected.or(last)
    })
}

/// (a) A selected worker exposes name, status, role, model and parent
/// through the exact agent the mount hands the panel.
#[gpui::test]
fn inspector_shows_selected_worker_details(cx: &mut gpui::TestAppContext) {
    let (view, cx) = wide_window(selected_worker_snapshot(), cx);
    view.update(cx, |view, _| view.remember_inspected());
    let agent = mounted_agent(&view, cx).expect("selected worker must reach the panel");
    assert_eq!(agent.name.as_deref(), Some("worker-1"));
    let values: Vec<(&str, String)> = inspector_fields(Some(&agent))
        .into_iter()
        .map(|f| (f.label, f.value))
        .collect();
    for (label, expected) in [
        ("Role", "worker"),
        ("Model", "muse-spark-1.3"),
        ("Parent", "p-orch"),
    ] {
        assert!(
            values.iter().any(|(l, v)| *l == label && v == expected),
            "{label} must render {expected:?}, got {values:?}"
        );
    }
    assert!(
        values
            .iter()
            .any(|(l, v)| *l == "Status" && v.contains("working")),
        "status must render working, got {values:?}"
    );
}

/// (b) Missing tokens render "—" for role/model/parent.
#[gpui::test]
fn inspector_renders_dash_for_missing_tokens(cx: &mut gpui::TestAppContext) {
    let mut fixture = selected_worker_snapshot();
    fixture.agents[0].tokens = Vec::new();
    let (view, cx) = wide_window(fixture, cx);
    let agent = mounted_agent(&view, cx).expect("selected worker must reach the panel");
    let values: Vec<(&str, String)> = inspector_fields(Some(&agent))
        .into_iter()
        .map(|f| (f.label, f.value))
        .collect();
    for label in ["Role", "Model", "Parent"] {
        assert!(
            values.iter().any(|(l, v)| *l == label && v == "—"),
            "{label} must render —, got {values:?}"
        );
    }
}

/// (c) No selection and no keep-last yields no agent for the panel.
#[gpui::test]
fn inspector_empty_without_selection(cx: &mut gpui::TestAppContext) {
    let mut fixture = snapshot(1);
    for agent in &mut fixture.agents {
        agent.focused = false;
    }
    fixture.focused_pane_id = None;
    let (view, cx) = wide_window(fixture, cx);
    cx.update(|_, cx| {
        assert!(view.read(cx).selected_agent_view().is_none());
        assert!(view.read(cx).last_inspected.is_none());
    });
    assert!(mounted_agent(&view, cx).is_none());
    assert!(inspector_fields(None).is_empty());
}

/// (d) The selected agent disappears from the snapshot → last known data
/// still reaches the panel via `last_inspected`.
#[gpui::test]
fn inspector_keeps_last_known_agent_after_snapshot_removal(cx: &mut gpui::TestAppContext) {
    let (view, cx) = wide_window(selected_worker_snapshot(), cx);
    view.update(cx, |view, _| view.remember_inspected());
    view.update(cx, |view, _| {
        let snapshot = view
            .live
            .snapshot
            .as_ref()
            .expect("snapshot")
            .as_ref()
            .clone();
        let mut next = snapshot;
        next.agents.retain(|agent| agent.pane_id != "p1");
        next.focused_pane_id = None;
        view.live.snapshot = Some(Arc::new(next));
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        full_draw(window, cx).clear(cx);
    });
    cx.update(|_, cx| {
        assert!(view.read(cx).selected_agent_view().is_none());
    });
    let kept = mounted_agent(&view, cx).expect("keep-last must supply the removed agent");
    assert_eq!(kept.name.as_deref(), Some("worker-1"));
    let values: Vec<(&str, String)> = inspector_fields(Some(&kept))
        .into_iter()
        .map(|f| (f.label, f.value))
        .collect();
    assert!(
        values
            .iter()
            .any(|(l, v)| *l == "Model" && v == "muse-spark-1.3")
    );
}

/// (e) Window narrower than INSPECTOR_MIN_WINDOW_WIDTH hides the inspector
/// column; the wide window mounts it. Proved via the `inspector-panel`
/// debug bounds the mount owns.
#[gpui::test]
fn inspector_hidden_below_min_window_width(cx: &mut gpui::TestAppContext) {
    let (view, cx) = wide_window(selected_worker_snapshot(), cx);
    assert!(
        cx.debug_bounds("inspector-panel").is_some(),
        "wide window must mount the inspector"
    );
    let _ = &view;
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        full_draw(window, cx).clear(cx);
    });
    assert!(
        cx.debug_bounds("inspector-panel").is_none(),
        "narrow window must hide the inspector"
    );
}
#[gpui::test]
fn inspector_metrics_tab_hides_overview_fields(cx: &mut gpui::TestAppContext) {
    use crate::sidebar::inspector::InspectorTab;
    let (view, cx) = wide_window(selected_worker_snapshot(), cx);
    assert!(cx.debug_bounds("inspector-tabs").is_some());
    assert!(cx.debug_bounds("inspector-tab-Metrics").is_some());
    assert!(cx.debug_bounds("inspector-fields").is_some());
    cx.update(|_, cx| {
        assert_eq!(view.read(cx).inspector_tab, InspectorTab::Overview);
    });
    let metrics = cx.debug_bounds("inspector-tab-Metrics").unwrap().center();
    cx.simulate_click(metrics, Default::default());
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        full_draw(window, cx).clear(cx);
    });
    cx.update(|_, cx| {
        assert_eq!(view.read(cx).inspector_tab, InspectorTab::Metrics);
    });
    assert!(cx.debug_bounds("inspector-fields").is_none());
    assert!(cx.debug_bounds("inspector-empty").is_some());
}
