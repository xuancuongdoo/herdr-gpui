//! The agents list: every listed host's agents in the panel's one order, so a
//! blocked agent on one host can sit above an idle one on another.

use crate::{
    HerdrWindow, NavigationTarget,
    sidebar::{
        Indicators, agent_name,
        agents::{agent_place, group_agents_by_space, state_label, status_text},
        cell::{AgentRow, Cell, RowContext, RowData, layout_for},
        label_text,
        layout::SidebarLook,
        line_height, tokens, wash,
    },
};
use gpui::{prelude::*, *};
use herdr_client::protocol::{ClientShellAgent, ClientShellSnapshot};

impl HerdrWindow {
    /// Appends a row for each agent `panel_agents` lists, in that order, and
    /// returns the list with how many rows it painted and where the selected
    /// host's focused agent sits among them, for the reveal. A row's host
    /// shapes its own context, as the agent may belong to any listed host.
    pub(super) fn append_agent_rows(
        &self,
        mut list: Stateful<Div>,
        indicators: Indicators,
        look: SidebarLook,
        width: f32,
        cx: &mut Context<Self>,
    ) -> (Stateful<Div>, usize, Option<usize>) {
        if self.agent_sort == crate::preferences::AgentSort::Grouped
            && self.config.tree_view
            && !self.plugin_agent_view()
        {
            return self.append_grouped_tree_rows(list, indicators, look, width, cx);
        }
        let font = &self.config.sidebar;
        let theme = &self.theme;
        let multi = self.endpoints.len() > 1;
        let grouped = self.agent_sort == crate::preferences::AgentSort::Grouped;
        let agents_custom = self.custom_agent_rows();
        let content_x = look.content_x();
        let mut count = 0;
        let mut list_rows = 0usize;
        let mut focused = None;
        let flat = self.panel_agents();
        if !grouped {
            for (index, agent) in &flat {
                let Some(endpoint) = self.endpoints.get(*index) else {
                    continue;
                };
                let selected = *index == self.selected_endpoint;
                let live = if selected { &self.live } else { &endpoint.live };
                let Some(snapshot) = live.snapshot.as_deref() else {
                    continue;
                };
                let row_cx = RowContext {
                    indicators,
                    font,
                    theme,
                    look,
                    width,
                    nest: 0.,
                    mark: wash::HostMark::resolve(
                        &self.config.sidebar_style,
                        &endpoint.label,
                        selected,
                        theme,
                    ),
                    host: (multi && endpoint.id != crate::endpoint::LOCAL)
                        .then_some(endpoint.label.as_str()),
                };
                let Some(lines) = self.configured_agent_lines(agent, snapshot, row_cx.host) else {
                    continue;
                };
                if selected && agent.focused {
                    focused = Some(list_rows);
                }
                let gap = self.agent_row_gap(count);
                count += 1;
                list_rows += 1;
                list =
                    list.child(self.agent_cell(*index, agent, snapshot, &row_cx, lines, gap, cx));
            }
            return (list, count, focused);
        }
        let mut host_order = Vec::new();
        for (index, _) in &flat {
            if host_order.last() != Some(index) {
                host_order.push(*index);
            }
        }
        for index in host_order {
            let Some(endpoint) = self.endpoints.get(index) else {
                continue;
            };
            let selected = index == self.selected_endpoint;
            let live = if selected { &self.live } else { &endpoint.live };
            let Some(snapshot) = live.snapshot.as_deref() else {
                continue;
            };
            let row_cx = RowContext {
                indicators,
                font,
                theme,
                look,
                width,
                nest: 0.,
                mark: wash::HostMark::resolve(
                    &self.config.sidebar_style,
                    &endpoint.label,
                    selected,
                    theme,
                ),
                host: (multi && endpoint.id != crate::endpoint::LOCAL)
                    .then_some(endpoint.label.as_str()),
            };
            let ordered: Vec<(Option<&str>, &ClientShellAgent)> =
                if snapshot.agent_view_label.is_none() {
                    group_agents_by_space(snapshot)
                        .into_iter()
                        .flat_map(|(label, agents)| {
                            agents
                                .into_iter()
                                .enumerate()
                                .map(move |(position, agent)| {
                                    (if position == 0 { Some(label) } else { None }, agent)
                                })
                        })
                        .collect()
                } else {
                    flat.iter()
                        .filter(|(candidate, _)| *candidate == index)
                        .map(|(_, agent)| (None, *agent))
                        .collect()
                };
            let mut pending_header: Option<&str> = None;
            for (header, agent) in ordered {
                if let Some(label) = header {
                    pending_header = Some(label);
                }
                let Some(lines) = self.configured_agent_lines(agent, snapshot, row_cx.host) else {
                    continue;
                };
                if let Some(label) = pending_header {
                    let show = !agents_custom
                        || lines
                            .iter()
                            .flatten()
                            .any(|token| token.kind.text().is_some());
                    if show {
                        list = list.child(
                            div()
                                .debug_selector(|| format!("agent-group-{label}"))
                                .flex_none()
                                .px(px(content_x))
                                .pt(px(4.))
                                .text_color(rgb(theme.muted))
                                .truncate()
                                .child(label_text(label)),
                        );
                        list_rows += 1;
                        pending_header = None;
                    }
                }
                if selected && agent.focused {
                    focused = Some(list_rows);
                }
                let gap = self.agent_row_gap(count);
                count += 1;
                list_rows += 1;
                list = list.child(self.agent_cell(index, agent, snapshot, &row_cx, lines, gap, cx));
            }
        }
        (list, count, focused)
    }
    pub(in crate::sidebar) fn plugin_agent_view(&self) -> bool {
        self.endpoints.iter().enumerate().any(|(index, endpoint)| {
            if !endpoint.enabled || !self.device_visible(&endpoint.id) {
                return false;
            }
            let live = if index == self.selected_endpoint {
                &self.live
            } else {
                &endpoint.live
            };
            live.snapshot
                .as_deref()
                .is_some_and(|snapshot| snapshot.agent_view_label.is_some())
        })
    }

    /// Whether the agents' rows follow a configured `[sidebar_layout.agents]`.
    pub(in crate::sidebar) fn custom_agent_rows(&self) -> bool {
        self.config.usage.inline
            && self.config.sidebar_layout.agents != crate::config::AgentLayout::default()
    }

    /// The lines a configured agent row shows, empty for Herdr's own row, or
    /// None when the configured rows leave this agent out.
    pub(in crate::sidebar) fn configured_agent_lines(
        &self,
        agent: &ClientShellAgent,
        snapshot: &ClientShellSnapshot,
        host: Option<&str>,
    ) -> Option<Vec<Vec<tokens::ResolvedToken>>> {
        if !self.custom_agent_rows() {
            return Some(Vec::new());
        }
        tokens::agent_rows(&self.config.sidebar_layout.agents, agent, snapshot, host)
    }

    /// Space above the agent row painted `count`-th in its list: a
    /// configured `row_gap` between configured rows, none otherwise.
    pub(super) fn agent_row_gap(&self, count: usize) -> f32 {
        if self.custom_agent_rows() && count > 0 {
            f32::from(self.config.sidebar_layout.agents.row_gap) * line_height(&self.config.sidebar)
        } else {
            0.
        }
    }

    /// One agent's row, for the agents list or under its device's header:
    /// the agent of endpoint `index`, drawn in `row_cx`, that shows its pane
    /// when clicked.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::sidebar) fn agent_cell(
        &self,
        index: usize,
        agent: &ClientShellAgent,
        snapshot: &ClientShellSnapshot,
        row_cx: &RowContext,
        lines: Vec<Vec<tokens::ResolvedToken>>,
        gap: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let rows = layout_for(self.config.layout.mode);
        let multi = self.endpoints.len() > 1;
        let selected = index == self.selected_endpoint;
        let endpoint_id = self
            .endpoints
            .get(index)
            .map(|endpoint| endpoint.id.clone())
            .unwrap_or_default();
        let id = agent.pane_id.clone();
        let navigate_endpoint = endpoint_id.clone();
        Cell::new(
            rows,
            RowData::Agent(AgentRow {
                key: format!("agent-{id}"),
                name: agent_name(agent),
                icon: crate::icons::AgentIcon::from_identity(agent.agent.as_deref()),
                status: agent.agent_status,
                place: agent_place(agent, snapshot),
                status_text: self
                    .config
                    .sidebar_layout
                    .agents
                    .shows_status_text(agent.agent.as_deref())
                    .then(|| state_label(agent, status_text(agent.agent_status))),
                lines,
            }),
            row_cx,
        )
        .selected(selected && agent.focused)
        .row()
        .when(gap > 0., |row| row.mt(px(gap)))
        .id(SharedString::from(format!("agent-{endpoint_id}-{id}")))
        .when(multi, |row| {
            row.debug_selector(|| format!("agent-{endpoint_id}-{id}"))
        })
        .on_click(cx.listener(move |this, _, window, cx| {
            this.navigate_endpoint(&navigate_endpoint, NavigationTarget::Pane(&id), cx);
            window.focus(&this.focus, cx);
        }))
    }
}
