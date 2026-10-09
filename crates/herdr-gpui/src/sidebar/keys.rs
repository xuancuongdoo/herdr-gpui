//! Sidebar agent-tree keyboard navigation (lane sr-keys).
//!
//! Owns: GPUI actions, sidebar key-context bindings, search/filter state
//! helpers and the pure filter predicate. Row rendering and grouping are
//! untouched; group expansion lives with lane sr-groups via `HerdrWindow::toggle_group`.

use crate::{HerdrWindow, navigation::NavigationTarget};
use gpui::{Context, KeyBinding, Window, actions};
use herdr_client::protocol::AgentStatus;

actions!(
    sidebar_tree,
    [
        SidebarUp,
        SidebarDown,
        SidebarOpen,
        SidebarToggle,
        SidebarSearch,
        SidebarCycleFilter,
        SidebarFocusNext,
        SidebarClearSearch
    ]
);

/// Key context for the sidebar tree. Bound keys fire only when focus is
/// inside the sidebar, so typing in the terminal never triggers them.
pub(crate) const KEY_CONTEXT: &str = "HerdrSidebar";

/// Status filter cycle: all → working → blocked → idle → done → all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum StatusFilter {
    #[default]
    All,
    Working,
    Blocked,
    Idle,
    Done,
}

impl StatusFilter {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::All => Self::Working,
            Self::Working => Self::Blocked,
            Self::Blocked => Self::Idle,
            Self::Idle => Self::Done,
            Self::Done => Self::All,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Working => "working",
            Self::Blocked => "blocked",
            Self::Idle => "idle",
            Self::Done => "done",
        }
    }

    pub(crate) fn matches(self, status: AgentStatus) -> bool {
        match self {
            Self::All => true,
            Self::Working => status == AgentStatus::Working,
            Self::Blocked => status == AgentStatus::Blocked,
            Self::Idle => status == AgentStatus::Idle,
            Self::Done => status == AgentStatus::Done,
        }
    }
}

/// Pure filter predicate: status filter plus case-insensitive substring on
/// the agent name. Empty search matches every name that passes the filter.
pub(crate) fn matches_agent(
    name: &str,
    status: AgentStatus,
    search: &str,
    filter: StatusFilter,
) -> bool {
    if !filter.matches(status) {
        return false;
    }
    let query = search.trim();
    if query.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&query.to_lowercase())
}

/// Sidebar-only bindings. Bare keys (`/`, `f`, arrows, …) are safe here
/// because [`KEY_CONTEXT`] is set only on the sidebar container.
pub(crate) fn key_bindings() -> [KeyBinding; 8] {
    [
        KeyBinding::new("up", SidebarUp, Some(KEY_CONTEXT)),
        KeyBinding::new("down", SidebarDown, Some(KEY_CONTEXT)),
        KeyBinding::new("enter", SidebarOpen, Some(KEY_CONTEXT)),
        KeyBinding::new("space", SidebarToggle, Some(KEY_CONTEXT)),
        KeyBinding::new("/", SidebarSearch, Some(KEY_CONTEXT)),
        KeyBinding::new("f", SidebarCycleFilter, Some(KEY_CONTEXT)),
        KeyBinding::new("tab", SidebarFocusNext, Some(KEY_CONTEXT)),
        KeyBinding::new("escape", SidebarClearSearch, Some(KEY_CONTEXT)),
    ]
}

impl HerdrWindow {
    /// Move the tree selection, wrapping around the agent list.
    pub(crate) fn move_sidebar_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.sidebar_visible_agents().len() + self.mission_hibernated().len();
        if len == 0 {
            return;
        }
        let cur = self.sidebar_selected.min(len - 1) as isize;
        self.sidebar_selected = (cur + delta).rem_euclid(len as isize) as usize;
        cx.notify();
    }

    pub(crate) fn clamp_sidebar_selection(&mut self) {
        let len = self.sidebar_visible_agents().len() + self.mission_hibernated().len();
        self.sidebar_selected = if len == 0 {
            0
        } else {
            self.sidebar_selected.min(len - 1)
        };
    }

    /// Open/focus the selected agent's pane. Reuses the same
    /// `navigate`/`navigate_endpoint` path as click-to-open.
    pub(crate) fn open_selected_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // While search is open, Enter closes the input but keeps the query.
        if self.sidebar_search_open {
            self.sidebar_search_open = false;
            window.focus(&self.sidebar_focus, cx);
            cx.notify();
            return;
        }
        self.clamp_sidebar_selection();
        let rows = self.sidebar_visible_agents();
        let mission = self.mission_hibernated();
        if rows.is_empty() && mission.is_empty() {
            return;
        }
        let idx = self.sidebar_selected.min(rows.len() + mission.len() - 1);
        if idx < rows.len() {
            let (host, id) = rows[idx];
            let id = id.to_owned();
            let target = NavigationTarget::Pane(id.as_str());
            if host == self.selected_endpoint {
                self.navigate(target, cx);
            } else if let Some(endpoint) =
                self.endpoints.get(host).map(|endpoint| endpoint.id.clone())
            {
                self.navigate_endpoint(&endpoint, target, cx);
            }
            window.focus(&self.focus, cx);
            return;
        }
        let (mid, agent) = &mission[idx - rows.len()];
        let (mid, id) = (mid.clone(), agent.identity_id.clone());
        self.select_mission_identity(&mid, &id, window, cx);
    }

    /// Expand/collapse the selected node via `HerdrWindow::toggle_group`.
    /// Uses the selected agent's workspace key (`ws:<label>:<workspace>`),
    /// matching the key shape in `tree/render.rs`.
    pub(crate) fn toggle_selected_agent_group(&mut self, cx: &mut Context<Self>) {
        // While search captures text, printable keys (including space) edit
        // the query; the key handler appends them, so skip the toggle here.
        if self.sidebar_search_open {
            return;
        }
        self.clamp_sidebar_selection();
        let rows = self.sidebar_visible_agents();
        if self.sidebar_selected >= rows.len() {
            return;
        }
        let (host, pane_id) = rows[self.sidebar_selected];
        // Find the agent's workspace to build the space key the renderer uses.
        let snapshot = if host == self.selected_endpoint {
            self.live.snapshot.as_ref()
        } else {
            self.endpoints
                .get(host)
                .and_then(|endpoint| endpoint.live.snapshot.as_ref())
        };
        let workspace_id = snapshot.and_then(|snapshot| {
            snapshot
                .agents
                .iter()
                .find(|agent| agent.pane_id == pane_id)
                .map(|agent| agent.workspace_id.clone())
        });
        let label = self
            .endpoints
            .get(host)
            .map(|endpoint| endpoint.label.as_str())
            .unwrap_or("local");
        let key = match workspace_id {
            Some(workspace_id) => format!("ws:{label}:{workspace_id}"),
            None => format!("dev:{label}"),
        };
        self.toggle_group(key, cx);
    }

    /// Focus the sidebar search box.
    pub(crate) fn focus_sidebar_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_search_open = true;
        window.focus(&self.sidebar_search_focus, cx);
        cx.notify();
    }

    /// Clear the search and return focus to the tree.
    pub(crate) fn clear_sidebar_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A press on a row focuses the sidebar (track_focus auto-focus on
        // mouse-down), so escape reaches this HerdrSidebar action instead of
        // the terminal's key_down that cancels drags. Cancel here too.
        let cancelled = self.cancel_workspace_drag(cx) | self.cancel_tab_drag(cx);
        let had_search = !self.sidebar_search.is_empty() || self.sidebar_search_open;
        self.sidebar_search.clear();
        self.sidebar_search_open = false;
        self.clamp_sidebar_selection();
        window.focus(&self.sidebar_focus, cx);
        if had_search {
            cx.notify();
        }
        if cancelled {
            cx.stop_propagation();
            window.prevent_default();
        }
    }

    /// Cycle the status filter all → working → blocked → idle → done → all.
    pub(crate) fn cycle_sidebar_filter(&mut self, cx: &mut Context<Self>) {
        // `f` edits the query while search is open; cycle only when search
        // is not capturing text.
        if self.sidebar_search_open {
            return;
        }
        self.sidebar_status_filter = self.sidebar_status_filter.next();
        self.clamp_sidebar_selection();
        cx.notify();
    }

    /// Plain-text search line for the top of the agent tree area, while
    /// search is open or the query/filter is non-empty.
    pub fn sidebar_search_line(&self) -> Option<String> {
        if !self.sidebar_search_open
            && self.sidebar_search.is_empty()
            && self.sidebar_status_filter == StatusFilter::All
        {
            return None;
        }
        let mut line = format!("/ {}", self.sidebar_search);
        if self.sidebar_status_filter != StatusFilter::All {
            line.push_str(&format!(" · {}", self.sidebar_status_filter.label()));
        }
        Some(line)
    }

    /// Cycle focus tree → chat (terminal) → inspector → tree.
    /// When the inspector is hidden (narrow window), skip it.
    pub(crate) fn focus_next_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sidebar_search_focus.is_focused(window) || self.sidebar_focus.is_focused(window) {
            window.focus(&self.focus, cx);
        } else if self.focus.is_focused(window) {
            let window_width = f32::from(window.viewport_size().width);
            if window_width < crate::inspector::INSPECTOR_MIN_WINDOW_WIDTH {
                window.focus(&self.sidebar_focus, cx);
            } else {
                window.focus(&self.inspector_focus, cx);
            }
        } else {
            window.focus(&self.sidebar_focus, cx);
        }
        cx.notify();
    }
    /// Append committed text to the sidebar query, keeping the tree
    /// selection in range. Committed IME text arrives whole ("日本"), so
    /// this takes the full string instead of one character.
    pub(crate) fn push_sidebar_search_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if text.is_empty() {
            return;
        }
        self.sidebar_search.push_str(text);
        self.clamp_sidebar_selection();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_case_insensitive_substring() {
        assert!(matches_agent(
            "REV-1354-worker",
            AgentStatus::Working,
            "rev-1354",
            StatusFilter::All
        ));
        assert!(matches_agent(
            "Orchestrator",
            AgentStatus::Idle,
            "ORCH",
            StatusFilter::All
        ));
        assert!(!matches_agent(
            "worker-a",
            AgentStatus::Working,
            "worker-b",
            StatusFilter::All
        ));
    }

    #[test]
    fn empty_search_matches_all_statuses_when_filter_all() {
        for status in [
            AgentStatus::Working,
            AgentStatus::Blocked,
            AgentStatus::Idle,
            AgentStatus::Done,
            AgentStatus::Unknown,
        ] {
            assert!(matches_agent("anything", status, "", StatusFilter::All));
            assert!(matches_agent("anything", status, "   ", StatusFilter::All));
        }
    }

    #[test]
    fn status_filter_gates_before_search() {
        assert!(matches_agent(
            "worker",
            AgentStatus::Working,
            "",
            StatusFilter::Working
        ));
        assert!(!matches_agent(
            "worker",
            AgentStatus::Idle,
            "",
            StatusFilter::Working
        ));
        assert!(!matches_agent(
            "worker",
            AgentStatus::Working,
            "worker",
            StatusFilter::Done
        ));
        assert!(matches_agent(
            "worker",
            AgentStatus::Done,
            "WORK",
            StatusFilter::Done
        ));
        // Unknown only passes the `all` filter.
        assert!(!matches_agent(
            "worker",
            AgentStatus::Unknown,
            "",
            StatusFilter::Idle
        ));
    }

    #[test]
    fn filter_cycles_all_working_blocked_idle_done_all() {
        let mut filter = StatusFilter::All;
        let expected = [
            StatusFilter::Working,
            StatusFilter::Blocked,
            StatusFilter::Idle,
            StatusFilter::Done,
            StatusFilter::All,
        ];
        for next in expected {
            filter = filter.next();
            assert_eq!(filter, next);
        }
    }
}
