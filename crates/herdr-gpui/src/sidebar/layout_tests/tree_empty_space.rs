//! Empty spaces draw no child connector.
//!
//! Regression: a space with zero agents painted `├─`/`└─` before its name,
//! as if it had children, while spaces with agents show `▾`/`▸`.
#![allow(clippy::expect_used)]

use crate::sidebar::tree::render::{fold_marker, guides};
use crate::sidebar::tree::{TreeNode, build_agent_tree};
use std::collections::HashSet;
type TestAgent = (String, String, String, Vec<(String, String)>);

#[test]
fn empty_space_draws_no_child_connector() {
    let devices = vec![(
        "Local".to_owned(),
        vec![("w0".to_owned(), "loadlab".to_owned())],
    )];
    let agents: Vec<TestAgent> = vec![];
    let rows = build_agent_tree(&devices, &agents, &HashSet::new());
    let space = rows
        .iter()
        .find(|row| matches!(&row.node, TreeNode::Space { .. }))
        .expect("space row");
    assert!(!space.has_children, "empty space has no children");
    assert_eq!(fold_marker(space.has_children, space.collapsed), "");
    let prefix = guides(space);
    assert!(
        !prefix.contains('├') && !prefix.contains('└'),
        "no child connector, got {prefix:?}"
    );
}
