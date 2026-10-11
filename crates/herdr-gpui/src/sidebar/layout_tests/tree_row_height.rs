use super::*;
use crate::config::LayoutMode;

#[gpui::test]
fn tree_agent_rows_grow_with_their_configured_lines(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        let mut listing = snapshot(1);
        let agents: Vec<_> = [
            ("p0", "orchestrator", None),
            ("p1", "worker", Some("p0")),
            ("p2", "worker", Some("p0")),
        ]
        .into_iter()
        .map(|(pane, role, parent)| {
            let mut tokens = vec![["role", role], ["model", "claude-sonnet-5-5"]];
            tokens.extend(parent.map(|parent| ["parent", parent]));
            serde_json::json!({
                "pane_id": pane, "workspace_id": "w0", "tab_id": "t0", "name": pane,
                "display_agent": "Claude Code", "agent": "claude",
                "agent_status": "working", "state_change_seq": 0, "state_labels": [],
                "tokens": tokens, "focused": false
            })
        })
        .collect();
        listing.agents = serde_json::from_value(serde_json::json!(agents)).unwrap();
        view.live.snapshot = Some(Arc::new(listing));
        view.config.sidebar_layout = toml::from_str(
            r#"
            [agents]
            rows = [["state_icon", "workspace"], ["state_text"], ["$model"], ["$role"]]
        "#,
        )
        .unwrap();
        view
    });
    cx.simulate_resize(size(px(800.), px(900.)));
    cx.run_until_parked();
    let keys = [
        ("row-agent-p0", "note-agent-p0-2", "note-agent-p0-3"),
        ("row-agent-p1", "note-agent-p1-2", "note-agent-p1-3"),
        ("row-agent-p2", "note-agent-p2-2", "note-agent-p2-3"),
    ];
    for mode in LayoutMode::ALL
        .into_iter()
        .filter(|mode| mode.lists_workspaces())
    {
        view.update(cx, |view, cx| {
            view.config.layout.mode = mode;
            view.sidebar_width = Some(320.);
            cx.notify();
        });
        cx.update(|window, cx| full_draw(window, cx).clear(cx));
        let rows = keys.map(|(row, ..)| cx.debug_bounds(row).unwrap());
        for (index, (row, last, extra)) in keys.into_iter().enumerate() {
            let last = cx.debug_bounds(last).unwrap();
            assert!(cx.debug_bounds(extra).is_none(), "{mode:?}: {row}");
            assert!(
                last.bottom() <= rows[index].bottom(),
                "{mode:?}: {row} {last:?} {:?}",
                rows[index]
            );
            if let Some(next) = rows.get(index + 1) {
                assert!(
                    rows[index].bottom() <= next.top(),
                    "{mode:?}: {row} {:?} {next:?}",
                    rows[index]
                );
            }
        }
    }
}
