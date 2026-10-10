use super::*;
use super::super::agents::group_agents_by_space;

#[test]
fn groups_agents_by_space_in_sidebar_workspace_order() {
    let mut snapshot = layout_tests::snapshot(4);
    snapshot.workspaces[0].workspace_id = "ws-resola".into();
    snapshot.workspaces[0].label = "resola".into();
    snapshot.workspaces[1].workspace_id = "ws-datacaliper".into();
    snapshot.workspaces[1].label = "datacaliper".into();
    snapshot.workspaces[2].workspace_id = "ws-omp".into();
    snapshot.workspaces[2].label = ".omp".into();
    snapshot.workspaces[3].workspace_id = "ws-empty".into();
    snapshot.workspaces[3].label = "empty".into();

    let template = snapshot.agents[0].clone();
    let agent = |pane_id: &str, workspace_id: &str| {
        let mut next = template.clone();
        next.pane_id = pane_id.into();
        next.workspace_id = workspace_id.into();
        next
    };

    // Shuffled input order on purpose: groups must still follow workspace order.
    snapshot.agents = vec![
        agent("p-omp-1", "ws-omp"),
        agent("p-resola-1", "ws-resola"),
        agent("p-datacaliper-1", "ws-datacaliper"),
        agent("p-resola-2", "ws-resola"),
        agent("p-omp-2", "ws-omp"),
    ];

    let groups = group_agents_by_space(&snapshot);
    // Empty workspaces emit no group: the implementation skips them.
    assert_eq!(
        groups.len(),
        3,
        "empty workspace must not emit a group: {groups:?}"
    );
    assert_eq!(groups[0].0, "resola");
    assert_eq!(groups[1].0, "datacaliper");
    assert_eq!(groups[2].0, ".omp");

    let panes = |index: usize| {
        groups[index]
            .1
            .iter()
            .map(|agent| agent.pane_id.as_str())
            .collect::<Vec<_>>()
    };
    // Each workspace keeps its agents in snapshot order.
    assert_eq!(panes(0), vec!["p-resola-1", "p-resola-2"]);
    assert_eq!(panes(1), vec!["p-datacaliper-1"]);
    assert_eq!(panes(2), vec!["p-omp-1", "p-omp-2"]);
}
