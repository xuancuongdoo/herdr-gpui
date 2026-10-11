use crate::HerdrWindow;
use gpui::{Context, KeyBinding, Window, actions};
use herdr_client::protocol::AgentStatus;

actions!(sidebar_tree, [SidebarCycleFilter]);

pub(crate) const KEY_CONTEXT: &str = "HerdrSidebar";

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

pub(crate) fn key_bindings() -> [KeyBinding; 1] {
    [KeyBinding::new("f", SidebarCycleFilter, Some(KEY_CONTEXT))]
}

impl HerdrWindow {
    pub(crate) fn cycle_sidebar_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sidebar_search.input.read(cx).focus.is_focused(window) {
            return;
        }
        self.sidebar_status_filter = self.sidebar_status_filter.next();
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
