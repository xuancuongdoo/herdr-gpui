//! Sidebar root element and section headers: the focus-tracked shell with
//! its key/action bindings and frame styling, plus the `spaces`/`agents`
//! section header. Lists are built in `render`; this module only frames them.

use super::{label_text, layout::SidebarLook, line_height, sidebar_width};
use crate::{
    HerdrWindow,
    config::{FontConfig, Theme},
    fonts::StyledFont,
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(crate) fn sidebar_shell(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let width = sidebar_width(self.sidebar_width, f32::from(window.viewport_size().width));
        let font = &self.config.sidebar;
        let theme = &self.theme;
        div()
            .id("sidebar")
            .debug_selector(|| "sidebar".into())
            .key_context(crate::sidebar::keys::KEY_CONTEXT)
            .track_focus(&self.sidebar_focus)
            .on_action(
                cx.listener(|this, _: &crate::sidebar::keys::SidebarUp, _, cx| {
                    this.move_sidebar_selection(-1, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::sidebar::keys::SidebarDown, _, cx| {
                    this.move_sidebar_selection(1, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::sidebar::keys::SidebarOpen, window, cx| {
                    this.open_selected_agent(window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::sidebar::keys::SidebarToggle, _, cx| {
                    this.toggle_selected_agent_group(cx)
                }),
            )
            .on_action(cx.listener(
                |this, _: &crate::sidebar::keys::SidebarSearch, window, cx| {
                    this.focus_sidebar_search(window, cx)
                },
            ))
            .on_action(cx.listener(
                |this, _: &crate::sidebar::keys::SidebarCycleFilter, _, cx| {
                    this.cycle_sidebar_filter(cx)
                },
            ))
            .on_action(cx.listener(
                |this, _: &crate::sidebar::keys::SidebarFocusNext, window, cx| {
                    this.focus_next_sidebar(window, cx)
                },
            ))
            .on_action(cx.listener(
                |this, _: &crate::sidebar::keys::SidebarClearSearch, window, cx| {
                    this.clear_sidebar_search(window, cx)
                },
            ))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                // Escape reaches here instead of the terminal while sidebar
                // focus holds it (track_focus auto-focus on mouse-down), and
                // the HerdrSidebar binding never fires in this path, so cancel
                // drags here, mirroring the terminal's key_down.
                if event.keystroke.key == "escape"
                    && (this.cancel_workspace_drag(cx) | this.cancel_tab_drag(cx))
                {
                    cx.stop_propagation();
                    window.prevent_default();
                    return;
                }
                if !this.sidebar_search_open {
                    return;
                }
                if event.keystroke.modifiers.platform
                    || event.keystroke.modifiers.control
                    || event.keystroke.modifiers.alt
                    || event.keystroke.modifiers.function
                {
                    return;
                }
                match event.keystroke.key.as_str() {
                    // Enter/Escape keep their actions: Enter closes keeping the
                    // query, Escape clears and closes.
                    "enter" | "escape" | "tab" | "up" | "down" | "left" | "right" => {}
                    "backspace" => {
                        this.sidebar_search.pop();
                        this.clamp_sidebar_selection();
                        cx.notify();
                        cx.stop_propagation();
                        window.prevent_default();
                    }
                    "space" => {
                        this.push_sidebar_search_text(" ", cx);
                        cx.stop_propagation();
                        window.prevent_default();
                    }
                    key if key.len() == 1 => {
                        let text = event.keystroke.key_char.as_deref().unwrap_or(key);
                        this.push_sidebar_search_text(text, cx);
                        cx.stop_propagation();
                        window.prevent_default();
                    }
                    _ => {
                        // Committed text (IME "日本", dead keys) arrives whole:
                        // accept it as-is instead of single characters only,
                        // mirroring how the find bar leaves composition to the
                        // platform instead of per-keystroke appends.
                        if let Some(text) = event.keystroke.key_char.as_deref()
                            && !text.is_empty()
                        {
                            this.push_sidebar_search_text(text, cx);
                            cx.stop_propagation();
                            window.prevent_default();
                        }
                    }
                }
            }))
            .relative()
            .w(px(width))
            .flex_none()
            .h_full()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .text_font(font)
            .text_size(px(font.size))
            .line_height(px(line_height(font)))
            .text_color(rgb(theme.foreground))
            .bg(rgb(theme.sidebar_background()))
            .border_r_1()
            .border_color(rgb(theme.active))
    }
}

pub(super) fn header(
    label: &'static str,
    font: &FontConfig,
    theme: &Theme,
    look: SidebarLook,
) -> Div {
    div()
        .debug_selector(|| format!("header-{label}"))
        .flex_none()
        .h(px(line_height(font) + 2. * look.density.header_padding()))
        .px(px(look.content_x()))
        .flex()
        .items_center()
        .text_size(px(font.size))
        .text_color(rgb(theme.muted))
        .child(
            div()
                .debug_selector(|| format!("header-label-{label}"))
                .child(label_text(&look.header_label(label))),
        )
}
