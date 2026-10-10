#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::model::{
    AgentSpec, DeviceSpec, SpaceSpec, TreeKey, TreeNode, TreeRow, build_agent_tree,
    has_orchestrator_role,
};
use herdr_client::protocol::AgentStatus;
use std::collections::HashSet;

const SPACES: &[SpaceSpec<'static>] = &[
    SpaceSpec {
        workspace_id: "w0",
        label: "herdr",
    },
    SpaceSpec {
        workspace_id: "w1",
        label: "other",
    },
];

fn device(spaces: &'static [SpaceSpec<'static>]) -> DeviceSpec<'static> {
    DeviceSpec {
        endpoint_id: "e0",
        label: "Local",
        spaces,
    }
}

/// Owned agent fixture; [`tree`] borrows it into [`AgentSpec`]s.
struct Agent {
    ws: &'static str,
    pane: &'static str,
    name: &'static str,
    tokens: Vec<(String, String)>,
}

fn agent(ws: &'static str, pane: &'static str) -> Agent {
    Agent {
        ws,
        pane,
        name: pane,
        tokens: Vec::new(),
    }
}

fn orch(ws: &'static str, pane: &'static str, name: &'static str) -> Agent {
    Agent {
        name,
        tokens: vec![("role".into(), "ORCH".into())],
        ..agent(ws, pane)
    }
}

fn worker(ws: &'static str, pane: &'static str, name: &'static str, parent: &str) -> Agent {
    Agent {
        name,
        tokens: vec![("parent".into(), parent.into())],
        ..agent(ws, pane)
    }
}

fn tree_on<'a>(
    device: &DeviceSpec<'a>,
    agents: &'a [Agent],
    collapsed: &HashSet<TreeKey>,
) -> Vec<TreeRow<'a>> {
    let specs: Vec<AgentSpec<'a>> = agents
        .iter()
        .map(|agent| AgentSpec {
            workspace_id: agent.ws,
            pane_id: agent.pane,
            name: agent.name,
            tokens: &agent.tokens,
            status: AgentStatus::Working,
        })
        .collect();
    build_agent_tree(device, &specs, collapsed, &HashSet::new())
}

fn tree(agents: &[Agent]) -> Vec<TreeRow<'_>> {
    tree_on(&device(SPACES), agents, &HashSet::new())
}

fn panes<'a>(rows: &[TreeRow<'a>]) -> Vec<&'a str> {
    rows.iter()
        .filter_map(|row| match row.node {
            TreeNode::Agent { pane_id, .. } => Some(pane_id),
            _ => None,
        })
        .collect()
}

fn pane_pos(rows: &[TreeRow<'_>], pane: &str) -> usize {
    rows.iter()
        .position(|row| matches!(row.node, TreeNode::Agent { pane_id, .. } if pane_id == pane))
        .unwrap_or_else(|| panic!("agent {pane} must have a row: {rows:?}"))
}

fn space_pos(rows: &[TreeRow<'_>], ws: &str) -> usize {
    rows.iter()
        .position(
            |row| matches!(row.node, TreeNode::Space { workspace_id, .. } if workspace_id == ws),
        )
        .unwrap_or_else(|| panic!("space {ws} must have a row: {rows:?}"))
}

fn space_key(ws: &str) -> TreeKey {
    TreeKey::Space("e0".into(), ws.into())
}

#[test]
fn device_then_spaces_then_agents_at_their_depths() {
    let agents = [agent("w0", "p0"), agent("w1", "p1")];
    let rows = tree(&agents);
    assert_eq!(rows[0].node, TreeNode::Device { label: "Local" });
    assert_eq!(rows[0].key, Some(TreeKey::Device("e0".into())));
    assert_eq!(rows[0].depth, 0);
    let (w0, p0) = (space_pos(&rows, "w0"), pane_pos(&rows, "p0"));
    let (w1, p1) = (space_pos(&rows, "w1"), pane_pos(&rows, "p1"));
    assert!(0 < w0 && w0 < p0 && p0 < w1 && w1 < p1, "{rows:?}");
    assert_eq!(rows[w0].depth, 1);
    assert_eq!(rows[w0].key, Some(space_key("w0")));
    assert_eq!(rows[p0].depth, 2);
    assert_eq!(rows[p0].key, None);
}

#[test]
fn workspace_with_no_agents_still_has_a_space_row() {
    let agents = [agent("w0", "p0")];
    let rows = tree(&agents);
    let empty = &rows[space_pos(&rows, "w1")];
    assert!(!empty.has_children);
    assert_eq!(empty.depth, 1);
}

#[test]
fn order_follows_spaces_and_agent_input() {
    let agents = [agent("w1", "p1"), agent("w0", "p0"), agent("w0", "p2")];
    let rows = tree(&agents);
    // Spaces stay in listed order; agents keep input order within a space.
    assert_eq!(panes(&rows), ["p0", "p2", "p1"], "{rows:?}");
    assert_eq!(rows, tree(&agents));
}

#[test]
fn unlisted_workspace_goes_to_unassigned_row() {
    const SPACES: &[SpaceSpec<'static>] = &[SpaceSpec {
        workspace_id: "unknown",
        label: "unknown",
    }];
    let agents = [agent("unknown", "p0"), agent("nope", "orphan")];
    let rows = tree_on(&device(SPACES), &agents, &HashSet::new());
    // A real workspace named `unknown` stays a space; it is no sentinel.
    let named = space_pos(&rows, "unknown");
    assert!(named < pane_pos(&rows, "p0"));
    assert!(!rows[named].last_child);
    let unassigned = rows
        .iter()
        .position(|row| row.node == TreeNode::Unassigned)
        .expect("unlisted agents collect under one row: {rows:?}");
    assert_eq!(rows[unassigned].depth, 1);
    assert!(rows[unassigned].last_child);
    assert_eq!(rows[unassigned].key, Some(TreeKey::Unassigned("e0".into())));
    let orphan = pane_pos(&rows, "orphan");
    assert_eq!(rows[orphan].depth, 2);
    assert!(orphan > unassigned, "{rows:?}");
}

#[test]
fn folding_a_space_by_key_hides_its_agents() {
    let agents = [agent("w0", "p0"), agent("w1", "p1")];
    let collapsed = HashSet::from([space_key("w0")]);
    let rows = tree_on(&device(SPACES), &agents, &collapsed);
    assert!(rows[space_pos(&rows, "w0")].collapsed);
    assert!(!rows[space_pos(&rows, "w1")].collapsed);
    assert_eq!(panes(&rows), ["p1"], "{rows:?}");
}

#[test]
fn fold_key_uses_workspace_id_not_label() {
    const SPACES: &[SpaceSpec<'static>] = &[
        SpaceSpec {
            workspace_id: "w0",
            label: "main",
        },
        SpaceSpec {
            workspace_id: "w1",
            label: "main",
        },
    ];
    let agents = [agent("w0", "p0"), agent("w1", "p1")];
    let collapsed = HashSet::from([space_key("w1")]);
    let rows = tree_on(&device(SPACES), &agents, &collapsed);
    assert!(!rows[space_pos(&rows, "w0")].collapsed);
    assert!(rows[space_pos(&rows, "w1")].collapsed);
    assert_eq!(panes(&rows), ["p0"], "{rows:?}");
    // Another endpoint's key for the same workspace ID does not fold it.
    let other = HashSet::from([TreeKey::Space("e1".into(), "w1".into())]);
    assert_eq!(
        panes(&tree_on(&device(SPACES), &agents, &other)),
        ["p0", "p1"]
    );
}

#[test]
fn folding_the_device_by_key_hides_everything_below() {
    let agents = [agent("w0", "p0"), agent("nope", "orphan")];
    let collapsed = HashSet::from([TreeKey::Device("e0".into())]);
    let rows = tree_on(&device(SPACES), &agents, &collapsed);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].collapsed);
    assert!(rows[0].has_children);
}

#[test]
fn last_child_only_on_last_row_per_group() {
    let agents = [agent("w0", "p0"), agent("w0", "p1"), agent("w1", "p2")];
    let rows = tree(&agents);
    let last = |pos: usize| rows[pos].last_child;
    assert!(last(0));
    assert!(!last(space_pos(&rows, "w0")));
    assert!(last(space_pos(&rows, "w1")));
    assert!(!last(pane_pos(&rows, "p0")));
    assert!(last(pane_pos(&rows, "p1")));
    assert!(last(pane_pos(&rows, "p2")));
}

fn assert_nested(rows: &[TreeRow<'_>], parent: &str, child: &str) {
    let (p, c) = (pane_pos(rows, parent), pane_pos(rows, child));
    assert_eq!(rows[p].depth, 2, "{rows:?}");
    assert!(rows[p].has_children, "{rows:?}");
    assert!(matches!(
        rows[p].node,
        TreeNode::Agent {
            orchestrator: true,
            ..
        }
    ));
    assert_eq!(c, p + 1, "{rows:?}");
    assert_eq!(rows[c].depth, 3, "{rows:?}");
    assert!(matches!(
        rows[c].node,
        TreeNode::Agent {
            orchestrator: false,
            ..
        }
    ));
}

#[test]
fn worker_nests_under_orchestrator_by_pane_id() {
    let agents = [
        orch("w0", "p0", "any-name"),
        worker("w0", "p1", "worker-1", "p0"),
    ];
    assert_nested(&tree(&agents), "p0", "p1");
}

#[test]
fn worker_nests_under_orchestrator_by_unique_name() {
    let agents = [
        orch("w0", "p0", "orch-lead"),
        worker("w0", "p1", "worker-1", "orch-lead"),
    ];
    assert_nested(&tree(&agents), "p0", "p1");
}

#[test]
fn pane_id_match_wins_over_name_match() {
    // `p2` is both the pane ID of the orchestrator and another agent's name.
    let agents = [
        Agent {
            name: "p2",
            ..agent("w0", "p0")
        },
        worker("w0", "p1", "worker-1", "p2"),
        orch("w0", "p2", "lead"),
    ];
    assert_nested(&tree(&agents), "p2", "p1");
}

#[test]
fn ambiguous_parent_name_stays_depth_two() {
    let agents = [
        orch("w0", "p0", "lead"),
        orch("w0", "p1", "lead"),
        worker("w0", "p2", "worker-1", "lead"),
    ];
    let rows = tree(&agents);
    let w = pane_pos(&rows, "p2");
    assert_eq!(rows[w].depth, 2, "{rows:?}");
    assert!(!rows[pane_pos(&rows, "p0")].has_children);
    assert!(!rows[pane_pos(&rows, "p1")].has_children);
}

#[test]
fn agent_named_as_parent_is_an_orchestrator() {
    let agents = [agent("w0", "p0"), worker("w0", "p1", "worker-1", "p0")];
    assert_nested(&tree(&agents), "p0", "p1");
}

#[test]
fn worker_parent_in_another_space_stays_depth_two() {
    let agents = [
        orch("w0", "p0", "orch-w0"),
        worker("w1", "p1", "worker-w1", "orch-w0"),
    ];
    let rows = tree(&agents);
    assert!(!rows[pane_pos(&rows, "p0")].has_children);
    assert_eq!(rows[pane_pos(&rows, "p1")].depth, 2);
}

#[test]
fn worker_unknown_parent_stays_depth_two() {
    let agents = [worker("w0", "p0", "orphan-worker", "ghost-orch")];
    let rows = tree(&agents);
    let row = &rows[pane_pos(&rows, "p0")];
    assert_eq!(row.depth, 2);
    assert!(matches!(
        row.node,
        TreeNode::Agent {
            orchestrator: false,
            ..
        }
    ));
}

#[test]
fn depth_three_last_child_and_ancestors() {
    let agents = [
        orch("w0", "p0", "orch-1"),
        worker("w0", "p1", "worker-1", "orch-1"),
        worker("w0", "p2", "worker-2", "orch-1"),
        agent("w0", "p3"),
        orch("w1", "p4", "orch-2"),
        worker("w1", "p5", "worker-3", "orch-2"),
    ];
    let rows = tree(&agents);
    let row = |pane| &rows[pane_pos(&rows, pane)];
    // Space w0 is not last; p0 is followed by its sibling p3.
    assert_eq!((row("p0").depth, row("p0").last_child), (2, false));
    assert_eq!(row("p0").ancestors(), [true, false]);
    assert_eq!((row("p1").depth, row("p1").last_child), (3, false));
    assert_eq!(row("p1").ancestors(), [true, false, false]);
    assert_eq!((row("p2").depth, row("p2").last_child), (3, true));
    assert_eq!(row("p2").ancestors(), [true, false, false]);
    assert_eq!((row("p3").depth, row("p3").last_child), (2, true));
    assert_eq!(row("p3").ancestors(), [true, false]);
    // Space w1 is last; p4 is its only top-level agent.
    assert_eq!((row("p4").depth, row("p4").last_child), (2, true));
    assert_eq!(row("p4").ancestors(), [true, true]);
    assert_eq!((row("p5").depth, row("p5").last_child), (3, true));
    assert_eq!(row("p5").ancestors(), [true, true, true]);
}

fn jobs(rows: &[TreeRow<'_>]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_, row)| matches!(row.node, TreeNode::Job { .. }))
        .map(|(i, _)| i)
        .collect()
}

fn display_name<'a>(rows: &[TreeRow<'a>], pane: &str) -> &'a str {
    match &rows[pane_pos(rows, pane)].node {
        TreeNode::Agent { name, .. } => name,
        other => panic!("{pane} is not an agent row: {other:?}"),
    }
}

#[test]
fn shared_ticket_groups_workers_under_job() {
    let agents = [
        orch("w0", "p0", "omp"),
        worker("w0", "p1", "rev1354-gapfk", "omp"),
        worker("w0", "p2", "rev1354-glsrc", "omp"),
        worker("w0", "p3", "rev1354-a1", "omp"),
    ];
    let rows = tree(&agents);
    assert_eq!(rows[pane_pos(&rows, "p0")].depth, 2, "{rows:?}");
    let job_rows = jobs(&rows);
    assert_eq!(job_rows.len(), 1, "{rows:?}");
    let job = &rows[job_rows[0]];
    assert!(
        matches!(
            &job.node,
            TreeNode::Job { label, total, .. } if label == "REV-1354" && *total == 3
        ),
        "{rows:?}"
    );
    assert_eq!(job.depth, 3, "{rows:?}");
    assert_eq!(pane_pos(&rows, "p0") + 1, job_rows[0], "{rows:?}");
    for pane in ["p1", "p2", "p3"] {
        assert_eq!(rows[pane_pos(&rows, pane)].depth, 4, "{rows:?}");
    }
    assert_eq!(display_name(&rows, "p1"), "gapfk", "{rows:?}");
    assert_eq!(display_name(&rows, "p2"), "glsrc", "{rows:?}");
    assert_eq!(display_name(&rows, "p3"), "a1", "{rows:?}");
}

#[test]
fn workers_without_shared_prefix_stay_flat() {
    let agents = [
        orch("w0", "p0", "omp"),
        worker("w0", "p1", "alpha", "omp"),
        worker("w0", "p2", "beta", "omp"),
    ];
    let rows = tree(&agents);
    assert!(jobs(&rows).is_empty(), "{rows:?}");
    assert_eq!(rows[pane_pos(&rows, "p1")].depth, 3, "{rows:?}");
    assert_eq!(rows[pane_pos(&rows, "p2")].depth, 3, "{rows:?}");
    assert_eq!(display_name(&rows, "p1"), "alpha", "{rows:?}");
    assert_eq!(display_name(&rows, "p2"), "beta", "{rows:?}");
}

#[test]
fn mixed_prefix_groups_only_matching_workers() {
    let agents = [
        orch("w0", "p0", "omp"),
        worker("w0", "p1", "rev1354-a1", "omp"),
        worker("w0", "p2", "rev1354-b2", "omp"),
        worker("w0", "p3", "other", "omp"),
    ];
    let rows = tree(&agents);
    let job_rows = jobs(&rows);
    assert_eq!(job_rows.len(), 1, "{rows:?}");
    let job = &rows[job_rows[0]];
    assert!(
        matches!(
            &job.node,
            TreeNode::Job { label, total, .. } if label == "REV-1354" && *total == 2
        ),
        "{rows:?}"
    );
    assert_eq!(job.depth, 3, "{rows:?}");
    assert_eq!(rows[pane_pos(&rows, "p1")].depth, 4, "{rows:?}");
    assert_eq!(rows[pane_pos(&rows, "p2")].depth, 4, "{rows:?}");
    assert_eq!(display_name(&rows, "p1"), "a1", "{rows:?}");
    assert_eq!(display_name(&rows, "p2"), "b2", "{rows:?}");
    assert_eq!(rows[pane_pos(&rows, "p3")].depth, 3, "{rows:?}");
    assert_eq!(display_name(&rows, "p3"), "other", "{rows:?}");
}

// Daemon explicit role (/tmp/omp-herdr-role-check/brief.md): the daemon sends
// lowercase `orchestrator`. Both spellings must detect; workers and blank
// roles must not. Fails until the shared role predicate lands. Grouped
// (`push_agents`) and Priority (`is_orchestrator`) both delegate to
// `has_orchestrator_role`, so this covers both sorts.
#[test]
fn lowercase_orchestrator_role_is_detected_like_legacy_orch() {
    for role in [
        "orchestrator",
        "ORCH",
        "*ORCH",
        "Orchestrator",
        "  orchestrator  ",
    ] {
        assert!(
            has_orchestrator_role(&[("role".to_owned(), role.to_owned())]),
            "tokens role {role:?}"
        );
    }
    for role in ["worker", "", "  "] {
        assert!(
            !has_orchestrator_role(&[("role".to_owned(), role.to_owned())]),
            "tokens role {role:?}"
        );
    }
    assert!(!has_orchestrator_role(&[]), "missing role");
}

#[test]
fn lowercase_orchestrator_groups_workers_first() {
    let agents = [
        worker("w0", "p1", "rev1354-a1", "p0"),
        Agent {
            name: "herdr-gpui-orch",
            tokens: vec![("role".into(), "orchestrator".into())],
            ..agent("w0", "p0")
        },
    ];
    let rows = tree(&agents);
    assert_nested(&rows, "p0", "p1");
    assert!(
        pane_pos(&rows, "p0") < pane_pos(&rows, "p1"),
        "orchestrator first: {rows:?}"
    );
}

// Priority sort path: `sidebar/agents.rs` pins `tree::is_orchestrator`
// agents first, and it delegates to the same role predicate.
#[test]
fn lowercase_orchestrator_counts_for_snapshot_sort() {
    let mut snapshot = crate::sidebar::layout_tests::snapshot(1);
    snapshot.agents[0].tokens = vec![("role".to_owned(), "orchestrator".to_owned())];
    assert!(super::is_orchestrator(&snapshot, &snapshot.agents[0]));
    snapshot.agents[0].tokens = vec![("role".to_owned(), "worker".to_owned())];
    assert!(!super::is_orchestrator(&snapshot, &snapshot.agents[0]));
}

// sr-groups-order: workers keep input order; the tree never re-sorts by
// status (`status_priority` stays in the flat Priority sort + workspace
// rollup only).
#[test]
fn workers_keep_input_order_without_status_resort() {
    let fwd = [
        orch("w0", "p0", "orch"),
        worker("w0", "p1", "rev1354-a1", "p0"),
        worker("w0", "p2", "rev1354-a2", "p0"),
    ];
    assert_eq!(panes(&tree(&fwd)), vec!["p0", "p1", "p2"]);
    let rev = [
        orch("w0", "p0", "orch"),
        worker("w0", "p2", "rev1354-a2", "p0"),
        worker("w0", "p1", "rev1354-a1", "p0"),
    ];
    assert_eq!(panes(&tree(&rev)), vec!["p0", "p2", "p1"]);
}
