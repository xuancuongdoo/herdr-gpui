use super::super::super::agents::{status_symbol, status_text};
use super::agent_line;
use super::job_element_id;
use herdr_client::protocol::AgentStatus;

// Exact-text contract for the sidebar AGENTS agent rows
// (/tmp/tree-restore-models/brief.md): orchestrator and worker glyphs, names,
// and `role · model` suffixes, with no stars. These fail until the render
// change lands; do not weaken them to match the old output.

#[test]
fn orchestrator_row_is_a_diamond_with_an_orchestrator_second_line() {
    assert_eq!(
        agent_line("omp", Some("*ORCH"), Some("claude-opus-4-5")),
        ("\u{25c6} omp".to_owned(), None)
    );
}

#[test]
fn worker_rows_show_the_status_glyph_name_and_state_word() {
    for (status, name, left, right) in [
        (AgentStatus::Working, "gapfk", "\u{25d0} gapfk", "working"),
        (AgentStatus::Blocked, "glimpact", "\u{d7} glimpact", "blocked"),
        (AgentStatus::Idle, "a1", "\u{25cb} a1", "idle"),
        (AgentStatus::Done, "glhist", "\u{2713} glhist", "done"),
    ] {
        assert_eq!(format!("{} {name}", status_symbol(status)), left);
        assert_eq!(status_text(status), right);
    }
}

#[test]
fn failed_workers_show_a_cross_instead_of_a_star() {
    assert_eq!(
        format!("{} s1", status_symbol(AgentStatus::Unknown)),
        "\u{b7} s1"
    );
}

// Deleted worker_rows_carry_role_and_model_suffix: tree rows no longer carry a role/model suffix by design (glyph+name only).

#[test]
fn blank_role_and_model_leave_the_bare_name() {
    assert_eq!(
        agent_line("edge-merge", None, None),
        ("edge-merge".to_owned(), None)
    );
    assert_eq!(
        agent_line("edge-merge", Some("  "), Some("")),
        ("edge-merge".to_owned(), None)
    );
}

// Spacer contract (/tmp/guides-spacer/brief.md): blank guide-only rows
// separate a device's children and wrap an expanded workspace's children
// block, while a collapsed row sits tight with what follows.
use super::super::build_agent_tree;
use super::spacer_between;
use std::collections::HashSet;

#[test]
fn spacers_wrap_device_children_but_not_collapsed_rows() {
    let devices = [(
        "m5".to_owned(),
        vec![
            ("w0".to_owned(), "loadlab".to_owned()),
            ("w1".to_owned(), "datacaliper".to_owned()),
        ],
    )];
    let agents = [(
        "w0".to_owned(),
        "p0".to_owned(),
        "a0".to_owned(),
        Vec::<(String, String)>::new(),
    )];
    let rows = build_agent_tree(&devices, &agents, &HashSet::new());
    // ▾ m5 → │ → ▾ loadlab → │ → a0 → │ → ▾ datacaliper.
    assert_eq!(rows.len(), 4);
    let gaps: Vec<bool> = rows
        .windows(2)
        .map(|pair| spacer_between(&pair[0], &pair[1]).is_some())
        .collect();
    assert_eq!(gaps, vec![true, true, true]);
    for pair in rows.windows(2) {
        if let Some(text) = spacer_between(&pair[0], &pair[1]) {
            assert!(
                !text.contains('├') && !text.contains('└'),
                "connector leaked into spacer {text:?}"
            );
        }
    }
    // ▸ loadlab hides its agent; ▸ loadlab → ▾ datacaliper sits tight.
    let folded = build_agent_tree(&devices, &agents, &HashSet::from(["ws:m5:w0".to_owned()]));
    assert_eq!(folded.len(), 3);
    assert!(spacer_between(&folded[0], &folded[1]).is_some());
    assert!(spacer_between(&folded[1], &folded[2]).is_none());
}

// Job fold collision (/tmp/tree-job-id/brief.md): two same-label tickets on
// different orchestrator panes must get different element ids, and the label
// itself must never appear in the id.
#[test]
fn same_label_jobs_on_different_panes_get_different_element_ids() {
    let first = job_element_id("e0", "p0");
    let second = job_element_id("e0", "p1");
    assert_ne!(first, second);
    assert!(!first.contains("REV-1354") && !second.contains("REV-1354"));
}

// Narrow-column guard (/tmp/tree-narrow-fix/brief.md): at 160px the guides
// are fixed chrome and never squeeze; the label absorbs the shortage and the
// state word drops. This replaces the old squeeze-guides policy, which
// painted `…` over the guides and shifted rows left past them.
#[test]
fn narrow_row_keeps_full_guides_and_truncates_label() {
    use super::tree_agent_widths;
    let glyph_w = 12.0 * 0.62;
    // 120px proxies the 160px column's content width; 12 cols is a depth-3
    // worker (`│  ` × 3 plus `├─ `).
    let content = 120.0;
    let prefix = 12.0 * glyph_w;
    let (guides_w, _, name_w, _, right_w) =
        tree_agent_widths(content, prefix, "\u{25cf}", "gapfk", "working", glyph_w);
    assert_eq!(guides_w, prefix, "guides never squeeze");
    assert_eq!(right_w, 0.0, "state word drops");
    assert!(
        name_w < (5.0 * glyph_w).ceil(),
        "label absorbs the shortage: {name_w}"
    );
    // Guide width is independent of the name: a long name takes no guide room.
    let (guides_long, _, _, _, _) = tree_agent_widths(
        content,
        prefix,
        "\u{25cf}",
        "rev1354-glimpact",
        "blocked",
        glyph_w,
    );
    assert_eq!(guides_long, prefix, "long names keep full guides");
}

#[test]
fn narrow_job_header_drops_count_but_keeps_name_room() {
    use super::job_count_fits;
    let glyph_w = 12.0 * 0.62;
    // `│  │  ▾ ` guides plus `REV-1354` plus `4 workers`, as painted.
    assert!(
        !job_count_fits(
            120.0,
            "\u{2502}  \u{2502}  \u{25be} ",
            "REV-1354",
            "4 workers",
            glyph_w
        ),
        "160px column drops the count so the name keeps room"
    );
    assert!(
        job_count_fits(
            440.0,
            "\u{2502}  \u{2502}  \u{25be} ",
            "REV-1354",
            "4 workers",
            glyph_w
        ),
        "500px column keeps the count"
    );
}

// Daemon explicit role (/tmp/omp-herdr-role-check/brief.md): the daemon sends
// lowercase `orchestrator`, which must paint exactly like the legacy `ORCH`
// markers. Fails until the shared role predicate lands.
#[test]
fn lowercase_orchestrator_role_paints_diamond_and_normalizes() {
    for role in ["orchestrator", "ORCH", "*ORCH", "Orchestrator"] {
        assert_eq!(
            agent_line("herdr-gpui-orch", Some(role), Some("muse-spark-1.3")),
            ("\u{25c6} herdr-gpui-orch".to_owned(), None),
            "role {role:?}"
        );
    }
}

// Search/status filter (sr-search): `filter_visible` keeps matches plus
// their ancestor chain, so an orchestrator stays when any worker matches.
use super::filter_visible;
use super::StatusFilter;

fn filter_agent(
    workspace: &str,
    pane: &str,
    name: &str,
    status: AgentStatus,
    tokens: &[(&str, &str)],
) -> (String, String, String, Vec<(String, String)>, AgentStatus) {
    (
        workspace.to_owned(),
        pane.to_owned(),
        name.to_owned(),
        tokens
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
        status,
    )
}

#[test]
fn search_keeps_matching_worker_and_its_orchestrator() {
    let agents = vec![
        filter_agent(
            "w",
            "o1",
            "orchestrator",
            AgentStatus::Idle,
            &[("role", "orchestrator")],
        ),
        filter_agent(
            "w",
            "w1",
            "rev-1354-worker",
            AgentStatus::Working,
            &[("parent", "o1")],
        ),
        filter_agent(
            "w",
            "w2",
            "other-worker",
            AgentStatus::Working,
            &[("parent", "o1")],
        ),
    ];
    let visible = filter_visible(&agents, "rev-1354", StatusFilter::All);
    let panes: Vec<&str> = visible.iter().map(|agent| agent.1.as_str()).collect();
    assert_eq!(panes, vec!["o1", "w1"]);
}

#[test]
fn status_filter_drops_non_matching_statuses() {
    let agents = vec![
        filter_agent(
            "w",
            "o1",
            "orchestrator",
            AgentStatus::Idle,
            &[("role", "orchestrator")],
        ),
        filter_agent(
            "w",
            "w1",
            "done-worker",
            AgentStatus::Done,
            &[("parent", "o1")],
        ),
        filter_agent(
            "w",
            "w2",
            "idle-worker",
            AgentStatus::Idle,
            &[("parent", "o1")],
        ),
    ];
    let visible = filter_visible(&agents, "", StatusFilter::Done);
    let panes: Vec<&str> = visible.iter().map(|agent| agent.1.as_str()).collect();
    // The done worker matches; its orchestrator stays as context, the idle
    // worker drops.
    assert_eq!(panes, vec!["o1", "w1"]);
}
