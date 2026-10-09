//! res-03 steps (b)+(c): hibernated mission identities in the sidebar.
//!
//! Pure UI helpers only. No pane IDs are invented here; hibernated rows carry
//! no pane and never enter `NavigationTarget::Pane`. The `mission resume` CLI
//! is spawned on the background executor; tests inject the runner so no
//! process spawns.
//!
//! Mission-id source: env `HERDR_MISSION_ID` (comma list). missiond has no
//! list-missions route; with none set the sidebar shows no mission rows and
//! starts no polling.

use std::collections::HashSet;
use std::process::Command;

/// Glyph for hibernated rows: status symbol + name only.
pub(crate) const HIBERNATED_GLYPH: &str = "z";

/// Suffix while a resume CLI is in flight for the row.
pub(crate) const RESUMING_SUFFIX: &str = "resuming\u{2026}";

/// Build the `mission resume` CLI per /tmp/res-03-gpui-cli/brief.md.
/// Integrated build for local use: /Users/xuancuong/Documents/work/missiond-wt/lc-resume/target/release/mission
pub(crate) fn mission_resume_command(
    mid: &str,
    identity_id: &str,
    spawner_pane: Option<&str>,
) -> Command {
    let bin = std::env::var("HERDR_MISSION_BIN").unwrap_or_else(|_| "mission".to_owned());
    let mut cmd = Command::new(bin);
    cmd.arg("resume");
    if let Some(pane) = spawner_pane {
        cmd.args(["--spawner-pane", pane]);
    }
    cmd.args([mid, identity_id]);
    cmd
}

/// Outcome of one `mission resume` run, by exit code (§2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResumeOutcome {
    /// 0: resumed; refresh the graph then focus the new live_pane.
    Resumed,
    /// 2: session missing; never start a new chat.
    SessionMissing,
    /// 3: lease open (300s); row disabled, no second launch.
    LeaseHeld,
    /// 4: identity mismatch; show the fields from stderr.
    Mismatch(String),
    /// 1/other: show stderr.
    Failed(String),
}

/// Map exit code + stderr to UI state, exactly per §2.
pub(crate) fn map_resume_exit(code: Option<i32>, stderr: &str) -> ResumeOutcome {
    let detail = stderr.trim().to_owned();
    match code {
        Some(0) => ResumeOutcome::Resumed,
        Some(2) => ResumeOutcome::SessionMissing,
        Some(3) => ResumeOutcome::LeaseHeld,
        Some(4) => ResumeOutcome::Mismatch(detail),
        _ => ResumeOutcome::Failed(detail),
    }
}

/// Local in-flight guard: a second Enter/click while in flight does nothing.
pub(crate) fn try_begin_resume(inflight: &mut HashSet<String>, identity_id: &str) -> bool {
    inflight.insert(identity_id.to_owned())
}

/// Clear the in-flight flag when the CLI settles.
pub(crate) fn finish_resume(inflight: &mut HashSet<String>, identity_id: &str) {
    inflight.remove(identity_id);
}

/// Launch via an injectable runner so tests never spawn a process.
/// Returns false when the guard blocks the second spawn.
pub(crate) fn launch_resume(
    runner: impl FnOnce(Command),
    mid: &str,
    identity_id: &str,
    spawner_pane: Option<&str>,
    inflight: &mut HashSet<String>,
) -> bool {
    if !try_begin_resume(inflight, identity_id) {
        return false;
    }
    runner(mission_resume_command(mid, identity_id, spawner_pane));
    true
}

/// Split `HERDR_MISSION_ID` comma list; pure for tests.
pub(crate) fn parse_mission_ids(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Mission ids the sidebar refreshes. No list-missions route exists, so this
/// is the only source. Empty means no mission rows and no polling.
pub(crate) fn mission_ids_from_env() -> Vec<String> {
    std::env::var("HERDR_MISSION_ID")
        .map(|v| parse_mission_ids(&v))
        .unwrap_or_default()
}

/// Display name: last `/` segment of `mission/node/agent`.
pub(crate) fn mission_display_name(identity_id: &str) -> &str {
    identity_id.rsplit('/').next().unwrap_or(identity_id)
}

/// `Some(pane)` for ACTIVE (focus it); `None` for hibernated (no pane id,
/// never a fake pane, never `NavigationTarget::Pane`).
pub(crate) fn mission_focus_pane(live_pane: Option<&str>) -> Option<&str> {
    live_pane
}

/// Note for an `Unavailable { reason }` row. `missing_session_ref` shows
/// "session missing"; the other reasons show their reason text.
pub(crate) fn unavailable_note(reason: &str) -> &str {
    if reason == "missing_session_ref" {
        "session missing"
    } else {
        reason
    }
}

/// Unavailable rows are never selectable; Focus rows are. Resume rows need a spawner pane.
pub(crate) fn is_selectable(agent: &crate::mission::graph::MissionAgent) -> bool {
    match &agent.action {
        crate::mission::graph::MissionAction::Unavailable { .. } => false,
        crate::mission::graph::MissionAction::Focus { .. } => true,
        crate::mission::graph::MissionAction::Resume { .. } => agent.spawner_pane.is_some(),
    }
}

/// True when the identity renders as a hibernated row (no live pane).
pub(crate) fn is_hibernated(agent: &crate::mission::graph::MissionAgent) -> bool {
    agent.live_pane.is_none()
}

/// Row suffix: `resuming…` while in flight, else the stored note
/// (`session missing` / mismatch fields / stderr). `None` means no suffix.
pub(crate) fn mission_row_suffix(inflight: bool, note: Option<&str>) -> Option<&str> {
    if inflight {
        Some(RESUMING_SUFFIX)
    } else {
        note
    }
}

/// Row text: symbol + name only, plus the state suffix when present.
pub(crate) fn mission_row_text(name: &str, suffix: Option<&str>) -> String {
    match suffix {
        Some(s) if !s.is_empty() => format!("{HIBERNATED_GLYPH} {name} · {s}"),
        _ => format!("{HIBERNATED_GLYPH} {name}"),
    }
}

/// Note for one mission row: in-flight, lease, Unavailable reason, or stored
/// outcome. `None` means no suffix.
pub(crate) fn mission_row_note(
    agent: &crate::mission::graph::MissionAgent,
    inflight: &HashSet<String>,
    notes: &std::collections::HashMap<String, String>,
    disabled: &HashSet<String>,
) -> Option<String> {
    let stored: Option<&str> = if disabled.contains(&agent.identity_id) {
        Some("resume in progress")
    } else if let crate::mission::graph::MissionAction::Unavailable { reason } = &agent.action {
        Some(unavailable_note(reason))
    } else if matches!(
        agent.action,
        crate::mission::graph::MissionAction::Resume { .. }
    ) && agent.spawner_pane.is_none()
    {
        Some("no spawner pane")
    } else {
        notes.get(&agent.identity_id).map(String::as_str)
    };
    mission_row_suffix(inflight.contains(&agent.identity_id), stored).map(str::to_owned)
}

impl crate::HerdrWindow {
    /// Hibernated rows: `(mid, agent)` with `live_pane: None`, in fetch order.
    /// ACTIVE identities stay on their live pane and are never duplicated here.
    pub(crate) fn mission_hibernated(&self) -> Vec<(String, &crate::mission::graph::MissionAgent)> {
        let fallback = self.mission_ids.first().cloned().unwrap_or_default();
        self.mission_agents
            .iter()
            .filter(|a| is_hibernated(a))
            .map(|a| {
                let mid = self
                    .mission_mid
                    .get(&a.identity_id)
                    .cloned()
                    .unwrap_or_else(|| fallback.clone());
                (mid, a)
            })
            .collect()
    }

    /// Interval refresh off the UI thread. No-op with no mission ids:
    /// no mission rows and no polling.
    pub(crate) fn start_mission_refresh(&self, cx: &mut gpui::Context<Self>) {
        let mids = self.mission_ids.clone();
        if mids.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(5))
                    .await;
                let mids_loop = mids.clone();
                let fetched = cx
                    .background_executor()
                    .spawn(async move {
                        let mut out = Vec::new();
                        for mid in &mids_loop {
                            if let Ok(agents) = crate::mission::client::fetch_graph(mid) {
                                out.push((mid.clone(), agents));
                            }
                        }
                        out
                    })
                    .await;
                let done = this.update(cx, |this, cx| {
                    this.mission_agents = fetched.iter().flat_map(|(_, a)| a.clone()).collect();
                    this.mission_mid = fetched
                        .iter()
                        .flat_map(|(m, a)| {
                            a.iter().map(move |ag| (ag.identity_id.clone(), m.clone()))
                        })
                        .collect();
                    cx.notify();
                });
                if done.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Selection branch for a mission row (Enter/click).
    pub(crate) fn select_mission_identity(
        &mut self,
        mid: &str,
        identity_id: &str,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let Some(agent) = self
            .mission_agents
            .iter()
            .find(|a| a.identity_id == identity_id)
            .cloned()
        else {
            return;
        };
        match &agent.action {
            crate::mission::graph::MissionAction::Focus { pane } => {
                let target =
                    mission_focus_pane(agent.live_pane.as_deref()).unwrap_or(pane.as_str());
                self.navigate(crate::navigation::NavigationTarget::Pane(target), cx);
                window.focus(&self.focus, cx);
            }
            crate::mission::graph::MissionAction::Resume { .. } => {
                if self.mission_disabled.contains(identity_id) {
                    return;
                }
                if !is_selectable(&agent) {
                    return;
                }
                let Some(spawner) = agent.spawner_pane.as_deref() else {
                    return;
                };
                let mid_owned = mid.to_owned();
                let id_owned = identity_id.to_owned();
                let mut bg_slot = None;
                if !launch_resume(
                    |mut cmd| {
                        bg_slot = Some(cx.background_executor().spawn(async move { cmd.output() }));
                    },
                    &mid_owned,
                    &id_owned,
                    // Never fall back to last_pane/live_pane.
                    Some(spawner),
                    &mut self.mission_inflight,
                ) {
                    return;
                }
                let Some(bg) = bg_slot else { return };
                self.mission_notes.remove(&id_owned);
                cx.notify();
                cx.spawn_in(window, async move |this, cx| {
                    let output = bg.await;
                    let (code, stderr) = match &output {
                        Ok(out) => (
                            out.status.code(),
                            String::from_utf8_lossy(&out.stderr).into_owned(),
                        ),
                        Err(e) => (None, e.to_string()),
                    };
                    let outcome = map_resume_exit(code, &stderr);
                    if outcome == ResumeOutcome::Resumed {
                        let mid2 = mid_owned.clone();
                        let fresh = cx
                            .background_executor()
                            .spawn(async move {
                                crate::mission::client::fetch_graph(&mid2).unwrap_or_default()
                            })
                            .await;
                        let _ = this.update(cx, |this, cx| {
                            finish_resume(&mut this.mission_inflight, &id_owned);
                            this.mission_notes.remove(&id_owned);
                            this.mission_disabled.remove(&id_owned);
                            this.mission_agents.retain(|a| {
                                this.mission_mid
                                    .get(&a.identity_id)
                                    .is_some_and(|m| m != &mid_owned)
                            });
                            for a in &fresh {
                                this.mission_mid
                                    .insert(a.identity_id.clone(), mid_owned.clone());
                            }
                            this.mission_agents.extend(fresh.clone());
                            if let Some(pane) = this
                                .mission_agents
                                .iter()
                                .find(|a| a.identity_id == id_owned)
                                .and_then(|a| a.live_pane.clone())
                            {
                                this.navigate(
                                    crate::navigation::NavigationTarget::Pane(pane.as_str()),
                                    cx,
                                );
                            }
                            cx.notify();
                        });
                    } else {
                        let _ = this.update(cx, |this, cx| {
                            finish_resume(&mut this.mission_inflight, &id_owned);
                            match outcome {
                                ResumeOutcome::SessionMissing => {
                                    this.mission_notes
                                        .insert(id_owned.clone(), "session missing".to_owned());
                                }
                                ResumeOutcome::LeaseHeld => {
                                    this.mission_disabled.insert(id_owned.clone());
                                    this.mission_notes.remove(&id_owned);
                                }
                                ResumeOutcome::Mismatch(detail) => {
                                    this.mission_notes.insert(id_owned.clone(), detail);
                                }
                                ResumeOutcome::Failed(detail) => {
                                    // 403 and other errors show as-is, never retried.
                                    this.mission_notes.insert(id_owned.clone(), detail);
                                }
                                ResumeOutcome::Resumed => {}
                            }
                            cx.notify();
                        });
                    }
                })
                .detach();
            }
            crate::mission::graph::MissionAction::Unavailable { reason } => {
                self.mission_notes
                    .insert(identity_id.to_owned(), unavailable_note(reason).to_owned());
                cx.notify();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::graph::MissionAction;

    #[test]
    fn exit_code_maps_to_state() {
        assert_eq!(map_resume_exit(Some(0), "ok"), ResumeOutcome::Resumed);
        assert_eq!(
            map_resume_exit(Some(2), "gone"),
            ResumeOutcome::SessionMissing
        );
        assert_eq!(map_resume_exit(Some(3), ""), ResumeOutcome::LeaseHeld);
        assert_eq!(
            map_resume_exit(Some(4), " role mismatch "),
            ResumeOutcome::Mismatch("role mismatch".into())
        );
        assert_eq!(
            map_resume_exit(Some(1), " boom "),
            ResumeOutcome::Failed("boom".into())
        );
        assert_eq!(
            map_resume_exit(None, "sig"),
            ResumeOutcome::Failed("sig".into())
        );
    }

    #[test]
    fn inflight_guard_blocks_second_spawn() {
        let mut inflight = HashSet::new();
        let mut spawns = 0;
        let first = launch_resume(
            |_| {
                spawns += 1;
            },
            "m1",
            "m1/n/a",
            None,
            &mut inflight,
        );
        assert!(first);
        let second = launch_resume(
            |_| {
                spawns += 1;
            },
            "m1",
            "m1/n/a",
            None,
            &mut inflight,
        );
        assert!(!second);
        assert_eq!(spawns, 1);
        finish_resume(&mut inflight, "m1/n/a");
        let third = launch_resume(
            |_| {
                spawns += 1;
            },
            "m1",
            "m1/n/a",
            None,
            &mut inflight,
        );
        assert!(third);
        assert_eq!(spawns, 2);
    }

    #[test]
    fn hibernated_row_has_no_pane_id() {
        assert_eq!(mission_focus_pane(None), None);
    }

    #[test]
    fn active_identity_routes_to_focus_pane() {
        assert_eq!(mission_focus_pane(Some("w1C:p9A")), Some("w1C:p9A"));
    }

    #[test]
    fn display_name_is_last_segment() {
        assert_eq!(mission_display_name("m/n/worker-1"), "worker-1");
        assert_eq!(mission_display_name("solo"), "solo");
    }

    #[test]
    fn resume_command_args_with_and_without_spawner() {
        let plain = mission_resume_command("m1", "m1/n/a", None);
        assert!(
            !plain
                .get_program()
                .to_string_lossy()
                .into_owned()
                .is_empty()
        );
        let args: Vec<_> = plain
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, vec!["resume", "m1", "m1/n/a"]);
        let with_spawner = mission_resume_command("m1", "m1/n/a", Some("w11:p68"));
        let args: Vec<_> = with_spawner
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec!["resume", "--spawner-pane", "w11:p68", "m1", "m1/n/a"]
        );
    }

    #[test]
    fn mission_ids_split_commas() {
        assert_eq!(
            parse_mission_ids("a,b , ,c"),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );
        assert!(parse_mission_ids("").is_empty());
    }

    #[test]
    fn row_text_is_symbol_plus_name_only() {
        assert_eq!(mission_row_text("worker-1", None), "z worker-1");
        assert_eq!(
            mission_row_text("worker-1", Some("session missing")),
            "z worker-1 · session missing"
        );
        assert_eq!(mission_row_suffix(true, None), Some(RESUMING_SUFFIX));
    }

    fn test_agent(
        action: MissionAction,
        spawner_pane: Option<&str>,
    ) -> crate::mission::graph::MissionAgent {
        crate::mission::graph::MissionAgent {
            identity_id: "m/n/a".into(),
            node_id: "n".into(),
            role: "worker".into(),
            model_tier: "high".into(),
            lifecycle_state: "HIBERNATED".into(),
            session_ref: Some("s".into()),
            last_pane: None,
            live_pane: None,
            spawner_pane: spawner_pane.map(str::to_owned),
            action,
        }
    }

    #[test]
    fn unavailable_note_and_selectability() {
        assert_eq!(unavailable_note("missing_session_ref"), "session missing");
        assert_eq!(unavailable_note("missing_live_pane"), "missing_live_pane");
        assert_eq!(
            unavailable_note("unsupported_lifecycle_state"),
            "unsupported_lifecycle_state"
        );
        let unavailable = test_agent(
            MissionAction::Unavailable {
                reason: "missing_session_ref".into(),
            },
            None,
        );
        assert!(!is_selectable(&unavailable));
        let focus = test_agent(
            MissionAction::Focus {
                pane: "w1C:p9A".into(),
            },
            None,
        );
        assert!(is_selectable(&focus));
        let resume = test_agent(
            MissionAction::Resume {
                identity_id: "m/n/a".into(),
                session_ref: "s".into(),
                method: "POST".into(),
                path: "/p".into(),
            },
            Some("w1C:p5M"),
        );
        assert!(is_selectable(&resume));
    }

    #[test]
    fn resume_without_spawner_is_not_resumable() {
        let agent = test_agent(
            MissionAction::Resume {
                identity_id: "m/n/a".into(),
                session_ref: "s".into(),
                method: "POST".into(),
                path: "/p".into(),
            },
            None,
        );
        assert!(!is_selectable(&agent));
        let note = mission_row_note(
            &agent,
            &HashSet::new(),
            &std::collections::HashMap::new(),
            &HashSet::new(),
        );
        assert_eq!(note.as_deref(), Some("no spawner pane"));
        let mut spawns = 0;
        if is_selectable(&agent) {
            launch_resume(
                |_| {
                    spawns += 1;
                },
                "m1",
                &agent.identity_id,
                agent.spawner_pane.as_deref(),
                &mut HashSet::new(),
            );
        }
        assert_eq!(spawns, 0);
    }
}
