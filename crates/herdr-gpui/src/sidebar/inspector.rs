//! Inspector panel: the mock's right column for the selected agent.
//!
//! Header (colored tile, name, role pill, status dot + pill), meta row
//! (short model alias, `—` placeholders, elapsed from client first-seen)
//! and the Overview field list (Workspace, Role, Model, Status + elapsed,
//! Started). Missing values render `—`; nothing is fabricated.
//! `render_inspector_sections` renders the Delegation Graph below the fields.

use super::agent_metrics::{
    AgentMetrics, FirstSeen, HostMetrics, MetricKind, metric_rows, metrics_for_pane,
};
use super::inspector_fields::{
    InspectorField, agent_display_name, delegation_nodes, first_seen, format_elapsed,
    format_started, inspector_fields, short_model, status_word,
};
use super::tree::model::is_orchestrator_role;
use crate::HerdrWindow;
use crate::config::Theme;
use crate::processes::Listing;
use gpui::{prelude::*, *};
use herdr_client::protocol::{AgentStatus, ClientShellAgent};
use std::collections::HashMap;
use std::time::Instant;

const ACCENT_BLUE: u32 = 0x0078E8;
const LINK_BLUE: u32 = 0x0098F8;
const IDLE_GREEN: u32 = 0x00D080;
const WORKING_YELLOW: u32 = 0xF0D058;
const BLOCKED_RED: u32 = 0xF02838;
const ORCHESTRATOR_PURPLE: u32 = 0x9850F8;
const WORKER_GREEN: u32 = 0x00C878;
const UNKNOWN_GREY: u32 = 0x8A90B8;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum InspectorTab {
    #[default]
    Overview,
    Context,
    Budget,
    Config,
    Metrics,
}
impl InspectorTab {
    pub(crate) const ALL: [Self; 5] = [
        Self::Overview,
        Self::Context,
        Self::Budget,
        Self::Config,
        Self::Metrics,
    ];
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Context => "Context",
            Self::Budget => "Budget",
            Self::Config => "Config",
            Self::Metrics => "Metrics",
        }
    }
}
fn tab_strip(
    selected: InspectorTab,
    theme: &Theme,
    cx: &mut Context<HerdrWindow>,
) -> impl IntoElement {
    div()
        .id("inspector-tabs")
        .debug_selector(|| "inspector-tabs".into())
        .flex()
        .flex_row()
        .flex_wrap()
        .gap_1()
        .children(InspectorTab::ALL.into_iter().map(|tab| {
            let label = tab.label();
            div()
                .id(("inspector-tab", tab as usize))
                .debug_selector(move || format!("inspector-tab-{label}"))
                .px_1()
                .py(px(2.))
                .rounded_md()
                .cursor_pointer()
                .text_xs()
                .text_color(rgb(if selected == tab {
                    ACCENT_BLUE
                } else {
                    theme.muted
                }))
                .when(selected == tab, |el| el.bg(rgb(theme.active)))
                .child(label.to_owned())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.inspector_tab = tab;
                    cx.notify();
                }))
        }))
}

fn status_color(status: AgentStatus) -> u32 {
    match status {
        AgentStatus::Idle | AgentStatus::Done => IDLE_GREEN,
        AgentStatus::Working => WORKING_YELLOW,
        AgentStatus::Blocked => BLOCKED_RED,
        AgentStatus::Unknown => UNKNOWN_GREY,
    }
}

fn pill(text: String, color: u32) -> Div {
    div()
        .px_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(color))
        .text_color(rgb(color))
        .text_xs()
        .child(text)
}

fn field(label: &'static str, value: impl IntoElement, muted: u32) -> Div {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .child(
            div()
                .flex_none()
                .w_20()
                .overflow_hidden()
                .text_xs()
                .text_color(rgb(muted))
                .child(label.to_owned()),
        )
        .child(div().flex_1().min_w_0().truncate().text_xs().child(value))
}

fn meta_item(text: String, muted: u32) -> Div {
    div().text_xs().text_color(rgb(muted)).child(text)
}

fn field_value<'a>(rows: &'a [InspectorField], label: &str) -> &'a str {
    rows.iter()
        .find(|row| row.label == label)
        .map(|row| row.value.as_str())
        .unwrap_or("—")
}
const METRIC_BAR_WIDTH: f32 = 96.;
fn metric_bar(fill: Option<f32>, color: u32, theme: &Theme) -> Div {
    div()
        .w(px(METRIC_BAR_WIDTH))
        .h(px(5.))
        .flex_none()
        .rounded_full()
        .overflow_hidden()
        .bg(rgb(theme.active))
        .child(
            div()
                .h_full()
                .w(px(METRIC_BAR_WIDTH * fill.unwrap_or(0.)))
                .rounded_full()
                .bg(rgb(color)),
        )
}
fn metrics_section(
    metrics: Option<&AgentMetrics>,
    host: &HostMetrics,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .id("inspector-metrics")
        .debug_selector(|| "inspector-metrics".into())
        .flex()
        .flex_col()
        .gap_1()
        .children(metric_rows(metrics, host).into_iter().map(|row| {
            let color = match row.kind {
                MetricKind::Cpu => ACCENT_BLUE,
                MetricKind::Mem => IDLE_GREEN,
                MetricKind::Elapsed => UNKNOWN_GREY,
            };
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_none()
                        .w_20()
                        .overflow_hidden()
                        .text_xs()
                        .text_color(rgb(theme.muted))
                        .child(row.label.to_owned()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .child(row.value),
                )
                .child(metric_bar(row.fill, color, theme))
        }))
}

pub(crate) fn render_inspector_sections(
    agent: &ClientShellAgent,
    agents: &[ClientShellAgent],
    theme: &Theme,
) -> impl IntoElement {
    let nodes = delegation_nodes(agent, agents);
    div()
        .id("inspector-delegation")
        .debug_selector(|| "inspector-delegation".into())
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_sm().child("Delegation Graph".to_owned()))
        .when(nodes.is_empty(), |section| {
            section.child(
                div()
                    .text_xs()
                    .text_color(rgb(theme.muted))
                    .child("—".to_owned()),
            )
        })
        .children(nodes.into_iter().map(|node| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .text_xs()
                .child(
                    div()
                        .flex_none()
                        .whitespace_nowrap()
                        .text_color(rgb(theme.muted))
                        .child(node.prefix),
                )
                .child(
                    div()
                        .flex_none()
                        .text_color(rgb(status_color(node.status)))
                        .child("●".to_owned()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .when(node.current, |name| {
                            name.font_weight(FontWeight::BOLD)
                                .text_color(rgb(LINK_BLUE))
                        })
                        .child(node.name.to_owned()),
                )
        }))
}

pub(crate) fn inspector_panel(
    theme: &Theme,
    agent: Option<&ClientShellAgent>,
    agents: &[ClientShellAgent],
    selected: InspectorTab,
    processes: Option<(&HashMap<String, Listing>, &FirstSeen)>,
    host: Option<HostMetrics>,
    cx: &mut Context<HerdrWindow>,
) -> impl IntoElement {
    let muted = theme.muted;
    let mut panel = div()
        .id("inspector")
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .overflow_y_scroll()
        .child(div().text_sm().child("Inspector".to_owned()))
        .child(tab_strip(selected, theme, cx));
    let Some(agent) = agent else {
        return panel.child(field("Agent", "—".to_owned(), muted));
    };
    let rows = inspector_fields(Some(agent));
    let role = field_value(&rows, "Role");
    let orchestrator = is_orchestrator_role(role);
    let model = short_model(field_value(&rows, "Model"));
    let (first_instant, first_wall) = first_seen(&agent.pane_id);
    let elapsed = format_elapsed(first_instant.elapsed());
    let status = agent.agent_status;
    let tile = if orchestrator {
        ORCHESTRATOR_PURPLE
    } else {
        WORKER_GREEN
    };
    let role_color = if orchestrator {
        ACCENT_BLUE
    } else {
        UNKNOWN_GREY
    };
    let dot = status_color(status);
    panel = panel
        .child(
            div()
                .id("inspector-header")
                .debug_selector(|| "inspector-header".into())
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(28.))
                        .h(px(28.))
                        .flex_none()
                        .rounded_md()
                        .bg(rgb(tile))
                        .text_sm()
                        .child("⬢".to_owned()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .child(agent_display_name(agent).to_owned()),
                )
                .child(pill(role.to_owned(), role_color))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(div().text_xs().text_color(rgb(dot)).child("●".to_owned()))
                        .child(pill(status_word(status).to_owned(), dot)),
                ),
        )
        .child(
            div()
                .id("inspector-meta")
                .debug_selector(|| "inspector-meta".into())
                .flex()
                .flex_row()
                .flex_wrap()
                .gap_x_2()
                .gap_y_1()
                .child(meta_item(model.clone(), theme.foreground))
                .child(meta_item("— tok".to_owned(), muted))
                .child(meta_item("—".to_owned(), muted))
                .child(meta_item("—".to_owned(), muted))
                .child(meta_item(elapsed.clone(), muted)),
        );
    if selected == InspectorTab::Metrics {
        let agent_metrics = processes.and_then(|(listings, seen)| {
            metrics_for_pane(&agent.pane_id, listings, seen, Instant::now())
        });
        return panel.child(metrics_section(
            agent_metrics.as_ref(),
            &host.unwrap_or_default(),
            theme,
        ));
    }
    if selected != InspectorTab::Overview {
        return panel.child(
            div()
                .id("inspector-empty")
                .debug_selector(|| "inspector-empty".into())
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(rgb(muted)).child("—".to_owned()))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(muted))
                        .child("No data".to_owned()),
                ),
        );
    }
    let workspace = field_value(&rows, "Workspace");
    panel = panel.child(
        div()
            .id("inspector-fields")
            .debug_selector(|| "inspector-fields".into())
            .flex()
            .flex_col()
            .gap_1()
            .child(field("Workspace", workspace.to_owned(), muted))
            .child(field("Role", role.to_owned(), muted))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .w_20()
                            .overflow_hidden()
                            .text_xs()
                            .text_color(rgb(muted))
                            .child("Model".to_owned()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(rgb(LINK_BLUE))
                            .child(model.clone()),
                    ),
            )
            .child(field(
                "Status",
                format!("● {} {}", status_word(status), elapsed),
                muted,
            ))
            .child(field("Started", format_started(first_wall), muted)),
    );
    panel.child(render_inspector_sections(agent, agents, theme))
}
