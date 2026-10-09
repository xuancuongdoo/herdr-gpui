//! Orchestrator-first rule (failing tests only, no production fix).
//!
//! User rule: an orchestrator is always the first row of its group in the
//! sidebar agent tree, above workers and job groups, in every sort mode.
//! Reuses `snapshot`/`fixture_window` (see `tree.rs`) and the legacy
//! `build_agent_tree` tuple shape (see `visual_tree_fixture.rs`).
use super::*;
use crate::preferences::AgentSort;
use crate::sidebar::tree::{TreeNode, build_agent_tree};
use herdr_client::protocol::AgentStatus;
use std::collections::HashSet;

/// Snapshot order `p0, p1, p2`, all in `w0`: worker, orchestrator, worker.
/// The ticket pair (`rev1354-*`, parent `p1`) forms one job group. `p0` is
/// `Blocked` so the `Priority` sort deterministically floats it above the
/// `Working` orchestrator instead of relying on stable input order.
fn orch_first_snapshot() -> ClientShellSnapshot {
    let mut fixture = snapshot(2);
    let tokens = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<Vec<_>>()
    };
    fixture.agents[0].workspace_id = "w0".into();
    fixture.agents[0].display_agent = Some("rev1354-alpha".into());
    fixture.agents[0].name = Some("rev1354-alpha".into());
    fixture.agents[0].tokens = tokens(&[("role", "worker"), ("parent", "p1")]);
    fixture.agents[0].agent_status = AgentStatus::Blocked;
    fixture.agents[1].workspace_id = "w0".into();
    fixture.agents[1].display_agent = Some("orch".into());
    fixture.agents[1].name = Some("orch".into());
    fixture.agents[1].tokens = tokens(&[("role", "ORCH")]);
    let mut tail = fixture.agents[0].clone();
    tail.pane_id = "p2".into();
    tail.tab_id = "none".into();
    tail.display_agent = Some("rev1354-beta".into());
    tail.name = Some("rev1354-beta".into());
    tail.tokens = tokens(&[("role", "worker"), ("parent", "p1")]);
    tail.agent_status = AgentStatus::Working;
    fixture.agents.push(tail);
    fixture
}

/// Opens the fixture with `snapshot` in `sort` and draws it (see `tree.rs`).
fn sorted_window(
    snapshot: ClientShellSnapshot,
    sort: AgentSort,
    cx: &mut gpui::TestAppContext,
) -> (Entity<HerdrWindow>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot));
        view.agent_sort = sort;
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.run_until_parked();
    cx.update(|window, cx| {
        cx.default_global::<TextProbes>().0.clear();
        window.refresh();
        full_draw(window, cx).clear(cx);
    });
    (view, cx)
}

fn assert_above(
    cx: &mut gpui::VisualTestContext,
    upper: &'static str,
    lower: &'static str,
    why: &str,
) {
    let top = cx.debug_bounds(upper).unwrap().top();
    let bottom = cx.debug_bounds(lower).unwrap().top();
    assert!(top < bottom, "{upper} must render above {lower}: {why}");
}

/// Workers listed before the orchestrator still render below it in the
/// `Grouped` tree.
#[gpui::test]
fn workers_listed_first_render_orchestrator_first(cx: &mut gpui::TestAppContext) {
    let (_, cx) = sorted_window(orch_first_snapshot(), AgentSort::Grouped, cx);
    assert_above(cx, "row-agent-p1", "row-agent-p0", "orchestrator first");
    assert_above(cx, "row-agent-p1", "row-agent-p2", "orchestrator first");
}

/// Unit level: with workers listed first, the tree keeps orchestrator, then
/// the ticket job group, then the workers.
#[test]
fn workers_listed_first_keep_orchestrator_above_job_group() {
    let devices = vec![(
        "Local".to_owned(),
        vec![("w0".to_owned(), "herdr".to_owned())],
    )];
    let tokens = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<Vec<_>>()
    };
    let agents = vec![
        (
            "w0".to_owned(),
            "p0".to_owned(),
            "rev1354-alpha".to_owned(),
            tokens(&[("parent", "p1")]),
        ),
        (
            "w0".to_owned(),
            "p1".to_owned(),
            "orch".to_owned(),
            tokens(&[("role", "ORCH")]),
        ),
        (
            "w0".to_owned(),
            "p2".to_owned(),
            "rev1354-beta".to_owned(),
            tokens(&[("parent", "p1")]),
        ),
    ];
    let rows = build_agent_tree(&devices, &agents, &HashSet::new());
    let position = |pane: &str| {
        rows.iter()
            .position(|row| {
                matches!(&row.node, TreeNode::Agent { pane_id, .. } if pane_id.as_str() == pane)
            })
            .unwrap()
    };
    let job = rows
        .iter()
        .position(|row| matches!(&row.node, TreeNode::Job { .. }))
        .unwrap();
    assert!(position("p1") < job, "orchestrator above its job group");
    assert!(job < position("p0"), "job group above worker p0");
    assert!(job < position("p2"), "job group above worker p2");
}
/// The orchestrator renders first in the `Grouped` tree sort.
#[gpui::test]
fn orchestrator_first_in_grouped_sort(cx: &mut gpui::TestAppContext) {
    let (_, cx) = sorted_window(orch_first_snapshot(), AgentSort::Grouped, cx);
    let top = cx.debug_bounds("name-agent-p1").unwrap().top();
    let worker = cx.debug_bounds("name-agent-p0").unwrap().top();
    assert!(top < worker, "orchestrator first in Grouped");
}

/// The orchestrator renders first in the `Priority` flat sort. Known red:
/// the flat sort orders by status, so the `Blocked` worker paints above the
/// orchestrator. One window per test: `debug_bounds` resolves against a
/// single window, so each sort mode gets its own test.
#[gpui::test]
fn orchestrator_first_in_priority_sort(cx: &mut gpui::TestAppContext) {
    let (_, cx) = sorted_window(orch_first_snapshot(), AgentSort::Priority, cx);
    let top = cx.debug_bounds("name-agent-p1").unwrap().top();
    let worker = cx.debug_bounds("name-agent-p0").unwrap().top();
    assert!(top < worker, "orchestrator first in Priority");
}

/// An orchestrator with zero workers still renders first in its group.
#[gpui::test]
fn lone_orchestrator_still_first(cx: &mut gpui::TestAppContext) {
    let mut fixture = snapshot(2);
    let tokens = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<Vec<_>>()
    };
    fixture.agents[0].workspace_id = "w0".into();
    fixture.agents[0].tokens = tokens(&[("role", "worker")]);
    fixture.agents[1].workspace_id = "w0".into();
    fixture.agents[1].tokens = tokens(&[("role", "ORCH")]);
    let (_, cx) = sorted_window(fixture, AgentSort::Grouped, cx);
    assert_above(
        cx,
        "row-agent-p1",
        "row-agent-p0",
        "lone orchestrator first",
    );
}
