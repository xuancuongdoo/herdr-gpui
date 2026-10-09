//! Hidden parked output must not redraw the window; visible parked
//! output must. See `HerdrWindow::poll_group_terminals`.
#![allow(clippy::unwrap_used)]
use super::tests::{fixture_window_on_t0, surface};
use super::*;
use crate::{controls::Command, sidebar::layout_tests::full_draw, sidebar::layout_tests::snapshot};
use core::prelude::v1::test;
use gpui::TestAppContext;
use herdr_client::protocol::ClientShellSnapshot;
use std::sync::Arc;

/// A parked connection with a dirty inbox, as a streaming tab leaves one.
fn dirty_parked(
    group: GroupId,
    workspace: (Scope, String),
    snapshot: ClientShellSnapshot,
) -> Parked {
    let target = herdr_client::ConnectTarget::Socket("/unused-redraw-test.sock".into());
    let mut live = LiveState::default();
    live.status = ConnectionStatus::Connected;
    live.surface = Some(surface(&snapshot, "p1"));
    live.snapshot = Some(Arc::new(snapshot));
    let parked = Parked {
        group,
        workspace,
        connection: ConnectionBridge::new(target),
        live,
        presentation: Presentation::default(),
        options: ConnectOptions::default(),
        last_queued_options: Some(ConnectOptions::default()),
        pending_resize: None,
        bounds: Bounds::default(),
        unfocused: false,
        asked: None,
    };
    if let Ok(mut inbox) = parked.connection.inbox.lock() {
        *inbox = parked.live.clone();
        inbox.dirty = true;
    }
    parked
}

#[gpui::test]
fn no_parked_updates_need_no_redraw(cx: &mut TestAppContext) {
    let (view, cx) = fixture_window_on_t0(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            assert!(!view.poll_group_terminals(cx));
        })
    });
}

#[gpui::test]
fn another_workspace_output_needs_no_redraw(cx: &mut TestAppContext) {
    let (view, cx) = fixture_window_on_t0(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let mut ids = crate::browser::GroupIds::default();
            let mut hidden = snapshot(40);
            hidden.focused_workspace_id = Some("w9".into());
            hidden.focused_tab_id = Some("t9".into());
            view.browser.terminals.parked.push(dirty_parked(
                ids.next(),
                (Scope::endpoint("local"), "w9".into()),
                hidden,
            ));
            assert!(!view.poll_group_terminals(cx));
            // Drained anyway, so the next frame does not tick on it again.
            assert!(!view.browser.terminals.parked[0].connection.has_update());
        })
    });
}

#[gpui::test]
fn unshown_tab_output_needs_no_redraw(cx: &mut TestAppContext) {
    let (view, cx) = fixture_window_on_t0(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let mut ids = crate::browser::GroupIds::default();
            // Same workspace, but no group shows t9.
            let mut hidden = snapshot(40);
            hidden.focused_workspace_id = Some("w0".into());
            hidden.focused_tab_id = Some("t9".into());
            view.browser.terminals.parked.push(dirty_parked(
                ids.next(),
                (Scope::endpoint("local"), "w0".into()),
                hidden,
            ));
            assert!(!view.poll_group_terminals(cx));
            assert!(!view.browser.terminals.parked[0].connection.has_update());
        })
    });
}

#[gpui::test]
fn visible_parked_output_needs_a_redraw(cx: &mut TestAppContext) {
    let (view, cx) = fixture_window_on_t0(cx);
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.command(Command::SplitEditor, window, cx)
        })
    });
    let [left, right] = view.read_with(cx, |view, _| {
        let slots = view.group_slots();
        [slots[0].id, slots[1].id]
    });
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let layout = view.ensure_layout().unwrap();
            layout.choose(left, Pick::Herdr("t1".into()));
            layout.activate(right);
            let mut on_t1 = snapshot(40);
            on_t1.focused_workspace_id = Some("w0".into());
            on_t1.focused_tab_id = Some("t1".into());
            let key = view.browser_key().unwrap();
            view.browser
                .terminals
                .parked
                .push(dirty_parked(left, key, on_t1));
            assert!(view.poll_group_terminals(cx));
        })
    });
}
