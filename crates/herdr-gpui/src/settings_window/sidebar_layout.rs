use super::SettingsWindow;
use crate::config::{Config, FontFace, LayoutMode};
use gpui::{prelude::*, *};

impl SettingsWindow {
    pub(super) fn render_sidebar_layout_controls(&self, cx: &mut Context<Self>) -> Div {
        let ready = !self.quitting && self.closing.is_none();
        let mode = self.config.layout.mode;
        let mut layouts = div().flex().flex_col().gap(px(4.));
        for mode in LayoutMode::ALL {
            layouts = layouts.child(
                self.control_choice(
                    format!("settings-layout-{}", mode.name()),
                    mode.label(),
                    self.config.layout.mode == mode,
                    ready,
                )
                .debug_selector(move || format!("settings-layout-{}", mode.name()))
                .py(px(5.))
                .when(ready, |button| {
                    button.on_click(cx.listener(move |this, _, _, cx| {
                        this.accept_layout_choice(mode, cx);
                    }))
                }),
            );
        }
        let widths = div().flex().gap_1().children(
            crate::sidebar::preview::Preview::WIDTHS
                .into_iter()
                .map(|width| {
                    self.control_choice(
                        format!("preview-width-{width}"),
                        width.to_string(),
                        self.controls.sidebar_preview.width() == width,
                        true,
                    )
                    .debug_selector(move || format!("preview-width-{width}"))
                    .px_2()
                    .py_1()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.controls.sidebar_preview.set_width(width);
                        cx.notify();
                    }))
                }),
        );
        let mut font = self.config.sidebar.clone();
        font.size = self.controls.size(FontFace::Sidebar, &self.config);
        let light = matches!(
            cx.window_appearance(),
            WindowAppearance::Light | WindowAppearance::VibrantLight
        );
        let preview = self.controls.sidebar_preview.render(
            mode,
            &font,
            &self.theme,
            crate::sidebar::Indicators::new(self.shared.as_ref(), light, &self.theme),
            cx.listener(|this, _, _, cx| {
                this.controls.sidebar_preview.toggle_fold();
                cx.notify();
            }),
            |target| {
                Box::new(cx.listener(move |this, _, _, cx| {
                    this.controls.sidebar_preview.select(target);
                    cx.notify();
                }))
            },
        );
        let chooser = div()
            .flex()
            .flex_wrap()
            .gap_4()
            .child(
                div()
                    .w(px(174.))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(self.control_note("Choose a layout"))
                    .child(layouts)
                    .child(self.control_note("Preview width (px)"))
                    .child(widths),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(mode.label())
                    .child(preview),
            );
        self.control_card("Sidebar layout")
            .mt(px(24.))
            .relative()
            .debug_selector(|| "settings-sidebar-layout".into())
            .map(|card| {
                #[cfg(all(feature = "integration-test", target_os = "macos"))]
                let card = card.child(super::native::probe(7));
                card
            })
            .child(chooser)
            .child(
                self.control_switch(
                    "settings-show-agents",
                    "Show agents",
                    self.config.show_agents,
                    !self.busy(),
                )
                .debug_selector(|| "settings-show-agents".into())
                .when(!self.busy(), |button| {
                    button.on_click(cx.listener(|this, _, _, cx| {
                        let show = !this.config.show_agents;
                        this.save_native(move || Config::save_show_agents(show), cx);
                    }))
                }),
            )
            .child(
                self.control_switch(
                    "settings-tree-view",
                    "Tree view",
                    self.config.tree_view,
                    !self.busy(),
                )
                .debug_selector(|| "settings-tree-view".into())
                .when(!self.busy(), |button| {
                    button.on_click(cx.listener(|this, _, _, cx| {
                        let on = !this.config.tree_view;
                        this.save_native(move || Config::save_tree_view(on), cx);
                    }))
                }),
            )
            .child(self.sidebar_gap_control(cx))
    }
}
