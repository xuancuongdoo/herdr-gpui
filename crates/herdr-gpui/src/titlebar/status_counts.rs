//! Mock top bar: brand, workspace breadcrumb, agent status counts, and the
//! search pill. Help, Panels, and settings arrive in a follow-up.
use crate::{controls::Command, window::HerdrWindow};
use gpui::{prelude::*, *};
use herdr_client::protocol::{AgentStatus, ClientShellAgent, ClientShellSnapshot};

const BRAND: u32 = 0x0098F8;
const IDLE: u32 = 0x00D080;
const WORKING: u32 = 0xF0D058;
const BLOCKED: u32 = 0xF02838;
const OTHER: u32 = 0x7AA2F7;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct StatusCounts {
    pub(crate) total: usize,
    pub(crate) idle: usize,
    pub(crate) working: usize,
    pub(crate) blocked: usize,
    pub(crate) other: usize,
}

impl StatusCounts {
    pub(crate) fn count(agents: &[ClientShellAgent]) -> Self {
        let (mut idle, mut working, mut blocked, mut other) = (0, 0, 0, 0);
        for agent in agents {
            match agent.agent_status {
                AgentStatus::Idle => idle += 1,
                AgentStatus::Working => working += 1,
                AgentStatus::Blocked => blocked += 1,
                AgentStatus::Done | AgentStatus::Unknown => other += 1,
            }
        }
        Self {
            total: agents.len(),
            idle,
            working,
            blocked,
            other,
        }
    }
}

pub(crate) fn active_workspace_label(snapshot: &ClientShellSnapshot) -> Option<&str> {
    snapshot
        .focused_workspace_id
        .as_deref()
        .and_then(|id| {
            snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == id)
        })
        .or_else(|| {
            snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.focused)
        })
        .or_else(|| snapshot.workspaces.first())
        .map(|workspace| workspace.label.trim())
        .filter(|label| !label.is_empty())
}

fn dot(color: u32) -> Div {
    div().size(px(8.)).flex_none().rounded_full().bg(rgb(color))
}

impl HerdrWindow {
    pub(super) fn render_topbar(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let snapshot = self.live.snapshot.clone();
        let label = snapshot
            .as_deref()
            .and_then(active_workspace_label)
            .unwrap_or("—")
            .to_owned();
        let counts = snapshot
            .as_deref()
            .map(|snapshot| StatusCounts::count(&snapshot.agents))
            .unwrap_or_default();
        div()
            .debug_selector(|| "topbar".into())
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .items_center()
            .gap(px(10.))
            .px(px(8.))
            .overflow_hidden()
            .text_size(px(12.))
            .child(
                div()
                    .debug_selector(|| "topbar-brand".into())
                    .flex_none()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(BRAND))
                    .child("HERDR"),
            )
            .child(
                div()
                    .debug_selector(|| "topbar-breadcrumb".into())
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(4.))
                    .child(div().text_color(rgb(self.theme.muted)).child("›"))
                    .child(
                        div()
                            .max_w(px(200.))
                            .truncate()
                            .text_color(rgb(self.theme.foreground))
                            .child(label),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "topbar-counts".into())
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(6.))
                    .text_color(rgb(self.theme.muted))
                    .child(format!("Agents {}", counts.total))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .child(dot(IDLE))
                            .child(format!("{}", counts.idle)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .child(dot(WORKING))
                            .child(format!("{}", counts.working)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .child(dot(BLOCKED))
                            .child(format!("{}", counts.blocked)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .child(dot(OTHER))
                            .child(format!("{}", counts.other)),
                    ),
            )
            .child(div().flex_1().min_w(px(8.)))
            .child(
                div()
                    .id("topbar-search")
                    .debug_selector(|| "topbar-search".into())
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(6.))
                    .h(px(24.))
                    .px(px(10.))
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(self.theme.active))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(self.theme.active)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|this, _, window, cx| {
                        cx.stop_propagation();
                        this.command(Command::Palette, window, cx);
                    }))
                    .text_color(rgb(self.theme.muted))
                    .child("Search…")
                    .child(
                        div()
                            .px(px(4.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(rgb(self.theme.active))
                            .child("⌘K"),
                    ),
            )
            .child(
                div()
                    .id("topbar-help")
                    .debug_selector(|| "topbar-help".into())
                    .flex()
                    .flex_none()
                    .items_center()
                    .h(px(24.))
                    .px(px(8.))
                    .rounded(px(crate::config::corners::CONTROL))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(self.theme.active)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|this, _, window, cx| {
                        cx.stop_propagation();
                        this.command(Command::Keybinds, window, cx);
                    }))
                    .text_color(rgb(self.theme.muted))
                    .child("? Help"),
            )
            .child(
                div()
                    .id("topbar-settings")
                    .debug_selector(|| "topbar-settings".into())
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(28.))
                    .rounded(px(crate::config::corners::CONTROL))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(self.theme.active)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|this, _, window, cx| {
                        cx.stop_propagation();
                        this.command(Command::Settings, window, cx);
                    }))
                    .child(
                        svg()
                            .path("icons/settings.svg")
                            .size(px(14.))
                            .text_color(rgb(self.theme.muted)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::StatusCounts;
    use crate::window::HerdrWindow;
    use gpui::{Context, Modifiers, TestAppContext, Window};
    use herdr_client::protocol::{AgentStatus, ClientShellAgent};

    #[gpui::test]
    fn search_pill_opens_the_palette(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(topbar_window);
        cx.simulate_resize(gpui::size(gpui::px(1200.), gpui::px(600.)));
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        let search = cx.debug_bounds("topbar-search").unwrap();
        cx.simulate_click(search.center(), Modifiers::default());
        view.read_with(cx, |view, _| {
            assert!(view.menu.page == Some(crate::menu::Page::Palette));
        });
    }

    #[gpui::test]
    fn help_button_opens_keybinds(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(topbar_window);
        cx.simulate_resize(gpui::size(gpui::px(1200.), gpui::px(600.)));
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        let help = cx.debug_bounds("topbar-help").unwrap();
        cx.simulate_click(help.center(), Modifiers::default());
        view.read_with(cx, |view, _| {
            assert!(view.menu.page == Some(crate::menu::Page::Keybinds));
        });
    }

    #[gpui::test]
    fn gear_button_opens_settings(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(topbar_window);
        cx.simulate_resize(gpui::size(gpui::px(1200.), gpui::px(600.)));
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        assert_eq!(cx.windows().len(), 1);
        let gear = cx.debug_bounds("topbar-settings").unwrap();
        cx.simulate_click(gear.center(), Modifiers::default());
        cx.run_until_parked();
        assert_eq!(cx.windows().len(), 2);
        view.read_with(cx, |view, _| {
            assert!(view.menu.page.is_none());
        });
    }

    fn agent(status: AgentStatus) -> ClientShellAgent {
        ClientShellAgent {
            pane_id: "p1".into(),
            workspace_id: "w1".into(),
            tab_id: "t1".into(),
            name: Some("worker".into()),
            display_agent: None,
            agent: None,
            title: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_status: status,
            state_change_seq: 0,
            state_labels: Vec::new(),
            tokens: Vec::new(),
            focused: false,
        }
    }

    #[test]
    fn counts_split_idle_working_blocked_and_other() {
        let agents = vec![
            agent(AgentStatus::Idle),
            agent(AgentStatus::Idle),
            agent(AgentStatus::Working),
            agent(AgentStatus::Blocked),
            agent(AgentStatus::Done),
            agent(AgentStatus::Unknown),
        ];
        assert_eq!(
            StatusCounts::count(&agents),
            StatusCounts {
                total: 6,
                idle: 2,
                working: 1,
                blocked: 1,
                other: 2,
            }
        );
        assert_eq!(StatusCounts::count(&[]), StatusCounts::default());
    }

    fn topbar_window(window: &mut Window, cx: &mut Context<HerdrWindow>) -> HerdrWindow {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        view.settings.shared = Some(
            crate::herdr_settings::Settings::parse_text("[ui]\ntab_bar_position = 'bottom'")
                .unwrap(),
        );
        view
    }
}
