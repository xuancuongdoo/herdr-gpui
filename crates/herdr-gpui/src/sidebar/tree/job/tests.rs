use super::*;

#[test]
fn shared_prefix_is_detected() {
    let names = ["rev1354-gapfk", "rev1354-glsrc"];
    assert_eq!(job_prefix(&names), Some("rev1354-"));
}

#[test]
fn mixed_prefixes_yield_none() {
    let names = ["rev1354-gapfk", "rev1355-glsrc"];
    assert_eq!(job_prefix(&names), None);
}

#[test]
fn single_name_yields_none() {
    let names = ["rev1354-gapfk"];
    assert_eq!(job_prefix(&names), None);
}

#[test]
fn non_ticket_names_yield_none() {
    let names = ["orchestrator", "worker-1"];
    assert_eq!(job_prefix(&names), None);
}

#[test]
fn label_formats_ticket_prefix() {
    assert_eq!(job_label("rev1354-"), "REV-1354");
}

#[test]
fn display_name_strips_prefix() {
    assert_eq!(display_name("rev1354-gapfk", "rev1354-"), "gapfk");
}

#[test]
fn display_name_keeps_full_name_when_empty_after_strip() {
    assert_eq!(display_name("rev1354-", "rev1354-"), "rev1354-");
}
