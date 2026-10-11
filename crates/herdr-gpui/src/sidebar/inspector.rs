//! Inspector panel: the mock's right column for the selected agent.
//!
//! Header (colored tile, name, role pill, status dot + pill), meta row
//! (short model alias, `—` placeholders, elapsed from client first-seen)
//! and the Overview field list (Workspace, Role, Model, Status + elapsed,
//! Started). Missing values render `—`; nothing is fabricated.
//! `render_inspector_sections` is the WP5 call site for the sections below
//! the fields.

use super::inspector_fields::{
    InspectorField, agent_display_name, first_seen, format_elapsed, format_started,
    inspector_fields, short_model, status_word,
};
use super::tree::model::is_orchestrator_role;
use crate::config::Theme;
use gpui::{prelude::*, *};
use herdr_client::protocol::{AgentStatus, ClientShellAgent};

const ACCENT_BLUE: u32 = 0x0078E8;
const LINK_BLUE: u32 = 0x0098F8;
const IDLE_GREEN: u32 = 0x00D080;
const WORKING_YELLOW: u32 = 0xF0D058;
const BLOCKED_RED: u32 = 0xF02838;
const ORCHESTRATOR_PURPLE: u32 = 0x9850F8;
const WORKER_GREEN: u32 = 0x00C878;
const UNKNOWN_GREY: u32 = 0x8A90B8;

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

/// WP5 call site: delegation, events and context sections plug in here.
pub(crate) fn render_inspector_sections(_agent: Option<&ClientShellAgent>) {}

/// Pure panel content: `agent` is already the selected-or-last-known agent.
pub(crate) fn inspector_panel(theme: &Theme, agent: Option<&ClientShellAgent>) -> impl IntoElement {
    let muted = theme.muted;
    let mut panel = div()
        .id("inspector")
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .overflow_y_scroll()
        .child(div().text_sm().child("Inspector".to_owned()));
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
    render_inspector_sections(Some(agent));
    panel
}
