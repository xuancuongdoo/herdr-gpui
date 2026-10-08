//! Rendered-geometry guard for the screenshot-proven `model`/`working`
//! collision (`tree-source-accept · worker · muse-spark-1.3working`): at a
//! tight-fit width the dim `role · model` suffix and the right-column state
//! word must keep a positive horizontal gap between their painted boxes.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{TextProbes, fixture_window, full_draw, snapshot};
use crate::preferences::AgentSort;
use gpui::{px, size};
use herdr_client::protocol::AgentStatus;
use std::sync::Arc;

// The dim suffix and the state word shared one tree line; the state word's
// box started one glyph past the suffix box, so their painted text never
// touched even when the suffix filled its own box exactly.
// Deleted tree_worker_keeps_gap_between_suffix_and_status: suffix and right-column status removed by design (glyph+name only).
