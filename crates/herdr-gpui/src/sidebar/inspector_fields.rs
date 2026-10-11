//! Pure data layer for the inspector: one row per agent field.
//!
//! No GPUI rendering here; lane sr-inspector renders these rows.

use herdr_client::protocol::{AgentStatus, ClientShellAgent};

pub(crate) struct InspectorField {
    pub(crate) label: &'static str,
    pub(crate) value: String,
}

pub(crate) fn token<'a>(tokens: &'a [(String, String)], key: &str) -> Option<&'a str> {
    tokens
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
}

pub(crate) fn agent_display_name(agent: &ClientShellAgent) -> &str {
    [
        agent.display_agent.as_deref(),
        agent.name.as_deref(),
        agent.agent.as_deref(),
        agent.title.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|s| !s.is_empty())
    .unwrap_or("agent")
}

pub(crate) struct DelegationNode<'a> {
    pub(crate) prefix: &'static str,
    pub(crate) name: &'a str,
    pub(crate) status: AgentStatus,
    pub(crate) current: bool,
}

fn delegation_node<'a>(
    prefix: &'static str,
    agent: &'a ClientShellAgent,
    current: bool,
) -> DelegationNode<'a> {
    DelegationNode {
        prefix,
        name: agent_display_name(agent),
        status: agent.agent_status,
        current,
    }
}

pub(crate) fn delegation_nodes<'a>(
    agent: &'a ClientShellAgent,
    agents: &'a [ClientShellAgent],
) -> Vec<DelegationNode<'a>> {
    let parent = token(&agent.tokens, "parent").map(|id| {
        agents.iter().find(|other| other.pane_id == id).map_or(
            DelegationNode {
                prefix: "",
                name: id,
                status: AgentStatus::Unknown,
                current: false,
            },
            |other| delegation_node("", other, false),
        )
    });
    let children: Vec<_> = agents
        .iter()
        .filter(|other| token(&other.tokens, "parent") == Some(agent.pane_id.as_str()))
        .collect();
    if parent.is_none() && children.is_empty() {
        return Vec::new();
    }
    let last = children.len().saturating_sub(1);
    parent
        .into_iter()
        .chain([delegation_node("", agent, true)])
        .chain(
            children.into_iter().enumerate().map(|(i, child)| {
                delegation_node(if i == last { "└─ " } else { "├─ " }, child, false)
            }),
        )
        .collect()
}

fn status_symbol(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Idle => "◌",
        AgentStatus::Working => "◐",
        AgentStatus::Blocked => "!",
        AgentStatus::Done => "✓",
        AgentStatus::Unknown => "?",
    }
}

pub(crate) fn status_word(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Idle => "idle",
        AgentStatus::Working => "working",
        AgentStatus::Blocked => "blocked",
        AgentStatus::Done => "done",
        AgentStatus::Unknown => "unknown",
    }
}

pub(crate) fn inspector_fields(agent: Option<&ClientShellAgent>) -> Vec<InspectorField> {
    let Some(agent) = agent else {
        return Vec::new();
    };
    let mut out = Vec::new();
    out.push(InspectorField {
        label: "Name",
        value: agent_display_name(agent).to_owned(),
    });
    out.push(InspectorField {
        label: "Status",
        value: format!(
            "{} {}",
            status_symbol(agent.agent_status),
            status_word(agent.agent_status)
        ),
    });
    for (label, key) in [("Role", "role"), ("Model", "model"), ("Parent", "parent")] {
        out.push(InspectorField {
            label,
            value: token(&agent.tokens, key).unwrap_or("—").to_owned(),
        });
    }
    let workspace = agent.workspace_id.trim();
    out.push(InspectorField {
        label: "Workspace",
        value: if workspace.is_empty() {
            "—".to_owned()
        } else {
            workspace.to_owned()
        },
    });
    let pane = agent.pane_id.trim();
    out.push(InspectorField {
        label: "Pane",
        value: if pane.is_empty() {
            "—".to_owned()
        } else {
            pane.to_owned()
        },
    });
    for (key, value) in &agent.tokens {
        if key == "role" || key == "model" || key == "parent" {
            continue;
        }
        let trimmed = value.trim();
        out.push(InspectorField {
            label: "Tokens",
            value: if trimmed.is_empty() {
                key.clone()
            } else {
                format!("{key}: {trimmed}")
            },
        });
    }
    for (state, label) in &agent.state_labels {
        let trimmed = label.trim();
        out.push(InspectorField {
            label: "State",
            value: if trimmed.is_empty() {
                state.clone()
            } else {
                format!("{state}: {trimmed}")
            },
        });
    }
    out
}

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime};

static FIRST_SEEN: LazyLock<Mutex<HashMap<String, (Instant, SystemTime)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Client first-seen time per pane: the mock's elapsed and Started source.
pub(crate) fn first_seen(pane_id: &str) -> (Instant, SystemTime) {
    let mut map = FIRST_SEEN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *map.entry(pane_id.to_owned())
        .or_insert_with(|| (Instant::now(), SystemTime::now()))
}

/// Mock short alias: `claude-opus-5-5` renders `opus-5.5`.
pub(crate) fn short_model(model: &str) -> String {
    let trimmed = model.trim();
    let bare = trimmed.strip_prefix("claude-").unwrap_or(trimmed);
    let parts: Vec<&str> = bare.split('-').collect();
    if parts.len() >= 3 {
        let (major, minor) = (parts[parts.len() - 2], parts[parts.len() - 1]);
        let numeric = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
        if numeric(major) && numeric(minor) {
            return format!("{}-{major}.{minor}", parts[..parts.len() - 2].join("-"));
        }
    }
    bare.to_owned()
}

pub(crate) fn format_elapsed(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    format!("{}h {}m", mins / 60, mins % 60)
}

pub(crate) fn format_started(started: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(started)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use herdr_client::protocol::AgentStatus;
    use std::time::{Duration, SystemTime};

    fn agent(
        tokens: Vec<(&str, &str)>,
        state_labels: Vec<(&str, &str)>,
        status: AgentStatus,
    ) -> ClientShellAgent {
        ClientShellAgent {
            pane_id: "p1".to_owned(),
            workspace_id: "w1".to_owned(),
            tab_id: "t1".to_owned(),
            name: Some("worker-1".to_owned()),
            display_agent: None,
            agent: None,
            title: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_status: status,
            state_change_seq: 1,
            state_labels: state_labels
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
            tokens: tokens
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
            focused: false,
        }
    }

    fn values(fields: &[InspectorField]) -> Vec<(&'static str, &str)> {
        fields.iter().map(|f| (f.label, f.value.as_str())).collect()
    }

    #[test]
    fn full_agent_in_order() {
        let a = agent(
            vec![
                ("role", "worker"),
                ("model", "sonnet"),
                ("parent", "p0"),
                ("task", "REV-1"),
            ],
            vec![("working", "Building")],
            AgentStatus::Working,
        );
        let fields = inspector_fields(Some(&a));
        assert_eq!(
            values(&fields),
            vec![
                ("Name", "worker-1"),
                ("Status", "◐ working"),
                ("Role", "worker"),
                ("Model", "sonnet"),
                ("Parent", "p0"),
                ("Workspace", "w1"),
                ("Pane", "p1"),
                ("Tokens", "task: REV-1"),
                ("State", "working: Building"),
            ]
        );
    }

    #[test]
    fn missing_tokens_use_dash() {
        let a = agent(vec![], vec![], AgentStatus::Idle);
        let fields = inspector_fields(Some(&a));
        assert_eq!(
            values(&fields),
            vec![
                ("Name", "worker-1"),
                ("Status", "◌ idle"),
                ("Role", "—"),
                ("Model", "—"),
                ("Parent", "—"),
                ("Workspace", "w1"),
                ("Pane", "p1"),
            ]
        );
    }

    #[test]
    fn none_agent_is_empty() {
        assert!(inspector_fields(None).is_empty());
    }

    #[test]
    fn orchestrator_role() {
        let a = agent(
            vec![("role", "ORCH"), ("model", "opus")],
            vec![],
            AgentStatus::Done,
        );
        let fields = inspector_fields(Some(&a));
        assert_eq!(
            values(&fields),
            vec![
                ("Name", "worker-1"),
                ("Status", "✓ done"),
                ("Role", "ORCH"),
                ("Model", "opus"),
                ("Parent", "—"),
                ("Workspace", "w1"),
                ("Pane", "p1"),
            ]
        );
    }
    #[test]
    fn short_model_aliases() {
        assert_eq!(short_model("claude-opus-5-5"), "opus-5.5");
        assert_eq!(short_model("opus-5.5"), "opus-5.5");
        assert_eq!(short_model("muse-spark-1.3"), "muse-spark-1.3");
        assert_eq!(short_model("sonnet"), "sonnet");
    }

    #[test]
    fn elapsed_formats() {
        assert_eq!(format_elapsed(Duration::from_secs(12)), "12s");
        assert_eq!(format_elapsed(Duration::from_secs(120)), "2m");
        assert_eq!(format_elapsed(Duration::from_secs(3700)), "1h 1m");
    }

    #[test]
    fn started_renders_timestamp() {
        let text = format_started(SystemTime::UNIX_EPOCH);
        assert_eq!(text.len(), 19);
        assert_eq!(&text[4..5], "-");
    }

    fn linked(pane: &str, name: &str, parent: Option<&str>) -> ClientShellAgent {
        let tokens = parent
            .map(|parent| ("parent", parent))
            .into_iter()
            .collect();
        ClientShellAgent {
            pane_id: pane.to_owned(),
            name: Some(name.to_owned()),
            ..agent(tokens, vec![], AgentStatus::Working)
        }
    }

    fn graph<'a>(
        agent: &'a ClientShellAgent,
        agents: &'a [ClientShellAgent],
    ) -> Vec<(&'static str, &'a str, bool)> {
        delegation_nodes(agent, agents)
            .into_iter()
            .map(|node| (node.prefix, node.name, node.current))
            .collect()
    }

    #[test]
    fn delegation_graph_links_orchestrator_and_workers() {
        let agents = [
            linked("w1:p1", "orch", None),
            linked("w1:p2", "alpha", Some("w1:p1")),
            linked("w1:p3", "beta", Some("w1:p1")),
            linked("w1:p4", "loner", None),
        ];
        assert_eq!(
            graph(&agents[0], &agents),
            vec![
                ("", "orch", true),
                ("├─ ", "alpha", false),
                ("└─ ", "beta", false)
            ]
        );
        assert_eq!(
            graph(&agents[1], &agents),
            vec![("", "orch", false), ("", "alpha", true)]
        );
        assert!(graph(&agents[3], &agents).is_empty());
    }
}
