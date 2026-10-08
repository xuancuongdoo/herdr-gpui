#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{TextProbes, fixture_window, full_draw, snapshot};
use crate::preferences::AgentSort;
use gpui::{px, size};
use std::sync::Arc;

/// The spaces sort keeps a custom agents layout on the flat path, so its
/// configured lines and row gap survive; only the default layout gets the tree.
#[gpui::test]
fn spaces_sort_keeps_custom_agent_rows_flat(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot(2)));
        view
    });
    cx.simulate_resize(size(px(800.), px(900.)));
    for custom in [true, false] {
        view.update(cx, |view, cx| {
            view.agent_sort = AgentSort::Grouped;
            view.sidebar_width = Some(320.);
            view.config.usage.inline = true;
            view.config.sidebar_layout.agents = if custom {
                toml::from_str("rows = [[\"state_text\"]]\nrow_gap = 1").unwrap()
            } else {
                Default::default()
            };
            cx.notify();
        });
        cx.run_until_parked();
        let probes = cx.update(|window, cx| {
            cx.default_global::<TextProbes>().0.clear();
            full_draw(window, cx).clear(cx);
            let probes = &cx.global::<TextProbes>().0;
            ["working", "Claude Code"].map(|text| probes.contains_key(text))
        });
        let tree = cx.debug_bounds("tree-agent-p0").is_some();
        assert_eq!(tree, !custom, "custom={custom}: tree guides");
        if custom {
            // Only the configured status word, not the default title line.
            assert_eq!(probes, [true, false]);
            let first = cx.debug_bounds("row-agent-p0").expect("first agent row");
            let second = cx.debug_bounds("row-agent-p1").expect("second agent row");
            assert!(second.top() > first.bottom(), "row_gap was dropped");
        }
    }
}
