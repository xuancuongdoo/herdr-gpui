#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::{
    layout::for_mode,
    layout_tests::{fixture_window, full_draw, snapshot},
};
use crate::{config::LayoutMode, preferences::AgentSort};
use gpui::{Bounds, Pixels, px, size};
use std::sync::Arc;

/// One tree agent row as drawn: its space header, guide column, and chrome,
/// and whether the layout's icon slot (with its status dot) was drawn too.
struct TreeRow {
    header: Bounds<Pixels>,
    guides: Bounds<Pixels>,
    row: Bounds<Pixels>,
    slot: bool,
}

/// Draws the agent tree in `mode` and checks the agent row's guides start
/// where its space header's do, whatever chrome the layout wraps them in.
fn tree_row(mode: LayoutMode, cx: &mut gpui::TestAppContext) -> TreeRow {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot(2)));
        view
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    view.update(cx, |view, cx| {
        view.config.layout.mode = mode;
        view.agent_sort = AgentSort::Grouped;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        let _ = full_draw(window, cx);
    });
    let tree = TreeRow {
        header: cx.debug_bounds("tree-space-w0").expect("space header"),
        guides: cx.debug_bounds("tree-agent-p0").expect("agent guides"),
        row: cx.debug_bounds("row-agent-p0").expect("agent row chrome"),
        slot: cx.debug_bounds("icon-agent-p0").is_some(),
    };
    assert_eq!(
        tree.guides.left(),
        tree.header.left() + px(for_mode(mode).content_x()),
        "{mode:?}: guides break from the header's"
    );
    assert!(!tree.slot, "{mode:?}: the tree glyph already shows status");
    tree
}

#[gpui::test]
fn classic_tree_rows_keep_the_flat_chrome(cx: &mut gpui::TestAppContext) {
    let tree = tree_row(LayoutMode::default(), cx);
    assert_eq!(tree.row.left(), tree.header.left());
}

#[gpui::test]
fn superset_tree_rows_keep_its_padded_chrome(cx: &mut gpui::TestAppContext) {
    let tree = tree_row(LayoutMode::Superset, cx);
    let gap = for_mode(LayoutMode::Superset).density.gap();
    assert_eq!(tree.row.left(), tree.header.left());
    assert_eq!(tree.row.size.height, tree.guides.size.height + px(2. * gap));
}

#[gpui::test]
fn orca_tree_rows_sit_in_an_inset_card(cx: &mut gpui::TestAppContext) {
    let tree = tree_row(LayoutMode::Orca, cx);
    assert_eq!(tree.row.left(), tree.header.left() + px(4.));
}

#[gpui::test]
fn minimal_tree_rows_keep_the_flat_chrome(cx: &mut gpui::TestAppContext) {
    let tree = tree_row(LayoutMode::Minimal, cx);
    assert_eq!(tree.row.left(), tree.header.left());
}
