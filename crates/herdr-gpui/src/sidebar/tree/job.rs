//! Ticket-like job grouping helpers for the sidebar agent tree.
//!
//! Pure string helpers: extract the shared `<letters><digits>-` prefix from a
//! set of agent names and format it as a group label.

#[cfg(test)]
mod tests;

/// Ticket-like leading prefix (`<letters><digits>-`) of one name.
fn ticket_of(name: &str) -> Option<&str> {
    let bytes = name.as_bytes();
    let mut end = 0;
    while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    let mut digits = end;
    while digits < bytes.len() && bytes[digits].is_ascii_digit() {
        digits += 1;
    }
    if digits == end || digits >= bytes.len() || bytes[digits] != b'-' {
        return None;
    }
    name.get(..digits + 1)
}

/// Shared ticket-like prefix of all names, e.g. `Some("rev1354-")`.
pub(super) fn job_prefix<'a>(names: &[&'a str]) -> Option<&'a str> {
    if names.len() < 2 {
        return None;
    }
    let first = names.first().and_then(|name| ticket_of(name))?;
    if names[1..].iter().all(|name| ticket_of(name) == Some(first)) {
        names.first().and_then(|name| name.get(..first.len()))
    } else {
        None
    }
}

/// Format a ticket prefix as a group label, e.g. `"rev1354-"` → `"REV-1354"`.
pub(super) fn job_label(prefix: &str) -> String {
    let stripped = prefix.strip_suffix('-').unwrap_or(prefix);
    let split = stripped
        .char_indices()
        .find(|(_, c)| c.is_ascii_digit())
        .map(|(i, _)| i);
    match split {
        Some(i) => {
            let (letters, digits) = stripped.split_at(i);
            if letters.is_empty() || digits.is_empty() {
                stripped.to_ascii_uppercase()
            } else {
                format!("{}-{digits}", letters.to_ascii_uppercase())
            }
        }
        None => stripped.to_ascii_uppercase(),
    }
}

/// Name with the shared prefix stripped; full name when nothing remains.
pub(super) fn display_name<'a>(name: &'a str, prefix: &str) -> &'a str {
    match name.strip_prefix(prefix) {
        Some(rest) if !rest.is_empty() => rest,
        _ => name,
    }
}
