//! Inspector layout slot (lane sr-insp-layout).
//!
//! Owns ONLY the right-hand column geometry and the mount point.
//! Lane sr-inspector owns the panel contents: replace the body of
//! [`HerdrWindow::inspector_panel`] without changing its signature.
use crate::window::HerdrWindow;
use gpui::{prelude::*, *};
use herdr_client::protocol::ClientShellAgent;
/// Floor of the inspector width: the mount takes ~31% of the window,
/// clamped to `INSPECTOR_WIDTH`–520 px.
pub(crate) const INSPECTOR_WIDTH: f32 = 280.;

/// Right-column width for a window: ~31% of the window clamped to
/// 280–520 px, never eating the terminal reserve.
pub(crate) fn inspector_width(window_width: f32) -> f32 {
    (window_width * 0.31)
        .clamp(INSPECTOR_WIDTH, 520.)
        .min((window_width - 240.).max(0.))
}
/// Window narrower than this hides the inspector first (sidebar pattern).
pub(crate) const INSPECTOR_MIN_WINDOW_WIDTH: f32 = 960.;
impl HerdrWindow {
    /// Right-hand slot for the inspector. Returns `None` when the window is
    /// too narrow, so the call site stays one call:
    /// `.children(self.inspector_panel(window, cx))`.
    ///
    /// sr-inspector plugs contents here by editing only this body.
    pub(crate) fn inspector_panel(
        &self,
        window: &Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let window_width = f32::from(window.viewport_size().width);
        if window_width < INSPECTOR_MIN_WINDOW_WIDTH {
            return None;
        }
        // Same terminal reserve the sidebar keeps, so a mid-size window never
        // loses its terminal to the inspector.
        let width = inspector_width(window_width);
        if width <= 0. {
            return None;
        }
        Some(
            div()
                .id("inspector-panel")
                .debug_selector(|| "inspector-panel".into())
                .flex()
                .flex_col()
                .flex_none()
                .w(px(width))
                .min_h_0()
                .bg(rgb(self.theme.sidebar_background()))
                .border_l_1()
                .border_color(rgb(self.theme.active))
                .track_focus(&self.inspector_focus)
                .child(crate::sidebar::inspector::inspector_panel(
                    &self.theme,
                    self.selected_agent_view()
                        .or(self.last_inspected.clone())
                        .as_ref(),
                ))
                .into_any_element(),
        )
    }
    pub(crate) fn selected_agent_view(&self) -> Option<ClientShellAgent> {
        self.live
            .snapshot
            .as_deref()?
            .agents
            .iter()
            .find(|agent| agent.focused)
            .cloned()
    }
    pub(crate) fn remember_inspected(&mut self) {
        if let Some(agent) = self.selected_agent_view() {
            self.last_inspected = Some(agent);
        }
    }
}
