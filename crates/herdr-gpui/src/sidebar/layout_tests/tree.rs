//! The agents panel's tree in the `spaces` sort: one line per agent, nested
//! under its space and orchestrator, and spaces that fold by click.
use super::*;
use crate::preferences::AgentSort;

/// Opens the fixture with `snapshot` in the `spaces` sort and draws it.
fn spaces_sort(
    snapshot: ClientShellSnapshot,
    cx: &mut gpui::TestAppContext,
) -> (Entity<HerdrWindow>, &mut gpui::VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
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

/// An orchestrator and its worker share a space: each is one line of guides,
/// status glyph and name only (no role/model suffix, no right status word),
/// with the worker nested deeper. No icon, place line, or second line.
#[gpui::test]
fn spaces_sort_draws_each_agent_on_one_tree_line(cx: &mut gpui::TestAppContext) {
    let mut fixture = snapshot(2);
    let tokens = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<Vec<_>>()
    };
    fixture.agents[0].tokens = tokens(&[("role", "ORCH"), ("model", "opus")]);
    fixture.agents[1].workspace_id = "w0".into();
    fixture.agents[1].tokens = tokens(&[("role", "worker"), ("model", "sonnet"), ("parent", "p0")]);
    let (_, cx) = spaces_sort(fixture, cx);

    for text in [
        "\u{25c6} Claude Code",
        "agent",
        // Both fixture agents are `working`, whose glyph is `◐` per
        // `agents::status_symbol`; `!` is `blocked`, and nothing is blocked.
        "\u{25d0}",
    ] {
        assert!(painted(cx, text), "missing {text:?}");
    }
    for text in [
        " \u{b7} orchestrator \u{b7} opus",
        " \u{b7} worker \u{b7} sonnet",
    ] {
        assert!(!painted(cx, text), "suffix removed by design: {text:?}");
    }
    // The tree's right column shows dynamic state (`status_text`, e.g.
    // `working`) by design; only the flat row's name and place must not leak.
    for text in ["Claude Code", "tab 1"] {
        assert!(!painted(cx, text), "{text:?} belongs to the flat row");
    }
    for (row, guides, name, icon, detail, status) in [
        (
            "row-agent-p0",
            "tree-agent-p0",
            "name-agent-p0",
            "agent-icon-agent-p0",
            "detail-agent-p0",
            "status-agent-p0",
        ),
        (
            "row-agent-p1",
            "tree-agent-p1",
            "name-agent-p1",
            "agent-icon-agent-p1",
            "detail-agent-p1",
            "status-agent-p1",
        ),
    ] {
        let row = cx.debug_bounds(row).unwrap();
        let guides = cx.debug_bounds(guides).unwrap();
        let name = cx.debug_bounds(name).unwrap();
        // The row chrome fixes its height (`cell::RowLayout::tree_agent`
        // wraps the line with padding), so single-line means well under two
        // text lines, not exactly one text height.
        assert!(row.size.height < name.size.height * 2., "one line");
        assert_eq!(guides.top(), name.top());
        assert!(guides.right() < name.left(), "the glyph sits between");
        for absent in [icon, detail, status] {
            assert!(cx.debug_bounds(absent).is_none(), "{absent}");
        }
    }
    let orchestrator = cx.debug_bounds("row-agent-p0").unwrap();
    let worker = cx.debug_bounds("row-agent-p1").unwrap();
    assert!(worker.top() >= orchestrator.bottom(), "worker follows");
    let orchestrator = cx.debug_bounds("tree-agent-p0").unwrap();
    let worker = cx.debug_bounds("tree-agent-p1").unwrap();
    assert!(worker.size.width > orchestrator.size.width, "worker nests");
}

/// Clicking a space row folds its agents away and keeps the row; clicking
/// again brings them back. Agents in other spaces stay put throughout.
#[gpui::test]
fn clicking_a_space_row_folds_and_restores_its_agents(cx: &mut gpui::TestAppContext) {
    let (_, cx) = spaces_sort(snapshot(2), cx);
    let header = |cx: &mut gpui::VisualTestContext, marker: char| {
        cx.update(|_, cx| {
            cx.global::<TextProbes>()
                .0
                .keys()
                .any(|text| text.ends_with(&format!("{marker} herdr")))
        })
    };
    assert!(painted(cx, "Claude Code") && painted(cx, "agent"));
    assert!(header(cx, '\u{25be}'));
    for folded in [true, false] {
        let space = cx.debug_bounds("tree-space-w0").unwrap();
        cx.simulate_click(space.center(), Modifiers::default());
        cx.run_until_parked();
        redraw(cx);
        assert_eq!(painted(cx, "Claude Code"), !folded, "folded={folded}");
        assert!(painted(cx, "agent"), "folded={folded}: other space");
        let marker = if folded { '\u{25b8}' } else { '\u{25be}' };
        assert!(header(cx, marker), "folded={folded}: space row stays");
    }
}

#[gpui::test]
fn query_down_enter_stays_on_a_visible_agent(cx: &mut gpui::TestAppContext) {
    let mut fixture = snapshot(2);
    fixture.agents[0].name = Some("hidden-agent".into());
    fixture.agents[0].display_agent = Some("hidden-agent".into());
    fixture.agents[1].name = Some("visible-agent".into());
    fixture.agents[1].display_agent = Some("visible-agent".into());
    let (view, cx) = spaces_sort(fixture, cx);
    view.update(cx, |this, cx| {
        this.sidebar_search = "visible".into();
        this.sidebar_search_open = false;
        this.sidebar_selected = 1;
        cx.notify();
    });
    redraw(cx);
    assert!(cx.debug_bounds("row-agent-p1").is_some());
    assert!(cx.debug_bounds("row-agent-p0").is_none());
    cx.update(|window, cx| {
        view.update(cx, |this, cx| {
            this.sidebar_selected = 1;
            window.focus(&this.sidebar_focus, cx);
        });
    });
    cx.run_until_parked();
    redraw(cx);
    cx.simulate_keystrokes("down enter");
    cx.update(|window, cx| {
        let this = view.read(cx);
        assert_eq!(this.sidebar_selected, 0);
        assert_eq!(this.sidebar_visible_agents()[this.sidebar_selected].1, "p1");
        assert!(this.focus.is_focused(window));
    });
}
