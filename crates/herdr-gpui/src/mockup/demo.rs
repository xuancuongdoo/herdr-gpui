// The built-in mockup, shown when no HERDR_MOCKUP_FILE was given. It is also
// the template for an agent's own file: the same imports, the same
// `pub(super) fn mockup() -> Mockup`, and both kinds of body. A file compiled
// in through `include!` may not start with inner attributes or `//!` docs.

use super::{Body, Look, Mockup, Variant};
use crate::sidebar::tree::render::{agent_line, fold_marker, guides, job_line};
use crate::sidebar::tree::{TreeNode, build_agent_tree};
use crate::{config::corners, fonts::StyledFont, icons::AgentIcon};
use gpui::{prelude::*, *};
use std::collections::HashSet;

pub(super) fn mockup() -> Mockup {
    Mockup {
        title: "Workspace status chip",
        frame: size(px(300.), px(96.)),
        variants: vec![
            Variant {
                name: "Pill",
                note: "Everything on one line: agent, name, branch in a pill.",
                body: Body::Element(pill),
            },
            Variant {
                name: "Two-line",
                note: "Name first, branch and state quieter underneath.",
                body: Body::Element(two_line),
            },
            Variant {
                name: "Accent bar",
                note: "A state-colored bar carries status; no dot.",
                body: Body::Element(accent_bar),
            },
            Variant {
                name: "Agent tree",
                note: "Real tree rows at a 160px sidebar: datacaliper into REV-1354 into its REV-1354 job, four short workers, collapsed Done (2).",
                body: Body::Element(agent_tree),
            },
            Variant {
                name: "Expandable",
                note: "Click to reveal the agent's last message.",
                body: Body::View(|_, cx| cx.new(|_| Expandable::default()).into()),
            },
        ],
    }
}

/// Realistic content every variant shows, so they differ only in design.
const NAME: &str = "herdr-gpui";
const BRANCH: &str = "feat/native-mockups";
const MESSAGE: &str = "Ran just ci: 412 tests passed. Waiting for review.";

fn agent_icon(look: &Look, size: f32) -> Svg {
    svg()
        .flex_none()
        .size(px(size))
        .path(AgentIcon::Claude.path())
        .text_color(rgb(look.theme.foreground))
}

fn working(look: &Look) -> u32 {
    look.theme.ink(look.theme.palette[2])
}

fn pill(look: &Look, _: &mut Window, _: &mut App) -> AnyElement {
    let theme = &look.theme;
    div()
        .p_3()
        .flex()
        .items_center()
        .gap_2()
        .child(agent_icon(look, 16.))
        .child(div().flex_none().child(NAME))
        .child(
            div()
                .min_w_0()
                .px_2()
                .rounded(px(corners::CONTROL))
                .bg(rgb(theme.active))
                .text_color(rgb(theme.subtext()))
                .text_font(&look.config.terminal)
                .text_size(px(look.config.ui.size - 1.))
                .truncate()
                .child(BRANCH),
        )
        .child(div().flex_1())
        .child(
            div()
                .flex_none()
                .size(px(8.))
                .rounded_full()
                .bg(rgb(working(look))),
        )
        .into_any_element()
}

fn two_line(look: &Look, _: &mut Window, _: &mut App) -> AnyElement {
    let theme = &look.theme;
    div()
        .p_3()
        .flex()
        .gap_3()
        .child(agent_icon(look, 20.))
        .child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .child(div().font_weight(FontWeight::SEMIBOLD).child(NAME))
                .child(
                    div()
                        .flex()
                        .gap_1()
                        .text_color(rgb(theme.muted))
                        .child(div().min_w_0().truncate().child(BRANCH))
                        .child("·")
                        .child(
                            div()
                                .flex_none()
                                .text_color(rgb(working(look)))
                                .child("working"),
                        ),
                ),
        )
        .into_any_element()
}

fn accent_bar(look: &Look, _: &mut Window, _: &mut App) -> AnyElement {
    let theme = &look.theme;
    div()
        .flex()
        .h_full()
        .child(div().flex_none().w(px(3.)).bg(rgb(working(look))))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .p_3()
                .flex()
                .flex_col()
                .child(NAME)
                .child(
                    div()
                        .text_color(rgb(theme.subtext()))
                        .truncate()
                        .child(BRANCH),
                ),
        )
        .into_any_element()
}

/// A stateful variant: the card keeps its expanded state across theme and
/// width changes, because the window builds the view only once.
#[derive(Default)]
struct Expandable {
    open: bool,
}

impl Render for Expandable {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.global::<Look>();
        let theme = &look.theme;
        let hover = theme.surface;
        div()
            .id("expandable")
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(hover)))
            .on_click(cx.listener(|this, _, _, cx| {
                this.open = !this.open;
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(agent_icon(look, 16.))
                    .child(div().flex_1().min_w_0().truncate().child(NAME))
                    .child(
                        svg()
                            .flex_none()
                            .size(px(12.))
                            .path(if self.open {
                                "icons/chevron-up.svg"
                            } else {
                                "icons/chevron-down.svg"
                            })
                            .text_color(rgb(theme.muted)),
                    ),
            )
            .when(self.open, |el| {
                el.child(
                    div()
                        .p_2()
                        .rounded(px(corners::SMALL))
                        .bg(rgb(theme.surface))
                        .text_color(rgb(theme.subtext()))
                        .child(MESSAGE),
                )
            })
    }
}

/// Narrow agent-tree fixture: the REAL sidebar tree rows (`spaces` sort) at a
/// 160px sidebar column, so a capture proves the approved design instead of a
/// generic chip. Data mirrors the canonical spec in
/// `crate::sidebar::layout_tests::visual_tree_fixture` (kept in sync by hand:
/// `mockup` builds only with its feature, so it cannot reuse that module).
/// Endpoint `datacaliper` expands through space `orch` into the `REV-1354`
/// ticket job with four short-named workers; endpoint `Done` stays collapsed
/// with its two agents hidden. Rows carry short names only, never model names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct VisualTreeSpace {
    id: String,
    label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct VisualTreeDevice {
    name: String,
    spaces: Vec<VisualTreeSpace>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct VisualTreeAttr {
    key: String,
    value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct VisualTreeAgent {
    workspace: String,
    pane_id: String,
    name: String,
    attrs: Vec<VisualTreeAttr>,
}

fn visual_tree_devices() -> Vec<VisualTreeDevice> {
    vec![
        VisualTreeDevice {
            name: "datacaliper".to_owned(),
            spaces: vec![VisualTreeSpace {
                id: "w-rev".to_owned(),
                label: "orch".to_owned(),
            }],
        },
        VisualTreeDevice {
            name: "Done".to_owned(),
            spaces: Vec::new(),
        },
    ]
}

fn visual_tree_agents() -> Vec<VisualTreeAgent> {
    vec![
        VisualTreeAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-orch".to_owned(),
            name: "rev1354-orch".to_owned(),
            attrs: vec![VisualTreeAttr {
                key: "role".to_owned(),
                value: "ORCH".to_owned(),
            }],
        },
        VisualTreeAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-gapfk".to_owned(),
            name: "rev1354-gapfk".to_owned(),
            attrs: vec![VisualTreeAttr {
                key: "parent".to_owned(),
                value: "p-orch".to_owned(),
            }],
        },
        VisualTreeAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-glimpact".to_owned(),
            name: "rev1354-glimpact".to_owned(),
            attrs: vec![VisualTreeAttr {
                key: "parent".to_owned(),
                value: "p-orch".to_owned(),
            }],
        },
        VisualTreeAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-glhist".to_owned(),
            name: "rev1354-glhist".to_owned(),
            attrs: vec![VisualTreeAttr {
                key: "parent".to_owned(),
                value: "p-orch".to_owned(),
            }],
        },
        VisualTreeAgent {
            workspace: "w-rev".to_owned(),
            pane_id: "p-a1".to_owned(),
            name: "rev1354-a1".to_owned(),
            attrs: vec![VisualTreeAttr {
                key: "parent".to_owned(),
                value: "p-orch".to_owned(),
            }],
        },
        VisualTreeAgent {
            workspace: "w-done".to_owned(),
            pane_id: "p-done-1".to_owned(),
            name: "done-1".to_owned(),
            attrs: Vec::new(),
        },
        VisualTreeAgent {
            workspace: "w-done".to_owned(),
            pane_id: "p-done-2".to_owned(),
            name: "done-2".to_owned(),
            attrs: Vec::new(),
        },
    ]
}

/// Agents of one device by the tree's own rule: listed workspaces, or the
/// last device when no device lists the workspace.
fn visual_tree_assigned(
    devices: &[VisualTreeDevice],
    agents: &[VisualTreeAgent],
    index: usize,
) -> usize {
    let listed = |workspace: &str| {
        devices
            .iter()
            .any(|device| device.spaces.iter().any(|space| space.id == workspace))
    };
    agents
        .iter()
        .filter(|agent| {
            devices[index]
                .spaces
                .iter()
                .any(|space| space.id == agent.workspace)
                || (index + 1 == devices.len() && !listed(&agent.workspace))
        })
        .count()
}

/// One row per visible tree row: its left text (built with the same helpers
/// production paints with) and whether it is a header. The collapsed endpoint
/// folds its hidden count into its row.
fn visual_tree_rows() -> Vec<(String, bool)> {
    let devices = visual_tree_devices();
    let agents = visual_tree_agents();
    let collapsed = HashSet::from(["dev:Done".to_owned()]);
    let device_tuples = devices
        .iter()
        .map(|device| {
            (
                device.name.clone(),
                device
                    .spaces
                    .iter()
                    .map(|space| (space.id.clone(), space.label.clone()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let agent_tuples = agents
        .iter()
        .map(|agent| {
            (
                agent.workspace.clone(),
                agent.pane_id.clone(),
                agent.name.clone(),
                agent
                    .attrs
                    .iter()
                    .map(|attr| (attr.key.clone(), attr.value.clone()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    for row in build_agent_tree(&device_tuples, &agent_tuples, &collapsed) {
        let header = !matches!(row.node, TreeNode::Agent { .. });
        let text = match &row.node {
            TreeNode::Device { label } if row.collapsed => {
                let hidden = device_tuples
                    .iter()
                    .position(|(name, _)| name == label)
                    .map_or(0, |index| visual_tree_assigned(&devices, &agents, index));
                format!(
                    "{}{} {label} ({hidden})",
                    guides(&row),
                    fold_marker(row.has_children, row.collapsed)
                )
            }
            TreeNode::Device { label } => format!(
                "{}{} {label}",
                guides(&row),
                fold_marker(row.has_children, row.collapsed)
            ),
            TreeNode::Space { label, .. } => format!(
                "{}{} {label}",
                guides(&row),
                fold_marker(row.has_children, row.collapsed)
            ),
            TreeNode::Job { label, total, .. } => {
                job_line(
                    label,
                    *total,
                    row.has_children,
                    row.collapsed,
                    &guides(&row),
                )
                .0
            }
            TreeNode::Group { kind, total, .. } => {
                job_line(
                    kind.label(),
                    *total,
                    row.has_children,
                    row.collapsed,
                    &guides(&row),
                )
                .0
            }
            TreeNode::Agent {
                name, orchestrator, ..
            } => {
                let role = if *orchestrator { Some("ORCH") } else { None };
                let (line, _) = agent_line(name, role, None);
                format!("{}● {line}", guides(&row))
            }
        };
        out.push((text, header));
    }
    out
}

fn agent_tree(look: &Look, _: &mut Window, _: &mut App) -> AnyElement {
    let theme = &look.theme;
    let mut column = div().w(px(160.)).flex().flex_col().p_2();
    for (text, header) in visual_tree_rows() {
        column = column.child(
            div()
                .flex_none()
                .w_full()
                .truncate()
                .when(header, |row| row.font_weight(FontWeight::BOLD))
                .text_color(rgb(theme.foreground))
                .child(SharedString::from(text)),
        );
    }
    column.into_any_element()
}
