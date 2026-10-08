//! Sidebar search accepts committed IME text (Finding 5): a commit
//! arrives whole ("日本"), not one keystroke per character.

use super::super::{
    keys::StatusFilter, layout_tests::SidebarFixture, layout_tests::fixture_window,
    tree::render::filter_visible,
};
use gpui::prelude::*;
use herdr_client::protocol::AgentStatus;

#[gpui::test]
fn ime_commit_lands_in_query_and_filters(cx: &mut gpui::TestAppContext) {
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| fixture_window(window, cx));
        SidebarFixture(view)
    });
    let view = cx.update(|_, cx| fixture.read(cx).0.clone());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.sidebar_search_open = true;
            // An IME commit arrives whole, not one keystroke per character.
            view.push_sidebar_search_text("日本", cx);
            assert_eq!(view.sidebar_search, "日本");
            let agents = vec![
                (
                    "w".to_owned(),
                    "p1".to_owned(),
                    "日本エージェント".to_owned(),
                    Vec::new(),
                    AgentStatus::Working,
                ),
                (
                    "w".to_owned(),
                    "p2".to_owned(),
                    "review".to_owned(),
                    Vec::new(),
                    AgentStatus::Working,
                ),
            ];
            let visible = filter_visible(&agents, &view.sidebar_search, StatusFilter::All);
            let panes: Vec<&str> = visible.iter().map(|agent| agent.1.as_str()).collect();
            assert_eq!(panes, vec!["p1"]);
        })
    });
}
