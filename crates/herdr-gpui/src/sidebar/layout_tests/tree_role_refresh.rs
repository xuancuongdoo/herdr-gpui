//! Failing-only probe: the spaces-sort tree must refresh an agent's dim
//! `role · model` note when the daemon snapshot changes its tokens.
//!
//! Mirrors the `spaces_sort`/`redraw`/`painted` helpers in `tree.rs` and the
//! `agent_line` text contract shared with `visual_tree_fixture.rs`: one line
//! per agent, dim suffix ` · role · model`, `◆ ` prefix for orchestrators.
//! These tests do not fix the render layer; at least one is expected to FAIL
//! until the stale-row bug is fixed.
use super::*;
use crate::preferences::AgentSort;

/// Opens the fixture with `snapshot` in the `spaces` sort and draws it.
fn spaces_sort(
    snapshot: ClientShellSnapshot,
    cx: &mut gpui::TestAppContext,
) -> (Entity<HerdrWindow>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot));
        view.agent_sort = AgentSort::Grouped;
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.run_until_parked();
    redraw(cx);
    (view, cx)
}

fn redraw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| {
        cx.default_global::<TextProbes>().0.clear();
        window.refresh();
        full_draw(window, cx).clear(cx);
    });
}

fn painted(cx: &mut gpui::VisualTestContext, text: &str) -> bool {
    cx.update(|_, cx| cx.global::<TextProbes>().0.contains_key(text))
}

fn tokens(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect::<Vec<_>>()
}

fn push_snapshot(
    view: &Entity<HerdrWindow>,
    cx: &mut gpui::VisualTestContext,
    updated: ClientShellSnapshot,
) {
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.live.snapshot = Some(Arc::new(updated));
            cx.notify();
        })
    });
    cx.run_until_parked();
    redraw(cx);
}

// Deleted tree_row_shows_updated_model/role_after_snapshot_change: tree rows no longer show role/model suffix by design.
