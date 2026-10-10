//! Tree row elements for the sidebar `spaces` sort: device → space → agent.
//!
//! The call site lives in `super::render`; this module only draws rows.

use super::super::{
    agents::{Indicators, agent_name, agent_place, state_label, status_symbol, status_text},
    cell::{AgentRow, Cell, RowContext, RowData, RowLayout},
    label_text,
    layout::SidebarLook,
    metrics::{glyph_width, line_height},
    tokens::{ResolvedToken, agent_rows},
};
use super::legacy::build_agent_tree_with_status;
use super::model::{WorkerGroup, group_key_string};
use super::{TreeNode, TreeRow};
use crate::{
    HerdrWindow, NavigationTarget,
    config::{AgentLayout, FontConfig, Theme},
};
use gpui::{prelude::*, *};
use herdr_client::protocol::{AgentStatus, ClientShellAgent, ClientShellSnapshot};
use std::collections::{HashMap, HashSet};
use unicode_width::UnicodeWidthStr;

use super::super::keys::{StatusFilter, matches_agent};
type FilterAgent = (String, String, String, Vec<(String, String)>, AgentStatus);
/// Guide prefix for a row: `│  `/blank per ancestor plus `├─ `/`└─ ` for
/// itself. Fold parents keep only the ancestor guides so the vertical `│`
/// stays continuous through rows like `│  ▾ REV-1354`.
pub(crate) fn guides(row: &TreeRow) -> String {
    let mut out = String::new();
    for last in &row.ancestors_last {
        out.push_str(if *last { "   " } else { "\u{2502}  " });
    }
    if row.depth > 0 && !row.has_children && matches!(row.node, TreeNode::Agent { .. }) {
        out.push_str(if row.last_child {
            "\u{2514}\u{2500} "
        } else {
            "\u{251c}\u{2500} "
        });
    }
    out
}

/// Fold marker for a parent row; empty when it has no children to fold.
pub(crate) fn fold_marker(has_children: bool, collapsed: bool) -> &'static str {
    if !has_children {
        ""
    } else if collapsed {
        "\u{25b8}"
    } else {
        "\u{25be}"
    }
}

/// Ancestor guides only (`│  `/blank per ancestor, no connector): the text
/// of a blank spacer row, so the verticals stay continuous through it.
fn spacer_guides(next: &TreeRow) -> String {
    next.ancestors_last
        .iter()
        .take(usize::from(next.depth))
        .map(|last| if *last { "   " } else { "\u{2502}  " })
        .collect()
}

/// Blank guide-only row between two consecutive visible rows; `None` keeps
/// them tight. Spacers separate a device's children and wrap an expanded
/// workspace's children block (including job blocks: a job header stays
/// tight with its first worker, as workers stay tight with each other).
/// Collapsed rows stay tight with what follows, and the caller emits nothing
/// before the first row (no spacer at the panel top).
fn spacer_between(prev: &TreeRow, next: &TreeRow) -> Option<String> {
    if prev.collapsed {
        return None;
    }
    let separated = match (&prev.node, &next.node) {
        (TreeNode::Device { .. }, _) | (_, TreeNode::Device { .. }) => true,
        (TreeNode::Space { .. }, _) | (_, TreeNode::Space { .. }) => true,
        (TreeNode::Agent { .. }, TreeNode::Job { .. }) => true,
        (TreeNode::Job { .. }, TreeNode::Job { .. }) => true,
        (TreeNode::Agent { .. }, TreeNode::Group { .. }) => true,
        (TreeNode::Job { .. }, TreeNode::Group { .. }) => true,
        (TreeNode::Group { .. }, TreeNode::Job { .. }) => true,
        (TreeNode::Group { .. }, TreeNode::Group { .. }) => true,
        (TreeNode::Agent { .. }, TreeNode::Agent { .. }) => next.depth < prev.depth,
        _ => false,
    };
    separated.then(|| spacer_guides(next))
}

/// Everything the grouped tree render needs from the endpoint loop.
pub(in crate::sidebar) struct GroupedCtx<'a> {
    pub snapshot: &'a ClientShellSnapshot,
    pub endpoint_id: &'a str,
    pub endpoint_label: &'a str,
    pub selected: bool,
    pub multi: bool,
    pub connected: bool,
    pub content_x: f32,
    pub agents_custom: bool,
    pub agent_layout: &'a AgentLayout,
    pub rows: &'a dyn RowLayout,
    pub indicators: Indicators,
    pub font: &'a FontConfig,
    pub theme: &'a Theme,
    pub look: SidebarLook,
    pub width: f32,
    pub host: Option<&'a str>,
    pub nest: f32,
    pub mark: super::super::wash::HostMark,
    pub collapsed: &'a HashSet<String>,
    pub search: &'a str,
    pub status_filter: StatusFilter,
}

pub(crate) struct GroupedOutcome {
    pub agents: Stateful<Div>,
    pub list_rows: usize,
    pub agent_count: usize,
    pub focused: Option<usize>,
}

/// The value of the agent token `key`, when present and not blank.
fn agent_token<'a>(agent: &'a ClientShellAgent, key: &str) -> Option<&'a str> {
    agent
        .tokens
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
}

/// A tree agent's text: the bare name with a `◆ ` prefix for an orchestrator.
/// No role/model/status text: the name gets the full row width; status is the
/// glyph prefix with colour as a secondary cue. The orchestrator rule is the
/// tree's shared explicit-role predicate, normalized to `orchestrator`.
/// Orchestrator vs worker reads from indentation plus `◆`, not from text.
pub(crate) fn agent_line(
    name: &str,
    role: Option<&str>,
    model: Option<&str>,
) -> (String, Option<String>) {
    let _ = model;
    let role = role.map(str::trim).filter(|role| !role.is_empty());
    let orchestrator = role.is_some_and(super::model::is_orchestrator_role);
    if orchestrator {
        (format!("\u{25c6} {name}"), None)
    } else {
        (name.to_owned(), None)
    }
}

/// A job header's text: fold marker + label on the left (`▾ REV-1354`), worker
/// count on the right (`1 worker` / `N workers`). Guides match a space row.
pub(crate) fn job_line(
    label: &str,
    total: usize,
    has_children: bool,
    collapsed: bool,
    guides: &str,
) -> (String, String) {
    let marker = fold_marker(has_children, collapsed);
    let right = if total == 1 {
        "1 worker".to_owned()
    } else {
        format!("{total} workers")
    };
    (format!("{guides}{marker} {label}"), right)
}

/// Element id for a job header row: endpoint plus orchestrator pane, never
/// the label, so two same-label tickets on different panes or endpoints
/// never share one GPUI element id (folding one must not fold the other).
pub(crate) fn job_element_id(endpoint_id: &str, orchestrator: &str) -> String {
    format!("tree-job-{endpoint_id}-{orchestrator}")
}

/// Element id for a status-group header row: endpoint plus workspace,
/// orchestrator pane and kind, never the label, so same-kind groups under
/// different orchestrators never share one GPUI element id.
pub(crate) fn group_element_id(
    endpoint_id: &str,
    workspace_id: &str,
    orchestrator: &str,
    kind: WorkerGroup,
) -> String {
    format!(
        "tree-group-{endpoint_id}-{workspace_id}-{orchestrator}-{}",
        kind.key_suffix()
    )
}

/// Width budget for one tree agent row, split from the element builder so the
/// 160px truncation guard is unit-testable without a window.
///
/// Guides are fixed chrome: the full indent always paints, even when the
/// label has to shrink to nothing. Only the name/suffix pool and the
/// right-column state word absorb a shortage; the state word is dropped
/// entirely when it cannot fit in full with a one-glyph gap before it,
/// because the status glyph already carries it. The gap lives inside the
/// right width so the boxes never touch even when the suffix fills its own
/// box exactly.
pub(crate) fn tree_agent_widths(
    content: f32,
    prefix_width: f32,
    glyph: &str,
    name: &str,
    right: &str,
    glyph_w: f32,
) -> (f32, f32, f32, f32, f32) {
    // Never capped, never reclaimed: squeezing the guides painted `…` over
    // them and shifted rows left past their depth at 160px.
    let guides_w = prefix_width.clamp(0., content);
    let glyph_need = (glyph.width().max(1) + 1) as f32 * glyph_w;
    let glyph_col = glyph_need.min((content - guides_w).max(0.));
    let pool = (content - guides_w - glyph_col).max(0.);
    let name_need = (name.width() as f32 * glyph_w).ceil();
    let right_need = (right.width() as f32 * glyph_w).ceil();
    // One glyph of breathing room before the state word, kept inside the
    // right width so the total still fits `content` exactly.
    let gap = glyph_w;
    if name_need <= pool {
        let name_w = name_need;
        if right_need > 0. && right_need + gap <= pool - name_w {
            let right_w = right_need + gap;
            (
                guides_w,
                glyph_col,
                name_w,
                pool - name_w - right_w,
                right_w,
            )
        } else {
            (guides_w, glyph_col, name_w, pool - name_w, 0.)
        }
    } else {
        (guides_w, glyph_col, pool, 0., 0.)
    }
}

/// Whether a job header keeps its right-side worker count: only when guides,
/// the full ticket label, and the count all fit with a one-glyph gap, so a
/// narrow column drops the count (the fold marker keeps its meaning) instead
/// of squeezing the label to nothing. Mirrors the agent-row state-word rule.
pub(crate) fn job_count_fits(
    content: f32,
    guide_text: &str,
    label: &str,
    right: &str,
    glyph_w: f32,
) -> bool {
    let need =
        ((guide_text.width() + label.width() + right.width()) as f32 * glyph_w).ceil() + glyph_w;
    need <= content
}

/// One tree agent row: the tree guides, then upstream's own agent cell for
/// this layout, so status words, icons, dots, configured rows, and the host
/// wash all match the flat list by construction. Only the guides gutter and
/// the diamond name are tree-only.
fn tree_agent_row(
    agent: &ClientShellAgent,
    ctx: &GroupedCtx<'_>,
    row_cx: &RowContext<'_>,
    gap: f32,
    guides: &str,
    lines: Vec<Vec<ResolvedToken>>,
    cx: &mut Context<HerdrWindow>,
) -> Stateful<Div> {
    let font = row_cx.font;
    let id = agent.pane_id.clone();
    let navigate_endpoint = ctx.endpoint_id.to_owned();
    let key = format!("agent-{id}");
    let (name, _) = agent_line(
        agent_name(agent),
        agent_token(agent, "role"),
        agent_token(agent, "model"),
    );
    let content = row_cx.look.content_width(row_cx.width).max(0.);
    let glyph_w = glyph_width(font);
    let (guides_w, _, _, _, _) = tree_agent_widths(
        content,
        guides.width() as f32 * glyph_w,
        status_symbol(agent.agent_status),
        &name,
        "",
        glyph_w,
    );
    let status_floor =
        (status_text(agent.agent_status).width() as f32 + 1.) * glyph_w + row_cx.look.content_x();
    let guides_w = guides_w.min((content - status_floor).max(0.));
    let inner_cx = RowContext {
        indicators: row_cx.indicators,
        font,
        theme: row_cx.theme,
        look: row_cx.look,
        width: (row_cx.width - guides_w).max(0.),
        nest: row_cx.nest,
        mark: row_cx.mark,
        host: row_cx.host,
    };
    let inner = Cell::new(
        ctx.rows,
        RowData::Agent(AgentRow {
            key,
            name: &name,
            icon: crate::icons::AgentIcon::from_identity(agent.agent.as_deref()),
            status: agent.agent_status,
            place: agent_place(agent, ctx.snapshot),
            status_text: ctx
                .agent_layout
                .shows_status_text(agent.agent.as_deref())
                .then(|| state_label(agent, status_text(agent.agent_status))),
            lines,
        }),
        &inner_cx,
    )
    .selected(ctx.selected && agent.focused)
    .row();
    div()
        .w_full()
        .flex_none()
        .flex()
        .flex_row()
        .child(
            div()
                .w(px(guides_w))
                .flex_none()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_color(rgb(row_cx.theme.muted))
                .child(label_text(guides)),
        )
        .child(div().flex_1().min_w_0().child(inner))
        .when(gap > 0., |row| row.mt(px(gap)))
        .id(SharedString::from(format!(
            "tree-agent-{}-{}",
            ctx.endpoint_id, agent.pane_id
        )))
        .when(ctx.multi, |row| {
            row.debug_selector(|| format!("agent-{}-{}", ctx.endpoint_id, agent.pane_id))
        })
        .on_click(cx.listener(move |this, _, window, cx| {
            this.navigate_endpoint(&navigate_endpoint, NavigationTarget::Pane(&id), cx);
            window.focus(&this.focus, cx);
        }))
}

/// Agents visible under the search query + status filter, computed before
/// grouping: matches stay, plus their ancestor chain (same workspace, pane ID
/// first then unambiguous name — mirroring `parent_of`) so an orchestrator
/// stays when any of its workers matches. Group/job totals below therefore
/// follow the filter.
pub(in crate::sidebar) fn filter_visible(
    agents: &[FilterAgent],
    search: &str,
    filter: StatusFilter,
) -> Vec<FilterAgent> {
    let mut keep: Vec<bool> = agents
        .iter()
        .map(|(_, _, name, _, status)| matches_agent(name, *status, search, filter))
        .collect();
    // Close over ancestors so a matching worker keeps its orchestrator.
    let mut changed = true;
    while changed {
        changed = false;
        for (i, (workspace, _, _, tokens, _)) in agents.iter().enumerate() {
            if !keep[i] {
                continue;
            }
            let parent = tokens
                .iter()
                .find(|(key, _)| key == "parent")
                .map(|(_, value)| value.trim())
                .filter(|value| !value.is_empty());
            let Some(parent) = parent else {
                continue;
            };
            let target = agents
                .iter()
                .enumerate()
                .position(|(j, a)| j != i && a.0 == *workspace && a.1 == parent)
                .or_else(|| {
                    let mut named = agents
                        .iter()
                        .enumerate()
                        .filter(|(j, a)| *j != i && a.0 == *workspace && a.2 == parent);
                    match (named.next(), named.next()) {
                        (Some((j, _)), None) => Some(j),
                        _ => None,
                    }
                });
            if let Some(j) = target
                && !keep[j]
            {
                keep[j] = true;
                changed = true;
            }
        }
    }
    agents
        .iter()
        .zip(keep)
        .filter(|(_, kept)| *kept)
        .map(|(agent, _)| agent.clone())
        .collect()
}
pub(in crate::sidebar) fn visible_agent_panes(
    snapshot: &ClientShellSnapshot,
    endpoint_label: &str,
    search: &str,
    status_filter: StatusFilter,
    collapsed: &HashSet<String>,
) -> Vec<String> {
    let devices = vec![(
        endpoint_label.to_owned(),
        snapshot
            .workspaces
            .iter()
            .map(|w| (w.workspace_id.clone(), w.label.clone()))
            .collect(),
    )];
    let input: Vec<_> = snapshot
        .agents
        .iter()
        .map(|agent| {
            (
                agent.workspace_id.clone(),
                agent.pane_id.clone(),
                agent_name(agent).to_owned(),
                agent.tokens.clone(),
                agent.agent_status,
            )
        })
        .collect();
    let visible = filter_visible(&input, search, status_filter);
    build_agent_tree_with_status(&devices, &visible, collapsed)
        .into_iter()
        .filter_map(|row| match row.node {
            TreeNode::Agent { pane_id, .. } => Some(pane_id),
            _ => None,
        })
        .collect()
}

/// Renders the device → space → agent rows for one endpoint snapshot.
/// Fold state comes from `GroupedCtx::collapsed` (`dev:`/`ws:`/`job:` keys
/// collapse; `grp:` keys expand, as status groups collapse by default).
pub(in crate::sidebar) fn render_grouped(
    agents: Stateful<Div>,
    ctx: &GroupedCtx<'_>,
    mut list_rows: usize,
    mut agent_count: usize,
    cx: &mut Context<HerdrWindow>,
) -> GroupedOutcome {
    let devices = vec![(
        ctx.endpoint_label.to_owned(),
        ctx.snapshot
            .workspaces
            .iter()
            .map(|workspace| (workspace.workspace_id.clone(), workspace.label.clone()))
            .collect::<Vec<_>>(),
    )];
    let input = filter_visible(
        &ctx.snapshot
            .agents
            .iter()
            .map(|agent| {
                (
                    agent.workspace_id.clone(),
                    agent.pane_id.clone(),
                    agent_name(agent).to_owned(),
                    agent.tokens.clone(),
                    agent.agent_status,
                )
            })
            .collect::<Vec<_>>(),
        ctx.search,
        ctx.status_filter,
    );
    let tree_rows = build_agent_tree_with_status(&devices, &input, ctx.collapsed);
    let by_pane: HashMap<&str, &ClientShellAgent> = ctx
        .snapshot
        .agents
        .iter()
        .map(|agent| (agent.pane_id.as_str(), agent))
        .collect();
    // Resolve token lines once: they feed both the space-header gate below
    // and the agent cells, exactly as the flat loop resolves them.
    let mut lines: HashMap<&str, Vec<Vec<ResolvedToken>>> = HashMap::new();
    for agent in ctx.snapshot.agents.iter() {
        let resolved = if ctx.agents_custom {
            agent_rows(ctx.agent_layout, agent, ctx.snapshot, ctx.host)
        } else {
            Some(Vec::new())
        };
        if let Some(resolved) = resolved {
            lines.insert(agent.pane_id.as_str(), resolved);
        }
    }
    let mut agents = agents;
    let mut focused = None;
    // Last visible row: hidden rows `continue` before reaching it, so gaps
    // only separate rows the panel actually shows.
    let mut prev: Option<TreeRow> = None;
    for row in &tree_rows {
        if let Some(text) = prev.as_ref().and_then(|last| spacer_between(last, row)) {
            // Guide-only gap: no id, no click, and no `list_rows` increment,
            // so keyboard highlight skips it.
            agents = agents.child(
                div()
                    .flex_none()
                    .w_full()
                    .px(px(ctx.content_x))
                    .child(label_text(&text)),
            );
        }
        match &row.node {
            TreeNode::Device { label } => {
                let marker = fold_marker(row.has_children, row.collapsed);
                let key = format!("dev:{label}");
                agents = agents.child(
                    div()
                        .debug_selector(|| format!("tree-device-{}", ctx.endpoint_id))
                        .flex_none()
                        .w_full()
                        .px(px(ctx.content_x))
                        .pt(px(ctx.look.density.header_padding()))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .font_weight(FontWeight::BOLD)
                        .cursor_pointer()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(label_text(&format!("{}{marker} {label}", guides(row)))),
                        )
                        .when(ctx.connected, |row| {
                            row.child(
                                div()
                                    .flex_none()
                                    .pl(px(8.))
                                    .truncate()
                                    .text_color(rgb(ctx.theme.muted))
                                    .child(label_text("\u{25cf} online")),
                            )
                        })
                        .id(SharedString::from(format!(
                            "tree-device-{}",
                            ctx.endpoint_id
                        )))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if !this.tree_collapsed.remove(&key) {
                                this.tree_collapsed.insert(key.clone());
                            }
                            cx.notify();
                        })),
                );
                list_rows += 1;
            }
            TreeNode::Space {
                workspace_id,
                label,
            } => {
                // Like the old pending headers, a space row stays hidden while
                // custom rows resolve to the icon-only fallback; its agents
                // still render below. Both the gate and the `N agents` count
                // use the filtered input so they follow search/status filter.
                let shown = !ctx.agents_custom
                    || input.iter().any(|(ws, pane_id, _, _, _)| {
                        ws == workspace_id
                            && lines.get(pane_id.as_str()).is_some_and(|resolved| {
                                resolved
                                    .iter()
                                    .flatten()
                                    .any(|token| token.kind.text().is_some())
                            })
                    });
                if !shown {
                    continue;
                }
                let marker = fold_marker(row.has_children, row.collapsed);
                let key = format!("ws:{}:{workspace_id}", ctx.endpoint_label);
                let count = input
                    .iter()
                    .filter(|(ws, _, _, _, _)| ws == workspace_id)
                    .count();
                let right = if count == 1 {
                    "1 agent".to_owned()
                } else {
                    format!("{count} agents")
                };
                agents = agents.child(
                    div()
                        .debug_selector(|| format!("tree-space-{workspace_id}"))
                        .flex_none()
                        .w_full()
                        .px(px(ctx.content_x))
                        .pt(px(ctx.look.density.header_padding()))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .font_weight(FontWeight::BOLD)
                        .cursor_pointer()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(label_text(&format!("{}{marker} {label}", guides(row)))),
                        )
                        .child(
                            div()
                                .flex_none()
                                .pl(px(8.))
                                .truncate()
                                .text_color(rgb(ctx.theme.muted))
                                .child(label_text(&right)),
                        )
                        .id(SharedString::from(format!(
                            "tree-space-{}-{workspace_id}",
                            ctx.endpoint_id
                        )))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if !this.tree_collapsed.remove(&key) {
                                this.tree_collapsed.insert(key.clone());
                            }
                            cx.notify();
                        })),
                );
                list_rows += 1;
            }
            TreeNode::Job {
                label,
                total,
                orchestrator,
            } => {
                let key = format!("job:{}:{orchestrator}", ctx.endpoint_label);
                let marker = fold_marker(row.has_children, row.collapsed);
                // Fixed chrome: guides plus fold marker never truncate; only
                // the ticket label ellipsizes at the sidebar edge.
                let guide_text = format!("{}{marker} ", guides(row));
                let (_, right) =
                    job_line(label, *total, row.has_children, row.collapsed, &guides(row));
                // Narrow columns drop the worker count (the marker keeps its
                // meaning) so the label keeps visible characters; at 500px
                // everything fits and the count stays.
                let content = ctx.look.content_width(ctx.width).max(0.);
                let show_right =
                    job_count_fits(content, &guide_text, label, &right, glyph_width(ctx.font));
                agents = agents.child(
                    div()
                        .flex_none()
                        .w_full()
                        .px(px(ctx.content_x))
                        .pt(px(ctx.look.density.header_padding()))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .font_weight(FontWeight::BOLD)
                        .cursor_pointer()
                        .child(div().flex_none().child(label_text(&guide_text)))
                        .child(div().flex_1().min_w_0().truncate().child(label_text(label)))
                        .when(show_right, |row| {
                            row.child(
                                div()
                                    .flex_none()
                                    .pl(px(8.))
                                    .truncate()
                                    .text_color(rgb(ctx.theme.muted))
                                    .child(label_text(&right)),
                            )
                        })
                        .id(SharedString::from(job_element_id(
                            ctx.endpoint_id,
                            orchestrator,
                        )))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if !this.tree_collapsed.remove(&key) {
                                this.tree_collapsed.insert(key.clone());
                            }
                            cx.notify();
                        })),
                );
                list_rows += 1;
            }
            TreeNode::Group {
                workspace_id,
                orchestrator,
                kind,
                total,
            } => {
                let key = group_key_string(workspace_id, orchestrator, *kind);
                let label = format!("{} ({})", kind.label(), total);
                let marker = fold_marker(row.has_children, row.collapsed);
                // Fixed chrome: guides plus fold marker never truncate; only
                // the group label ellipsizes at the sidebar edge.
                let guide_text = format!("{}{marker} ", guides(row));
                agents = agents.child(
                    div()
                        .flex_none()
                        .w_full()
                        .px(px(ctx.content_x))
                        .pt(px(ctx.look.density.header_padding()))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .font_weight(FontWeight::BOLD)
                        .cursor_pointer()
                        .child(div().flex_none().child(label_text(&guide_text)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(label_text(&label)),
                        )
                        .id(SharedString::from(group_element_id(
                            ctx.endpoint_id,
                            workspace_id,
                            orchestrator,
                            *kind,
                        )))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_group(key.clone(), cx);
                        })),
                );
                list_rows += 1;
            }
            TreeNode::Agent { pane_id, .. } => {
                let Some(agent) = by_pane.get(pane_id.as_str()).copied() else {
                    continue;
                };
                let Some(resolved) = lines.remove(pane_id.as_str()) else {
                    continue;
                };
                if ctx.selected && agent.focused {
                    focused = Some(list_rows);
                }
                let prefix = if row.depth > 0 && !row.has_children {
                    if row.last_child {
                        "└─ ".to_owned()
                    } else {
                        "├─ ".to_owned()
                    }
                } else {
                    String::new()
                };
                let row_cx = RowContext {
                    indicators: ctx.indicators,
                    font: ctx.font,
                    theme: ctx.theme,
                    look: ctx.look,
                    width: ctx.width.max(0.),
                    nest: ctx.nest,
                    mark: ctx.mark,
                    host: ctx.host,
                };
                let gap = if ctx.agents_custom && agent_count > 0 {
                    f32::from(ctx.agent_layout.row_gap) * line_height(ctx.font)
                } else {
                    0.
                };
                list_rows += 1;
                agent_count += 1;
                agents = agents.child(tree_agent_row(
                    agent, ctx, &row_cx, gap, &prefix, resolved, cx,
                ));
            }
        }
        prev = Some(row.clone());
    }
    GroupedOutcome {
        agents,
        list_rows,
        agent_count,
        focused,
    }
}

impl HerdrWindow {
    pub(in crate::sidebar) fn append_grouped_tree_rows(
        &self,
        mut list: Stateful<Div>,
        indicators: Indicators,
        look: SidebarLook,
        width: f32,
        cx: &mut Context<Self>,
    ) -> (Stateful<Div>, usize, Option<usize>) {
        let multi = self.endpoints.len() > 1;
        let rows = super::super::cell::layout_for(self.config.layout.mode);
        let font = &self.config.sidebar;
        let theme = &self.theme;
        let content_x = look.content_x();
        let agents_custom = self.custom_agent_rows();
        let query = self.sidebar_search.query().unwrap_or("");
        let mut list_rows = 0usize;
        let mut agent_count = 0usize;
        let mut focused = None;
        for (index, endpoint) in self.endpoints.iter().enumerate() {
            if !endpoint.enabled || !self.device_visible(&endpoint.id) {
                continue;
            }
            let selected = index == self.selected_endpoint;
            let live = if selected { &self.live } else { &endpoint.live };
            let Some(snapshot) = live.snapshot.as_deref() else {
                continue;
            };
            let mark = super::super::wash::HostMark::resolve(
                &self.config.sidebar_style,
                &endpoint.label,
                selected,
                theme,
            );
            let host =
                (multi && endpoint.id != crate::endpoint::LOCAL).then_some(endpoint.label.as_str());
            let ctx = GroupedCtx {
                snapshot,
                endpoint_id: &endpoint.id,
                endpoint_label: &endpoint.label,
                selected,
                multi,
                connected: live.status.is_connected(),
                content_x,
                agents_custom,
                agent_layout: &self.config.sidebar_layout.agents,
                rows,
                indicators,
                font,
                theme,
                look,
                width,
                host,
                nest: 0.,
                mark,
                collapsed: &self.tree_collapsed,
                search: query,
                status_filter: self.sidebar_status_filter,
            };
            let outcome = render_grouped(list, &ctx, list_rows, agent_count, cx);
            list = outcome.agents;
            list_rows = outcome.list_rows;
            agent_count = outcome.agent_count;
            if outcome.focused.is_some() {
                focused = outcome.focused;
            }
        }
        (list, agent_count, focused)
    }
    pub(crate) fn toggle_group(&mut self, key: String, cx: &mut Context<Self>) {
        if !self.tree_collapsed.remove(&key) {
            self.tree_collapsed.insert(key);
        }
        cx.notify();
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
