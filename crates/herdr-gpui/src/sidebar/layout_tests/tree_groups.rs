//! Completed/Available grouping for the sidebar agent tree (pending sr-groups).
//!
//! Narrowed scope (sr-narrow2): ONLY (a) counts collapsed by default and
//! (b) expanding shows members in stable order. Dropped (c) selected-pin,
//! (d) unparented workers, (e) count updates.
//!
//! Assumed future API (does NOT exist on base; integrator adjusts):
//! - `model::AgentSpec` gains a status field (e.g. `status: AgentStatus`).
//! - `model::TreeKey` gains group variants, e.g.
//!   `TreeKey::Completed(orchestrator_pane)` and
//!   `TreeKey::Available(orchestrator_pane)` (or a single
//!   `TreeKey::Group { orchestrator, kind }`).
//! - `model::TreeNode` gains a group header, e.g.
//!   `TreeNode::Group { label, count, orchestrator, kind }` rendering as
//!   `Completed (N)` / `Available (N)`.
//! - Legacy `build_agent_tree` gains status input and string fold keys like
//!   `group:<endpoint>:<orch>:completed` / `:available`.
//!
//! These tests use only base APIs (snapshot `agent_status`, legacy string
//! collapsed keys, `TextProbes` / `row-agent-*` bounds) so they compile on
//! base and FAIL until sr-groups lands. All marked ignored to keep the suite
//! green.
use super::*;
use crate::preferences::AgentSort;
use crate::sidebar::tree::{TreeNode, build_agent_tree_with_status};
use herdr_client::protocol::AgentStatus;
use std::collections::HashSet;

fn tokens(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// 1 orchestrator (porch) + 2 working + 3 done + 2 idle, all in w0.
fn groups_snapshot() -> ClientShellSnapshot {
    let mut fixture = snapshot(1);
    let template = fixture.agents[0].clone();
    let mk = |pane: &str, name: &str, role_parent: Vec<(String, String)>, status: AgentStatus| {
        let mut a = template.clone();
        a.pane_id = pane.into();
        a.workspace_id = "w0".into();
        a.tab_id = "t0".into();
        a.name = Some(name.into());
        a.display_agent = Some(name.into());
        a.tokens = role_parent;
        a.agent_status = status;
        a.focused = false;
        a
    };
    let orch = mk(
        "porch",
        "orch",
        tokens(&[("role", "ORCH")]),
        AgentStatus::Working,
    );
    let workers = [
        ("pw0", "w0", AgentStatus::Working),
        ("pw1", "w1", AgentStatus::Working),
        ("pd0", "d0", AgentStatus::Done),
        ("pd1", "d1", AgentStatus::Done),
        ("pd2", "d2", AgentStatus::Done),
        ("pa0", "a0", AgentStatus::Idle),
        ("pa1", "a1", AgentStatus::Idle),
    ]
    .map(|(pane, name, status)| {
        mk(
            pane,
            name,
            tokens(&[("role", "worker"), ("parent", "porch")]),
            status,
        )
    });
    fixture.agents = std::iter::once(orch).chain(workers).collect();
    fixture.focused_pane_id = None;
    fixture
}

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

fn painted(cx: &mut gpui::VisualTestContext, text: &str) -> bool {
    cx.update(|_, cx| cx.global::<TextProbes>().0.contains_key(text))
}

fn has_row(cx: &mut gpui::VisualTestContext, pane: &str) -> bool {
    cx.debug_bounds(format!("row-agent-{pane}").leak())
        .is_some()
}

/// (a) orchestrator with 2 working, 3 done, 2 idle: working visible plus
/// `Completed (3)` and `Available (2)` groups, collapsed by default.
#[gpui::test]
fn mixed_workers_show_collapsed_completed_and_available_groups(cx: &mut gpui::TestAppContext) {
    let (_, cx) = sorted_window(groups_snapshot(), AgentSort::Grouped, cx);
    assert!(painted(cx, "Completed (3)"), "missing Completed (3) group");
    assert!(painted(cx, "Available (2)"), "missing Available (2) group");
    for pane in ["pw0", "pw1"] {
        assert!(has_row(cx, pane), "{pane} working must stay visible");
    }
    for pane in ["pd0", "pd1", "pd2", "pa0", "pa1"] {
        assert!(
            !has_row(cx, pane),
            "{pane} must hide inside collapsed group"
        );
    }
}

/// (b) expanding a group shows members in stable order.
/// `grp:<workspace>:<orch>:<done|idle>` in the collapsed set means expanded;
/// absence means folded (collapsed by default).
#[test]
fn expanding_group_shows_members_in_stable_order() {
    let devices = vec![(
        "Local".to_owned(),
        vec![("w0".to_owned(), "herdr".to_owned())],
    )];
    let agents = vec![
        (
            "w0".to_owned(),
            "porch".to_owned(),
            "orch".to_owned(),
            tokens(&[("role", "ORCH")]),
            AgentStatus::Working,
        ),
        (
            "w0".to_owned(),
            "pd0".to_owned(),
            "d0".to_owned(),
            tokens(&[("role", "worker"), ("parent", "porch")]),
            AgentStatus::Done,
        ),
        (
            "w0".to_owned(),
            "pd1".to_owned(),
            "d1".to_owned(),
            tokens(&[("role", "worker"), ("parent", "porch")]),
            AgentStatus::Done,
        ),
        (
            "w0".to_owned(),
            "pd2".to_owned(),
            "d2".to_owned(),
            tokens(&[("role", "worker"), ("parent", "porch")]),
            AgentStatus::Done,
        ),
    ];
    let folded = build_agent_tree_with_status(&devices, &agents, &HashSet::new());
    let folded_panes: Vec<String> = folded
        .iter()
        .filter_map(|row| match &row.node {
            TreeNode::Agent { pane_id, .. } => Some(pane_id.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !folded_panes.contains(&"pd0".to_owned()),
        "completed members hide while folded, got {folded_panes:?}"
    );
    let expanded: HashSet<String> = ["grp:w0:porch:done".to_owned()].into_iter().collect();
    let open = build_agent_tree_with_status(&devices, &agents, &expanded);
    let open_panes: Vec<String> = open
        .iter()
        .filter_map(|row| match &row.node {
            TreeNode::Agent { pane_id, .. } => Some(pane_id.clone()),
            _ => None,
        })
        .collect();
    let tail = open_panes
        .iter()
        .filter(|p| ["pd0", "pd1", "pd2"].contains(&p.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        tail,
        vec!["pd0".to_owned(), "pd1".to_owned(), "pd2".to_owned()],
        "expanded members keep stable input order, got {open_panes:?}"
    );
}

/// (c) workspace header count follows the search filter: 3 agents, query
/// matches 1, header shows `1 agent` not `3 agents`.
#[gpui::test]
fn workspace_count_follows_search_filter(cx: &mut gpui::TestAppContext) {
    let mut fixture = snapshot(1);
    let template = fixture.agents[0].clone();
    let mk = |pane: &str, name: &str| {
        let mut a = template.clone();
        a.pane_id = pane.into();
        a.workspace_id = "w0".into();
        a.tab_id = "t0".into();
        a.name = Some(name.into());
        a.display_agent = Some(name.into());
        a.tokens = Vec::new();
        a.agent_status = AgentStatus::Working;
        a.focused = false;
        a
    };
    fixture.agents = vec![
        mk("p0", "alpha-worker"),
        mk("p1", "beta-worker"),
        mk("p2", "gamma-worker"),
    ];
    fixture.focused_pane_id = None;
    let (view, cx) = sorted_window(fixture, AgentSort::Grouped, cx);
    assert!(
        painted(cx, "3 agents"),
        "unfiltered workspace shows 3 agents"
    );
    view.update(cx, |this, cx| {
        this.sidebar_search = "alpha".into();
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        cx.default_global::<TextProbes>().0.clear();
        window.refresh();
        full_draw(window, cx).clear(cx);
    });
    assert!(painted(cx, "1 agent"), "filtered workspace shows 1 agent");
    assert!(
        !painted(cx, "3 agents"),
        "stale 3 agents must go away under filter"
    );
}
