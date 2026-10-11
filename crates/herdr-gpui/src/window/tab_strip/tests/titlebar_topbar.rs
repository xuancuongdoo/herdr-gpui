//! With Herdr's tab bar at the top the strip stands in for the header, and
//! the header's top bar gets a row of its own under the strip.
use super::*;

#[gpui::test]
fn titlebar_topbar_sits_on_its_own_row_under_the_merged_strip(cx: &mut TestAppContext) {
    let (_, cx) = shared_tabs(cx, &["a", "b"], "tab_bar_position = 'top'");
    assert!(cx.debug_bounds("titlebar-center").is_none());
    let room = cx.debug_bounds("strip-titlebar-room").unwrap();
    let row = cx.debug_bounds("strip-topbar").unwrap();
    let topbar = cx.debug_bounds("topbar").unwrap();
    assert!(row.top() >= room.bottom(), "{row:?} {room:?}");
    assert_eq!(topbar.size.width, row.size.width);
    assert!(cx.debug_bounds("topbar-brand").is_some());
    assert!(cx.debug_bounds("topbar-search").is_some());
}

#[gpui::test]
fn titlebar_topbar_stays_whole_when_the_tabs_fill_the_row(cx: &mut TestAppContext) {
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
    let room = cx.debug_bounds("strip-titlebar-room").unwrap();
    let row = cx.debug_bounds("strip-topbar").unwrap();
    assert!(row.top() >= room.bottom(), "{row:?} {room:?}");
    assert!(row.right() <= px(1400.), "{row:?}");
    for part in ["topbar-brand", "topbar-search", "topbar-help"] {
        let bounds = cx.debug_bounds(part).unwrap();
        assert!(bounds.left() >= row.left(), "{part} {bounds:?} {row:?}");
        assert!(bounds.right() <= row.right(), "{part} {bounds:?} {row:?}");
    }
}
