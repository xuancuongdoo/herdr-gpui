//! Narrow agent-tree fixture behind the visual mockup: endpoint `datacaliper`
//! with space `orch` and one `REV-1354` ticket job grouping four short-named
//! workers, plus a collapsed `Done` endpoint hiding two agents.
//!
//! The mockup variant in `crate::mockup::demo` paints these same rows at a
//! 160px sidebar column. It cannot reuse this module: `mockup` builds only
//! with its feature flag, while this test module builds in every test run.
//! Keep the two in sync by hand.
use crate::sidebar::tree::render::{agent_line, fold_marker, guides, job_line};
use crate::sidebar::tree::{TreeNode, build_agent_tree};
use std::collections::HashSet;

/// `◐`: the `Working` case of `status_symbol` (`sidebar/agents.rs`); every
/// visible agent in this fixture is working. `◆` comes from `agent_line` for
/// the orchestrator.
const WORKING: &str = "\u{25d0}";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FixtureSpace {
    id: String,
    label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FixtureDevice {
    name: String,
    spaces: Vec<FixtureSpace>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FixtureAttr {
    key: String,
    value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FixtureAgent {
    workspace: String,
    pane_id: String,
    name: String,
    attrs: Vec<FixtureAttr>,
}

fn devices() -> Vec<FixtureDevice> {
    vec![
        FixtureDevice {
            name: "datacaliper".to_owned(),
            spaces: vec![FixtureSpace {
                id: "w-rev".to_owned(),
                label: "orch".to_owned(),
            }],
        },
        FixtureDevice {
            name: "Done".to_owned(),
            spaces: Vec::new(),
        },
    ]
}

fn agents() -> Vec<FixtureAgent> {
    vec![
        FixtureAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-orch".to_owned(),
            name: "rev1354-orch".to_owned(),
            attrs: vec![FixtureAttr {
                key: "role".to_owned(),
                value: "ORCH".to_owned(),
            }],
        },
        FixtureAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-gapfk".to_owned(),
            name: "rev1354-gapfk".to_owned(),
            attrs: vec![FixtureAttr {
                key: "parent".to_owned(),
                value: "p-orch".to_owned(),
            }],
        },
        FixtureAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-glimpact".to_owned(),
            name: "rev1354-glimpact".to_owned(),
            attrs: vec![FixtureAttr {
                key: "parent".to_owned(),
                value: "p-orch".to_owned(),
            }],
        },
        FixtureAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-glhist".to_owned(),
            name: "rev1354-glhist".to_owned(),
            attrs: vec![FixtureAttr {
                key: "parent".to_owned(),
                value: "p-orch".to_owned(),
            }],
        },
        FixtureAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-a1".to_owned(),
            name: "rev1354-a1".to_owned(),
            attrs: vec![FixtureAttr {
                key: "parent".to_owned(),
                value: "p-orch".to_owned(),
            }],
        },
        FixtureAgent {
            workspace: "w-done".to_owned(),
            pane_id: "p-done-1".to_owned(),
            name: "done-1".to_owned(),
            attrs: Vec::new(),
        },
        FixtureAgent {
            workspace: "w-done".to_owned(),
            pane_id: "p-done-2".to_owned(),
            name: "done-2".to_owned(),
            attrs: Vec::new(),
        },
    ]
}

fn collapsed() -> HashSet<String> {
    HashSet::from(["dev:Done".to_owned()])
}

/// Agents of one device by the tree's own rule: listed workspaces, or the
/// last device when no device lists the workspace.
fn assigned(devices: &[FixtureDevice], agents: &[FixtureAgent], index: usize) -> usize {
    let listed = |workspace: &str| {
        devices
            .iter()
            .any(|device| device.spaces.iter().any(|space| space.id == workspace))
    };
    agents
        .iter()
        .filter(|agent| {
            devices[index]
                .spaces
                .iter()
                .any(|space| space.id == agent.workspace)
                || (index + 1 == devices.len() && !listed(&agent.workspace))
        })
        .count()
}

/// Left text of every visible row, using the same helpers production paints
/// with. The collapsed endpoint folds its hidden count into its row.
fn row_texts() -> Vec<String> {
    let devices = devices();
    let agents = agents();
    let device_tuples = devices
        .iter()
        .map(|device| {
            (
                device.name.clone(),
                device
                    .spaces
                    .iter()
                    .map(|space| (space.id.clone(), space.label.clone()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let agent_tuples = agents
        .iter()
        .map(|agent| {
            (
                agent.workspace.clone(),
                agent.pane_id.clone(),
                agent.name.clone(),
                agent
                    .attrs
                    .iter()
                    .map(|attr| (attr.key.clone(), attr.value.clone()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    for row in build_agent_tree(&device_tuples, &agent_tuples, &collapsed()) {
        let text = match &row.node {
            TreeNode::Device { label } if row.collapsed => {
                let hidden = device_tuples
                    .iter()
                    .position(|(name, _)| name == label)
                    .map_or(0, |index| assigned(&devices, &agents, index));
                format!(
                    "{}{} {label} ({hidden})",
                    guides(&row),
                    fold_marker(row.has_children, row.collapsed)
                )
            }
            TreeNode::Device { label } => format!(
                "{}{} {label}",
                guides(&row),
                fold_marker(row.has_children, row.collapsed)
            ),
            TreeNode::Space { label, .. } => format!(
                "{}{} {label}",
                guides(&row),
                fold_marker(row.has_children, row.collapsed)
            ),
            TreeNode::Job { label, total, .. } => {
                job_line(
                    label,
                    *total,
                    row.has_children,
                    row.collapsed,
                    &guides(&row),
                )
                .0
            }
            // Unreachable via `build_agent_tree` (all statuses default to
            // working); kept for exhaustiveness, rendered like a job header.
            TreeNode::Group { kind, total, .. } => {
                job_line(
                    kind.label(),
                    *total,
                    row.has_children,
                    row.collapsed,
                    &guides(&row),
                )
                .0
            }
            TreeNode::Agent {
                name, orchestrator, ..
            } => {
                let role = if *orchestrator { Some("ORCH") } else { None };
                let (line, _) = agent_line(name, role, None);
                format!("{}{WORKING} {line}", guides(&row))
            }
        };
        out.push(text);
    }
    out
}

#[test]
fn visual_tree_shows_ticket_hierarchy_with_short_names() {
    let texts = row_texts();
    assert_eq!(
        texts.len(),
        9,
        "device, space, orch, job, 4 workers, Done: {texts:?}"
    );
    for required in [
        "▾ datacaliper",
        "▾ orch",
        "▾ REV-1354",
        "gapfk",
        "glimpact",
        "glhist",
        "a1",
        "▸ Done (2)",
    ] {
        assert!(
            texts.iter().any(|text| text.contains(required)),
            "missing {required:?} in {texts:?}"
        );
    }
    // Space and ticket job have distinct labels; the ticket shows exactly once.
    assert!(
        texts
            .iter()
            .filter(|text| text.contains("▾ REV-1354"))
            .count()
            == 1,
        "job header shows once: {texts:?}"
    );
    // Workers show stripped short names only: no shared ticket prefix remains.
    for worker in ["gapfk", "glimpact", "glhist", "a1"] {
        let rows: Vec<&String> = texts.iter().filter(|text| text.contains(worker)).collect();
        assert_eq!(rows.len(), 1, "{worker} shows once: {texts:?}");
        assert!(
            !rows[0].contains("rev1354"),
            "{worker} keeps no ticket prefix: {:?}",
            rows[0]
        );
    }
    // The orchestrator keeps its diamond; no row names a model.
    assert!(
        texts.iter().any(|text| text.contains("◆")),
        "orchestrator marker: {texts:?}"
    );
    let joined = texts.join("\n").to_lowercase();
    for model in [
        "opus",
        "sonnet",
        "haiku",
        "muse-spark",
        "flash-2.0",
        "opus-5.5",
    ] {
        assert!(
            !joined.contains(model),
            "model name {model:?} in: {texts:?}"
        );
    }
    // Hierarchy order: device, job, collapsed endpoint last.
    let position = |needle: &str| texts.iter().position(|text| text.contains(needle));
    let device = position("datacaliper");
    let job = position("▾ REV-1354");
    let done = position("Done (2)");
    assert!(
        device < job && job < done,
        "device, job, collapsed endpoint order: {texts:?}"
    );
}
