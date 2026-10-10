//! missiond graph source (res-03 step a).
//! Parser for `GET /missions/:mid/graph` agent entries.
//! missiond stays authoritative; this module never builds `action.path`.

use serde::Deserialize;

/// One projected agent identity entry from the graph `agents` array.
///
/// Shape matches `crates/missiond/src/projection.rs::project_agent_entry`
/// (`/tmp/resume-res-02/report.md:10-11`, verified `/tmp/verify-res-02/report.md:7`).
/// `identity_id` contains "/" (`mission/node/agent`); never build `action.path`
/// yourself, use the `Resume.path` value or the CLI.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MissionAgent {
    pub identity_id: String,
    pub node_id: String,
    pub role: String,
    pub model_tier: String,
    pub lifecycle_state: String,
    #[serde(default)]
    pub session_ref: Option<String>,
    #[serde(default)]
    pub last_pane: Option<String>,
    #[serde(default)]
    pub live_pane: Option<String>,
    #[serde(default)]
    pub spawner_pane: Option<String>,
    pub action: MissionAction,
}

/// Action descriptor for one agent entry.
///
/// Exact shapes from `projection.rs:443-525`:
/// - `focus` → `{"kind":"focus","pane":...}` (ACTIVE with non-empty pane)
/// - `resume` → `{"kind":"resume","identity_id":...,"session_ref":...,"method":"POST","path":...}`
/// - `unavailable` → `{"kind":"unavailable","reason":...}` where reason is
///   `missing_live_pane` | `missing_session_ref` | `unsupported_lifecycle_state`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum MissionAction {
    Focus {
        pane: String,
    },
    Resume {
        identity_id: String,
        session_ref: String,
        method: String,
        path: String,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug, Deserialize)]
struct GraphEnvelope {
    #[serde(default)]
    agents: Vec<MissionAgent>,
}

/// Extract agent entries from the graph JSON envelope.
///
/// Envelope shape from `projection.rs:737-742` + `build_graph_with_identities`
/// (`server.rs:1775` `get_mission_graph`):
/// `{"mission_id":...,"state":...,"workstreams":...,"agents":[...]}`.
pub fn parse_graph(json: &str) -> Result<Vec<MissionAgent>, serde_json::Error> {
    let env: GraphEnvelope = serde_json::from_str(json)?;
    Ok(env.agents)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn envelope_with_agents(agents_json: &str) -> String {
        format!(
            r#"{{"mission_id":"m-test","state":"WORKING","workstreams":[],"agents":[{agents_json}]}}"#
        )
    }

    #[test]
    fn parses_hibernated_resume_sample() {
        // Exact sample from /tmp/resume-res-02/report.md:10.
        let agent = r#"{"identity_id":"ident-hib","node_id":"n-a","role":"worker","model_tier":"high","lifecycle_state":"HIBERNATED","session_ref":"/tmp/sess-hib.jsonl","last_pane":"w1C:p9H","live_pane":null,"action":{"kind":"resume","identity_id":"ident-hib","session_ref":"/tmp/sess-hib.jsonl","method":"POST","path":"/missions/<mid>/agents/ident-hib/resume"}}"#;
        let agents = parse_graph(&envelope_with_agents(agent)).expect("parse");
        assert_eq!(agents.len(), 1);
        let a = &agents[0];
        assert_eq!(a.identity_id, "ident-hib");
        assert_eq!(a.node_id, "n-a");
        assert_eq!(a.lifecycle_state, "HIBERNATED");
        assert_eq!(a.session_ref.as_deref(), Some("/tmp/sess-hib.jsonl"));
        assert_eq!(a.last_pane.as_deref(), Some("w1C:p9H"));
        assert_eq!(a.live_pane, None);
        assert_eq!(a.spawner_pane, None);
        match &a.action {
            MissionAction::Resume {
                identity_id,
                session_ref,
                method,
                path,
            } => {
                assert_eq!(identity_id, "ident-hib");
                assert_eq!(session_ref, "/tmp/sess-hib.jsonl");
                assert_eq!(method, "POST");
                assert_eq!(path, "/missions/<mid>/agents/ident-hib/resume");
            }
            other => panic!("expected resume, got {other:?}"),
        }
    }

    #[test]
    fn parses_active_focus_sample() {
        // Exact sample from /tmp/resume-res-02/report.md:11.
        let agent = r#"{"identity_id":"ident-active","node_id":"n-a","role":"worker","model_tier":"low","lifecycle_state":"ACTIVE","session_ref":"/tmp/sess-active.jsonl","last_pane":"w1C:p9A","live_pane":"w1C:p9A","action":{"kind":"focus","pane":"w1C:p9A"}}"#;
        let agents = parse_graph(&envelope_with_agents(agent)).expect("parse");
        assert_eq!(agents.len(), 1);
        let a = &agents[0];
        assert_eq!(a.lifecycle_state, "ACTIVE");
        assert_eq!(a.live_pane.as_deref(), Some("w1C:p9A"));
        match &a.action {
            MissionAction::Focus { pane } => assert_eq!(pane, "w1C:p9A"),
            other => panic!("expected focus, got {other:?}"),
        }
    }

    #[test]
    fn parses_unavailable_missing_session_ref() {
        // Sample from projection_identities.rs::parked_without_session_ref_is_unavailable.
        let agent = r#"{"identity_id":"ident-empty","node_id":"n-a","role":"worker","model_tier":"high","lifecycle_state":"PARKED","session_ref":"","last_pane":"w1C:p9E","live_pane":null,"action":{"kind":"unavailable","reason":"missing_session_ref"}}"#;
        let agents = parse_graph(&envelope_with_agents(agent)).expect("parse");
        assert_eq!(agents.len(), 1);
        let a = &agents[0];
        assert_eq!(a.lifecycle_state, "PARKED");
        assert_eq!(a.session_ref.as_deref(), Some(""));
        assert_eq!(a.live_pane, None);
        match &a.action {
            MissionAction::Unavailable { reason } => {
                assert_eq!(reason, "missing_session_ref")
            }
            other => panic!("expected unavailable, got {other:?}"),
        }
    }

    #[test]
    fn preserves_slash_identity_and_encoded_path() {
        // identity_id contains "/"; action.path percent-encodes it ("/" -> "%2F"
        // per res-03 §1, fix 769743e). The parser must keep action.path verbatim
        // and never rebuild it from identity_id.
        let agent = r#"{"identity_id":"m-1/n-a/agent-1","node_id":"n-a","role":"worker","model_tier":"high","lifecycle_state":"HIBERNATED","session_ref":"/tmp/sess-slash.jsonl","last_pane":"w1C:p9H","live_pane":null,"action":{"kind":"resume","identity_id":"m-1/n-a/agent-1","session_ref":"/tmp/sess-slash.jsonl","method":"POST","path":"/missions/m-1/agents/m-1%2Fn-a%2Fagent-1/resume"}}"#;
        let agents = parse_graph(&envelope_with_agents(agent)).expect("parse");
        assert_eq!(agents.len(), 1);
        let a = &agents[0];
        assert_eq!(a.identity_id, "m-1/n-a/agent-1");
        assert!(a.identity_id.contains('/'));
        match &a.action {
            MissionAction::Resume { path, .. } => {
                assert_eq!(path, "/missions/m-1/agents/m-1%2Fn-a%2Fagent-1/resume");
                assert!(path.contains("%2F"), "path must stay encoded: {path}");
            }
            other => panic!("expected resume, got {other:?}"),
        }
    }

    #[test]
    fn parses_spawner_pane_present() {
        let agent = r#"{"identity_id":"ident-hib","node_id":"n-a","role":"worker","model_tier":"high","lifecycle_state":"HIBERNATED","session_ref":"/tmp/sess-hib.jsonl","last_pane":"w1C:p9H","live_pane":null,"spawner_pane":"w1C:p5M","action":{"kind":"resume","identity_id":"ident-hib","session_ref":"/tmp/sess-hib.jsonl","method":"POST","path":"/missions/<mid>/agents/ident-hib/resume"}}"#;
        let agents = parse_graph(&envelope_with_agents(agent)).expect("parse");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].spawner_pane.as_deref(), Some("w1C:p5M"));
    }

    #[test]
    fn parses_spawner_pane_null() {
        let agent = r#"{"identity_id":"ident-hib","node_id":"n-a","role":"worker","model_tier":"high","lifecycle_state":"HIBERNATED","session_ref":"/tmp/sess-hib.jsonl","last_pane":"w1C:p9H","live_pane":null,"spawner_pane":null,"action":{"kind":"resume","identity_id":"ident-hib","session_ref":"/tmp/sess-hib.jsonl","method":"POST","path":"/missions/<mid>/agents/ident-hib/resume"}}"#;
        let agents = parse_graph(&envelope_with_agents(agent)).expect("parse");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].spawner_pane, None);
    }
}
