//! With Herdr's tab bar at the top the strip stands in for the header, so
//! the top bar has to show in the strip's trailing room.
use super::*;

#[gpui::test]
fn titlebar_topbar_shows_in_the_merged_strip(cx: &mut TestAppContext) {
    let (_, cx) = shared_tabs(cx, &["a", "b"], "tab_bar_position = 'top'");
    assert!(cx.debug_bounds("titlebar-center").is_none());
    let strip = cx.debug_bounds("strip-titlebar-room").unwrap();
    let topbar = cx.debug_bounds("topbar").unwrap();
    assert!(topbar.left() >= strip.left(), "{topbar:?} {strip:?}");
    assert!(cx.debug_bounds("topbar-brand").is_some());
    assert!(cx.debug_bounds("topbar-search").is_some());
    let tab = cx.debug_bounds("tab-b").unwrap();
    assert!(tab.right() <= strip.left() + px(0.5), "{tab:?} {strip:?}");
}

#[gpui::test]
fn titlebar_topbar_keeps_its_width_when_the_tabs_fill_the_row(cx: &mut TestAppContext) {
    let ids = [
        "alpha-service-deployment-pipeline",
        "bravo-service-deployment-pipeline",
        "charlie-service-deployment-pipeline",
        "delta-service-deployment-pipeline",
        "echo-service-deployment-pipeline",
        "foxtrot-service-deployment-pipeline",
    ];
    let (_, cx) = shared_tabs(cx, &ids, "tab_bar_position = 'top'");
    cx.simulate_resize(size(px(1400.), px(600.)));
    draw(cx);
    draw(cx);
    let topbar = cx.debug_bounds("topbar").unwrap();
    assert!(topbar.size.width >= px(360.), "{topbar:?}");
    let search = cx.debug_bounds("topbar-search").unwrap();
    assert!(search.right() <= px(1400.), "{search:?}");
}
