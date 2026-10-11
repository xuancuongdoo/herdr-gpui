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

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx.debug_bounds(selector).unwrap();
    cx.simulate_click(bounds.center(), Modifiers::default());
    draw(cx);
    draw(cx);
}

#[gpui::test]
fn panels_1_keeps_the_topbar_so_panels_3_brings_everything_back(cx: &mut TestAppContext) {
    let (view, cx) = shared_tabs(cx, &["a", "b"], "tab_bar_position = 'top'");
    click(cx, "topbar-panels-1");
    view.read_with(cx, |view, _| {
        assert_eq!(view.panel_preset, crate::window::PanelPreset::TreeOnly)
    });
    assert!(cx.debug_bounds("sidebar").is_some());
    assert!(cx.debug_bounds("terminal").is_none());
    assert!(cx.debug_bounds("inspector-panel").is_none());
    for button in ["topbar-panels-1", "topbar-panels-2", "topbar-panels-3"] {
        assert!(cx.debug_bounds(button).is_some(), "{button}");
    }
    click(cx, "topbar-panels-3");
    assert!(cx.debug_bounds("terminal").is_some());
    assert!(cx.debug_bounds("inspector-panel").is_some());
}

#[gpui::test]
fn topbar_right_controls_stay_inside_the_window_beside_the_inspector(cx: &mut TestAppContext) {
    let (view, cx) = shared_tabs(cx, &["a", "b"], "tab_bar_position = 'top'");
    view.update(cx, |view, _| {
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        for workspace in &mut snapshot.workspaces {
            workspace.label = "a-workspace-with-a-very-long-name-indeed".into();
        }
    });
    cx.simulate_resize(size(px(1400.), px(600.)));
    draw(cx);
    draw(cx);
    let row = cx.debug_bounds("strip-topbar").unwrap();
    let inspector = cx.debug_bounds("inspector-panel").unwrap();
    assert!(
        row.right() <= inspector.left() + px(0.5),
        "{row:?} {inspector:?}"
    );
    for part in ["topbar-panels", "topbar-help", "topbar-settings"] {
        let bounds = cx.debug_bounds(part).unwrap();
        assert!(bounds.left() >= row.left(), "{part} {bounds:?} {row:?}");
        assert!(bounds.right() <= row.right(), "{part} {bounds:?} {row:?}");
    }
    let search = cx.debug_bounds("topbar-search").unwrap();
    let panels = cx.debug_bounds("topbar-panels").unwrap();
    assert!(search.right() <= panels.left(), "{search:?} {panels:?}");
}
