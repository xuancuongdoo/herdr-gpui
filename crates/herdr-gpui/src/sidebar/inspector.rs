//! Inspector panel content for the agent selected in the sidebar tree.
//!
//! Owned by lane sr-inspector (render only). Data layer
//! (`InspectorField { label, value }` + `inspector_fields(agent)` in
//! `sidebar/inspector_fields.rs`) is owned by lane sr-insp-fields and called
//! here; selection (`HerdrWindow::selected_agent_view()` +
//! `last_inspected`) is owned by sr-insp-select; the right-hand slot,
//! `INSPECTOR_WIDTH`, `INSPECTOR_MIN_WINDOW_WIDTH` and `inspector_visible`
//! are owned by sr-insp-layout. This file renders whatever
//! `Option<&ClientShellAgent>` the caller hands it, so keep-last works by
//! the caller passing the last known agent when the live snapshot no longer
//! contains it. Missing values arrive as "—" from the data layer; nothing is
//! fabricated here.

use super::inspector_fields::{InspectorField, inspector_fields};
use gpui::{prelude::*, *};
use herdr_client::protocol::ClientShellAgent;

fn row(field: &InspectorField) -> Div {
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
                .child(field.label.to_owned()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_xs()
                .child(field.value.clone()),
        )
}

/// Pure panel content: `agent` is already the selected-or-last-known agent.
///
/// Assumed caller (sr-insp-select): `HerdrWindow::selected_agent_view() ->
/// Option<ClientShellAgent>` plus `last_inspected: Option<ClientShellAgent>`.
/// Assumed mount (sr-insp-layout): right-hand slot calling
/// `inspector_panel(last_or_selected.as_ref())` inside an `INSPECTOR_WIDTH`
/// column hidden below `INSPECTOR_MIN_WINDOW_WIDTH` when `inspector_visible`.
/// Assumed data (sr-insp-fields): `inspector_fields(agent:
/// Option<&ClientShellAgent>) -> Vec<InspectorField>` with `label:
/// &'static str, value: String`; `None` yields an empty vec.
pub(crate) fn inspector_panel(agent: Option<&ClientShellAgent>) -> impl IntoElement {
    let fields = inspector_fields(agent);
    div()
        .id("inspector")
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .overflow_y_scroll()
        .child(div().text_sm().child("Inspector".to_owned()))
        .children(fields.iter().map(row))
        .when(fields.is_empty(), |panel| {
            panel.child(row(&InspectorField {
                label: "Agent",
                value: "—".to_owned(),
            }))
        })
}
