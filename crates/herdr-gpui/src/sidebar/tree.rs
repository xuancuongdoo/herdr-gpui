//! Agent tree for the sidebar `spaces` sort: device → space → orchestrator → workers.

#[cfg(test)]
pub(crate) use legacy::build_agent_tree;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use legacy::build_agent_tree_with_status;
pub(crate) use legacy::{TreeNode, TreeRow};

mod job;
#[cfg(test)]
use herdr_client::protocol::{ClientShellAgent, ClientShellSnapshot};

#[cfg(test)]
use super::agents::agent_name;

/// The single orchestrator rule at snapshot level, mirroring the tree's
/// (`model::push_agents`): a role containing `ORCH`, or named as another
/// agent's parent. The flat Priority sort pins these first, as the grouped
/// tree already does.
#[cfg(test)]
pub(super) fn is_orchestrator(snapshot: &ClientShellSnapshot, agent: &ClientShellAgent) -> bool {
    if model::has_orchestrator_role(&agent.tokens) {
        return true;
    }
    let name = agent_name(agent);
    snapshot.agents.iter().any(|other| {
        if other.pane_id == agent.pane_id {
            return false;
        }
        let parent = other
            .tokens
            .iter()
            .find(|(key, _)| key == "parent")
            .map(|(_, value)| value.trim())
            .filter(|value| !value.is_empty());
        let Some(parent) = parent else {
            return false;
        };
        if parent == agent.pane_id {
            return true;
        }
        // A name match counts only when unambiguous, as in `parent_of`: the
        // only agent besides the namer carrying that name is this one.
        parent == name
            && snapshot
                .agents
                .iter()
                .filter(|candidate| {
                    candidate.pane_id != other.pane_id && agent_name(candidate) == parent
                })
                .count()
                == 1
    })
}

/// The tree model: typed fold keys, borrowed inputs, one device per call.
pub(crate) mod model {
    use herdr_client::protocol::AgentStatus;
    use std::collections::HashSet;

    /// Done/idle worker groups under one orchestrator. Collapsed by default;
    /// the view stores expanded keys, so absence means folded.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum WorkerGroup {
        Completed,
        Available,
    }

    impl WorkerGroup {
        pub(crate) fn label(self) -> &'static str {
            match self {
                WorkerGroup::Completed => "Completed",
                WorkerGroup::Available => "Available",
            }
        }

        pub(crate) fn key_suffix(self) -> &'static str {
            match self {
                WorkerGroup::Completed => "done",
                WorkerGroup::Available => "idle",
            }
        }

        pub(crate) fn of(status: AgentStatus) -> Option<WorkerGroup> {
            match status {
                AgentStatus::Done => Some(WorkerGroup::Completed),
                AgentStatus::Idle => Some(WorkerGroup::Available),
                _ => None,
            }
        }
    }

    /// Fold-state key of a tree row. Keys hold daemon IDs, never labels, so a
    /// rename keeps the fold and equal labels never share one.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum TreeKey {
        /// An endpoint's device row, by endpoint ID.
        Device(String),
        /// A workspace row, by endpoint ID and workspace ID.
        Space(String, String),
        /// An endpoint's row for agents whose workspace it does not list.
        Unassigned(String),
        /// A ticket group under an orchestrator, by orchestrator pane ID, so a
        /// rename keeps the fold and equal labels never share one.
        Job(String),
        /// A done/idle worker group, by workspace ID, orchestrator pane ID
        /// and kind. Collapsed by default: presence in the view's expanded
        /// set means expanded.
        Group(String, String, WorkerGroup),
    }

    /// One endpoint: its device row and its workspaces in display order.
    pub(crate) struct DeviceSpec<'a> {
        pub endpoint_id: &'a str,
        pub label: &'a str,
        pub spaces: &'a [SpaceSpec<'a>],
    }

    pub(crate) struct SpaceSpec<'a> {
        pub workspace_id: &'a str,
        pub label: &'a str,
    }

    #[derive(Clone, Copy)]
    pub(crate) struct AgentSpec<'a> {
        pub workspace_id: &'a str,
        pub pane_id: &'a str,
        pub name: &'a str,
        pub tokens: &'a [(String, String)],
        pub status: AgentStatus,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) enum TreeNode<'a> {
        Device {
            label: &'a str,
        },
        Space {
            workspace_id: &'a str,
            label: &'a str,
        },
        /// Agents whose workspace the device does not list.
        Unassigned,
        /// Workers of one orchestrator sharing a ticket prefix, e.g. label
        /// `REV-1354` for prefix `rev1354-`. Workers sit one depth below.
        Job {
            label: String,
            total: usize,
            orchestrator: &'a str,
        },
        /// Workers of one orchestrator sharing a status (`Completed` for
        /// done, `Available` for idle). Members sit one depth below, sorted
        /// by pane ID. Collapsed by default.
        Group {
            workspace_id: &'a str,
            orchestrator: &'a str,
            kind: WorkerGroup,
            total: usize,
        },
        Agent {
            pane_id: &'a str,
            orchestrator: bool,
            /// Display name: the worker name with the job prefix stripped when
            /// grouped, otherwise the full agent name.
            name: &'a str,
        },
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct TreeRow<'a> {
        pub node: TreeNode<'a>,
        /// Fold key; `None` for rows that cannot fold (agents).
        pub key: Option<TreeKey>,
        pub depth: u8,
        pub last_child: bool,
        /// Whether each ancestor was its parent's last child; only the first
        /// `depth` entries apply (see [`TreeRow::ancestors`]).
        pub ancestors_last: [bool; 4],
        pub has_children: bool,
        pub collapsed: bool,
    }

    impl TreeRow<'_> {
        /// The `last_child` flag of each ancestor, root first.
        pub(crate) fn ancestors(&self) -> &[bool] {
            &self.ancestors_last[..usize::from(self.depth)]
        }
    }
    /// The value of token `key`, when present and not blank.
    fn token<'a>(tokens: &'a [(String, String)], key: &str) -> Option<&'a str> {
        tokens
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.trim())
            .filter(|v| !v.is_empty())
    }

    /// Explicit orchestrator role (trimmed): lowercase `orchestrator` from the
    /// daemon, or a legacy `ORCH` marker. Parent-naming stays with the callers.
    pub(crate) fn is_orchestrator_role(role: &str) -> bool {
        role.eq_ignore_ascii_case("orchestrator") || role.contains("ORCH")
    }

    /// The single orchestrator role rule: [`is_orchestrator_role`] on the
    /// `role` token.
    pub(super) fn has_orchestrator_role(tokens: &[(String, String)]) -> bool {
        token(tokens, "role").is_some_and(is_orchestrator_role)
    }

    /// The parent of `agents[i]` in its space. A `parent` token matches a pane ID
    /// first; otherwise it matches an agent name only when exactly one other agent
    /// has that name, so duplicate names never pick a parent by input order.
    fn parent_of(agents: &[&AgentSpec<'_>], i: usize) -> Option<usize> {
        let parent = token(agents[i].tokens, "parent")?;
        let others = || agents.iter().enumerate().filter(move |&(j, _)| j != i);
        if let Some((j, _)) = others().find(|(_, agent)| agent.pane_id == parent) {
            return Some(j);
        }
        let mut named = others().filter(|(_, agent)| agent.name == parent);
        match (named.next(), named.next()) {
            (Some((j, _)), None) => Some(j),
            _ => None,
        }
    }
    // Stable ordering: workers keep input order; the tree never re-sorts by
    // status. No helper needed: `push_agents` preserves input order throughout.

    fn agent_row<'a>(
        agent: &AgentSpec<'a>,
        orchestrator: bool,
        depth: u8,
        last_child: bool,
        ancestors_last: [bool; 4],
        has_children: bool,
        name: &'a str,
    ) -> TreeRow<'a> {
        TreeRow {
            node: TreeNode::Agent {
                pane_id: agent.pane_id,
                orchestrator,
                name,
            },
            key: None,
            depth,
            last_child,
            ancestors_last,
            has_children,
            collapsed: false,
        }
    }

    /// String key for a status group in the view's `tree_collapsed` set:
    /// `grp:<workspace>:<orchestrator>:<done|idle>`. Presence means expanded;
    /// absence means folded (collapsed by default).
    pub(crate) fn group_key_string(
        workspace_id: &str,
        orchestrator: &str,
        kind: WorkerGroup,
    ) -> String {
        format!(
            "grp:{}:{}:{}",
            workspace_id,
            orchestrator,
            kind.key_suffix()
        )
    }

    /// Agents of one space: orchestrators at depth 2 with their workers below.
    /// Active workers (working/blocked/...) behave as before: two or more
    /// sharing a ticket prefix group under a job row at depth 3 (members at
    /// depth 4 with stripped display names, full IDs kept); unrelated workers
    /// stay directly at depth 3, all in input order. Done workers collect
    /// under a `Completed` group and idle workers under an `Available` group
    /// (depth 3 header, members at depth 4, each in input order), both
    /// collapsed by default and expanded via [`group_key_string`] keys.
    /// Then agents without an orchestrator at depth 2, each in input order.
    /// (Stable ordering and selected-pin live in lane sr-groups-order.)
    fn push_agents<'a>(
        rows: &mut Vec<TreeRow<'a>>,
        agents: &[&AgentSpec<'a>],
        space_last: bool,
        collapsed: &HashSet<TreeKey>,
        expanded: &HashSet<TreeKey>,
    ) {
        let parents: Vec<Option<usize>> = (0..agents.len()).map(|i| parent_of(agents, i)).collect();
        // The single orchestrator rule: `has_orchestrator_role`, or named as a
        // parent by another agent in the same space.
        let orchestrator: Vec<bool> = (0..agents.len())
            .map(|i| has_orchestrator_role(agents[i].tokens) || parents.contains(&Some(i)))
            .collect();
        let (parents, orchestrator) = (&parents, &orchestrator);
        // A parent is always an orchestrator, so a non-orchestrator with a parent
        // is a worker; everything else is top level.
        let workers_of = |o: usize| {
            (0..agents.len()).filter(move |&w| !orchestrator[w] && parents[w] == Some(o))
        };
        let top_level = (0..agents.len())
            .filter(|&i| orchestrator[i] || parents[i].is_none())
            .count();
        let mut position = 0;
        for o in (0..agents.len()).filter(|&o| orchestrator[o]) {
            position += 1;
            let last = position == top_level;
            let worker_ids: Vec<usize> = workers_of(o).collect();
            rows.push(agent_row(
                agents[o],
                true,
                2,
                last,
                [true, space_last, false, false],
                !worker_ids.is_empty(),
                agents[o].name,
            ));
            if worker_ids.is_empty() {
                continue;
            }
            // Partition workers by status, keeping input order.
            let mut visible: Vec<usize> = Vec::new();
            let mut done: Vec<usize> = Vec::new();
            let mut idle: Vec<usize> = Vec::new();
            for w in worker_ids {
                match WorkerGroup::of(agents[w].status) {
                    Some(WorkerGroup::Completed) => done.push(w),
                    Some(WorkerGroup::Available) => idle.push(w),
                    None => visible.push(w),
                }
            }
            let ws = agents[o].workspace_id;
            let done_key = TreeKey::Group(
                ws.to_owned(),
                agents[o].pane_id.to_owned(),
                WorkerGroup::Completed,
            );
            let idle_key = TreeKey::Group(
                ws.to_owned(),
                agents[o].pane_id.to_owned(),
                WorkerGroup::Available,
            );
            let done_open = expanded.contains(&done_key);
            let idle_open = expanded.contains(&idle_key);
            // Partition visible workers by shared ticket: each ticket shared by
            // 2+ workers gets one job row at depth 3, headed at its first
            // member, with members at depth 4 under stripped display names
            // (full IDs kept); other workers stay directly at depth 3, all in
            // input order. An orchestrator's jobs share one fold key and
            // collapse together.
            let mut groups: Vec<(Option<&str>, Vec<usize>)> = Vec::new();
            let mut rest: Vec<usize> = visible;
            while !rest.is_empty() {
                let head = rest.remove(0);
                let mut prefix = None;
                let mut hit = vec![head];
                let mut miss = Vec::new();
                for w in rest {
                    match super::job::job_prefix(&[agents[head].name, agents[w].name]) {
                        Some(ticket) => {
                            prefix = Some(ticket);
                            hit.push(w);
                        }
                        None => miss.push(w),
                    }
                }
                groups.push((prefix, hit));
                rest = miss;
            }
            let key = TreeKey::Job(agents[o].pane_id.to_owned());
            let folded = collapsed.contains(&key);
            let mut status_groups: Vec<(WorkerGroup, TreeKey, Vec<usize>, bool)> = Vec::new();
            if !done.is_empty() {
                status_groups.push((WorkerGroup::Completed, done_key, done, done_open));
            }
            if !idle.is_empty() {
                status_groups.push((WorkerGroup::Available, idle_key, idle, idle_open));
            }
            let sections = groups.len() + status_groups.len();
            let mut section = 0;
            for (ticket, members) in groups.iter() {
                let tail = section + 1 == sections;
                section += 1;
                match ticket {
                    Some(prefix) => {
                        rows.push(TreeRow {
                            node: TreeNode::Job {
                                label: super::job::job_label(prefix),
                                total: members.len(),
                                orchestrator: agents[o].pane_id,
                            },
                            key: Some(key.clone()),
                            depth: 3,
                            last_child: tail && folded,
                            ancestors_last: [true, space_last, last, false],
                            has_children: true,
                            collapsed: folded,
                        });
                        if folded {
                            continue;
                        }
                        for (k, &w) in members.iter().enumerate() {
                            rows.push(agent_row(
                                agents[w],
                                false,
                                4,
                                tail && k + 1 == members.len(),
                                [true, space_last, last, tail],
                                false,
                                super::job::display_name(agents[w].name, prefix),
                            ));
                        }
                    }
                    None => {
                        for (k, &w) in members.iter().enumerate() {
                            rows.push(agent_row(
                                agents[w],
                                false,
                                3,
                                tail && k + 1 == members.len(),
                                [true, space_last, last, false],
                                false,
                                agents[w].name,
                            ));
                        }
                    }
                }
            }
            for (kind, gkey, members, open) in &status_groups {
                let tail = section + 1 == sections;
                section += 1;
                rows.push(TreeRow {
                    node: TreeNode::Group {
                        workspace_id: ws,
                        orchestrator: agents[o].pane_id,
                        kind: *kind,
                        total: members.len(),
                    },
                    key: Some(gkey.clone()),
                    depth: 3,
                    last_child: tail && !*open,
                    ancestors_last: [true, space_last, last, false],
                    has_children: true,
                    collapsed: !*open,
                });
                if !*open {
                    continue;
                }
                for (k, &w) in members.iter().enumerate() {
                    rows.push(agent_row(
                        agents[w],
                        false,
                        4,
                        tail && k + 1 == members.len(),
                        [true, space_last, last, tail],
                        false,
                        agents[w].name,
                    ));
                }
            }
        }
        for u in (0..agents.len()).filter(|&u| !orchestrator[u] && parents[u].is_none()) {
            position += 1;
            rows.push(agent_row(
                agents[u],
                false,
                2,
                position == top_level,
                [true, space_last, false, false],
                false,
                agents[u].name,
            ));
        }
    }

    /// A depth-1 row and, unless folded, its agents.
    fn push_space<'a>(
        rows: &mut Vec<TreeRow<'a>>,
        node: TreeNode<'a>,
        key: TreeKey,
        last: bool,
        agents: &[&AgentSpec<'a>],
        collapsed: &HashSet<TreeKey>,
        expanded: &HashSet<TreeKey>,
    ) {
        let has_children = !agents.is_empty();
        let folded = has_children && collapsed.contains(&key);
        rows.push(TreeRow {
            node,
            key: Some(key),
            depth: 1,
            last_child: last,
            ancestors_last: [true, false, false, false],
            has_children,
            collapsed: folded,
        });
        if !folded {
            push_agents(rows, agents, last, collapsed, expanded);
        }
    }

    /// Rows for one endpoint: its device row, one space row per listed workspace
    /// (even with no agents) and the agents under their space. Agents whose
    /// workspace is not listed collect under a final [`TreeNode::Unassigned`]
    /// row. A folded row keeps itself and hides its descendants.
    pub(crate) fn build_agent_tree<'a>(
        device: &DeviceSpec<'a>,
        agents: &[AgentSpec<'a>],
        collapsed: &HashSet<TreeKey>,
        expanded: &HashSet<TreeKey>,
    ) -> Vec<TreeRow<'a>> {
        let listed = |workspace_id: &str| {
            device
                .spaces
                .iter()
                .any(|space| space.workspace_id == workspace_id)
        };
        let unassigned: Vec<&AgentSpec<'a>> = agents
            .iter()
            .filter(|agent| !listed(agent.workspace_id))
            .collect();
        let key = TreeKey::Device(device.endpoint_id.to_owned());
        let has_children = !device.spaces.is_empty() || !unassigned.is_empty();
        let folded = has_children && collapsed.contains(&key);
        let mut rows = vec![TreeRow {
            node: TreeNode::Device {
                label: device.label,
            },
            key: Some(key),
            depth: 0,
            last_child: true,
            ancestors_last: [false; 4],
            has_children,
            collapsed: folded,
        }];
        if folded {
            return rows;
        }
        for (index, space) in device.spaces.iter().enumerate() {
            let space_agents: Vec<&AgentSpec<'a>> = agents
                .iter()
                .filter(|agent| agent.workspace_id == space.workspace_id)
                .collect();
            push_space(
                &mut rows,
                TreeNode::Space {
                    workspace_id: space.workspace_id,
                    label: space.label,
                },
                TreeKey::Space(device.endpoint_id.to_owned(), space.workspace_id.to_owned()),
                index + 1 == device.spaces.len() && unassigned.is_empty(),
                &space_agents,
                collapsed,
                expanded,
            );
        }
        if !unassigned.is_empty() {
            push_space(
                &mut rows,
                TreeNode::Unassigned,
                TreeKey::Unassigned(device.endpoint_id.to_owned()),
                true,
                &unassigned,
                collapsed,
                expanded,
            );
        }
        rows
    }
}

/// Old-shape bridge so `tree/render.rs` and the window tests keep compiling
/// until they switch to [`model`]; delete it with that switch.
mod legacy {
    use super::model::{
        self, AgentSpec, DeviceSpec, SpaceSpec, TreeKey, WorkerGroup, group_key_string,
    };
    use herdr_client::protocol::AgentStatus;
    use std::collections::HashSet;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) enum TreeNode {
        Device {
            label: String,
        },
        Space {
            workspace_id: String,
            label: String,
        },
        Job {
            label: String,
            total: usize,
            orchestrator: String,
        },
        /// Done/idle workers of one orchestrator (`Completed`/`Available`).
        /// Rendered like a job header; sr-rows owns the final text.
        Group {
            workspace_id: String,
            orchestrator: String,
            kind: WorkerGroup,
            total: usize,
        },
        Agent {
            pane_id: String,
            orchestrator: bool,
            name: String,
        },
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct TreeRow {
        pub node: TreeNode,
        pub depth: u8,
        pub last_child: bool,
        pub ancestors_last: Vec<bool>,
        pub has_children: bool,
        pub collapsed: bool,
    }

    /// Old keys: `dev:<label>`, `ws:<label>:<ws>`, `ws:<label>:unknown`,
    /// `job:<label>:<pane>`, plus `grp:<ws>:<pane>:<done|idle>` (presence
    /// means expanded; status groups are collapsed by default). Each device
    /// uses its label as endpoint ID; unlisted agents go to the last.
    #[cfg(test)]
    type AgentTuple = (String, String, String, Vec<(String, String)>);
    type AgentTupleWithStatus = (String, String, String, Vec<(String, String)>, AgentStatus);

    #[cfg(test)]
    pub(crate) fn build_agent_tree(
        devices: &[(String, Vec<(String, String)>)],
        agents: &[AgentTuple],
        collapsed: &HashSet<String>,
    ) -> Vec<TreeRow> {
        let with_status: Vec<AgentTupleWithStatus> = agents
            .iter()
            .map(|(workspace_id, pane_id, name, tokens)| {
                (
                    workspace_id.clone(),
                    pane_id.clone(),
                    name.clone(),
                    tokens.clone(),
                    AgentStatus::Working,
                )
            })
            .collect();
        build_agent_tree_with_status(devices, &with_status, collapsed)
    }

    /// Same as [`build_agent_tree`], with per-agent daemon statuses so done
    /// workers collect under `Completed` and idle workers under `Available`.
    pub(crate) fn build_agent_tree_with_status(
        devices: &[(String, Vec<(String, String)>)],
        agents: &[AgentTupleWithStatus],
        collapsed: &HashSet<String>,
    ) -> Vec<TreeRow> {
        let specs: Vec<AgentSpec<'_>> = agents
            .iter()
            .map(|(workspace_id, pane_id, name, tokens, status)| AgentSpec {
                workspace_id,
                pane_id,
                name,
                tokens,
                status: *status,
            })
            .collect();
        let listed = |ws: &str| {
            devices
                .iter()
                .any(|(_, s)| s.iter().any(|(id, _)| id == ws))
        };
        let mut out = Vec::new();
        for (index, (label, spaces)) in devices.iter().enumerate() {
            let last = index + 1 == devices.len();
            let spaces: Vec<SpaceSpec<'_>> = spaces
                .iter()
                .map(|(workspace_id, label)| SpaceSpec {
                    workspace_id,
                    label,
                })
                .collect();
            let own: Vec<AgentSpec<'_>> = specs
                .iter()
                .filter(|a| {
                    spaces.iter().any(|s| s.workspace_id == a.workspace_id)
                        || (last && !listed(a.workspace_id))
                })
                .copied()
                .collect();
            let mut keys = HashSet::new();
            if collapsed.contains(&format!("dev:{label}")) {
                keys.insert(TreeKey::Device(label.clone()));
            }
            for space in &spaces {
                if collapsed.contains(&format!("ws:{label}:{}", space.workspace_id)) {
                    keys.insert(TreeKey::Space(label.clone(), space.workspace_id.into()));
                }
            }
            if collapsed.contains(&format!("ws:{label}:unknown")) {
                keys.insert(TreeKey::Unassigned(label.clone()));
            }
            for spec in &specs {
                if collapsed.contains(&format!("job:{label}:{}", spec.pane_id)) {
                    keys.insert(TreeKey::Job(spec.pane_id.to_owned()));
                }
            }
            let mut expanded = HashSet::new();
            for spec in &specs {
                for kind in [WorkerGroup::Completed, WorkerGroup::Available] {
                    if collapsed.contains(&group_key_string(spec.workspace_id, spec.pane_id, kind))
                    {
                        expanded.insert(TreeKey::Group(
                            spec.workspace_id.to_owned(),
                            spec.pane_id.to_owned(),
                            kind,
                        ));
                    }
                }
            }
            let device = DeviceSpec {
                endpoint_id: label,
                label,
                spaces: &spaces,
            };
            for row in model::build_agent_tree(&device, &own, &keys, &expanded) {
                let mut ancestors_last = row.ancestors().to_vec();
                if let Some(first) = ancestors_last.first_mut() {
                    *first = last;
                }
                out.push(TreeRow {
                    node: match row.node {
                        model::TreeNode::Device { label } => TreeNode::Device {
                            label: label.into(),
                        },
                        model::TreeNode::Space {
                            workspace_id,
                            label,
                        } => TreeNode::Space {
                            workspace_id: workspace_id.into(),
                            label: label.into(),
                        },
                        model::TreeNode::Unassigned => TreeNode::Space {
                            workspace_id: "unknown".into(),
                            label: "unknown".into(),
                        },
                        model::TreeNode::Job {
                            label,
                            total,
                            orchestrator,
                            ..
                        } => TreeNode::Job {
                            label,
                            total,
                            orchestrator: orchestrator.into(),
                        },
                        model::TreeNode::Group {
                            workspace_id,
                            orchestrator,
                            kind,
                            total,
                        } => TreeNode::Group {
                            workspace_id: workspace_id.into(),
                            orchestrator: orchestrator.into(),
                            kind,
                            total,
                        },
                        model::TreeNode::Agent {
                            pane_id,
                            orchestrator,
                            name,
                        } => TreeNode::Agent {
                            pane_id: pane_id.into(),
                            orchestrator,
                            name: name.into(),
                        },
                    },
                    depth: row.depth,
                    last_child: if row.depth == 0 { last } else { row.last_child },
                    ancestors_last,
                    has_children: row.has_children,
                    collapsed: row.collapsed,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod render;
