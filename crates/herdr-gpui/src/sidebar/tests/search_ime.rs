use super::super::{
    keys::StatusFilter, layout_tests::fixture_window, tree::render::filter_visible,
};
use herdr_client::protocol::AgentStatus;

#[gpui::test]
fn ime_commit_lands_in_query_and_filters(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| fixture_window(window, cx));
    cx.update(|window, cx| {
        let focus = view.read(cx).sidebar_search.input.read(cx).focus.clone();
        window.focus(&focus, cx);
    });
    cx.simulate_input("日本");
    cx.run_until_parked();
    cx.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.sidebar_search.query(), Some("日本"));
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
        let visible = filter_visible(
            &agents,
            view.sidebar_search.query().unwrap_or(""),
            StatusFilter::All,
        );
        let panes: Vec<&str> = visible.iter().map(|agent| agent.1.as_str()).collect();
        assert_eq!(panes, vec!["p1"]);
    });
}
