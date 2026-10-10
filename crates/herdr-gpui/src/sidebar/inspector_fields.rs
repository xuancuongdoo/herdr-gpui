//! Pure data layer for the inspector: one row per agent field.
//!
//! No GPUI rendering here; lane sr-inspector renders these rows.

use herdr_client::protocol::{AgentStatus, ClientShellAgent};

pub(crate) struct InspectorField {
    pub(crate) label: &'static str,
    pub(crate) value: String,
}

fn token<'a>(tokens: &'a [(String, String)], key: &str) -> Option<&'a str> {
    tokens
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
}

fn agent_display_name(agent: &ClientShellAgent) -> &str {
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

fn status_symbol(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Idle => "◌",
        AgentStatus::Working => "◐",
        AgentStatus::Blocked => "!",
        AgentStatus::Done => "✓",
        AgentStatus::Unknown => "?",
    }
}

fn status_word(status: AgentStatus) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    use herdr_client::protocol::AgentStatus;

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
}
