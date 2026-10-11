use crate::processes::Listing;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub(crate) struct AgentMetrics {
    pub(crate) cpu: f32,
    pub(crate) memory: u64,
    pub(crate) elapsed: Duration,
}

pub(crate) fn totals(listing: &Listing) -> (f32, u64) {
    (listing.cpu, listing.memory)
}

#[derive(Default)]
pub(crate) struct FirstSeen {
    seen: HashMap<String, Instant>,
}

impl FirstSeen {
    pub(crate) fn mark(&mut self, pane_id: &str, now: Instant) {
        self.seen.entry(pane_id.to_owned()).or_insert(now);
    }

    pub(crate) fn elapsed(&self, pane_id: &str, now: Instant) -> Duration {
        self.seen
            .get(pane_id)
            .map(|first| now.saturating_duration_since(*first))
            .unwrap_or_default()
    }
}

pub(crate) fn metrics_for_pane(
    pane_id: &str,
    listings: &HashMap<String, Listing>,
    first_seen: &FirstSeen,
    now: Instant,
) -> Option<AgentMetrics> {
    let listing = listings.get(pane_id)?;
    let (cpu, memory) = totals(listing);
    Some(AgentMetrics {
        cpu,
        memory,
        elapsed: first_seen.elapsed(pane_id, now),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::processes::{Entry, Identity, Row};

    fn entry(pid: u32, cpu: Option<f32>, memory: u64) -> Entry {
        Entry {
            identity: Identity {
                pid,
                started: u64::from(pid),
            },
            parent: None,
            name: format!("p{pid}"),
            command: String::new(),
            cpu,
            memory,
        }
    }

    fn listing_from(rows: Vec<Row>, cpu: f32, memory: u64) -> Listing {
        Listing {
            rows,
            hidden: 0,
            cpu,
            memory,
        }
    }

    #[test]
    fn pane_resources_sum_the_process_tree_and_elapsed_keys_by_pane() {
        let rows = vec![
            Row {
                entry: entry(10, Some(2.5), 10 << 20),
                depth: 0,
                foreground: false,
            },
            Row {
                entry: entry(20, Some(7.5), 20 << 20),
                depth: 1,
                foreground: true,
            },
            Row {
                entry: entry(30, None, 5 << 20),
                depth: 1,
                foreground: false,
            },
        ];
        let listing = listing_from(rows, 10.0, 35 << 20);
        assert_eq!(totals(&listing), (10.0, 35 << 20));

        let mut listings = HashMap::new();
        listings.insert("pane-1".to_owned(), listing);
        let mut first_seen = FirstSeen::default();
        let start = Instant::now();
        first_seen.mark("pane-1", start);
        let later = start + Duration::from_secs(125);
        let metrics =
            metrics_for_pane("pane-1", &listings, &first_seen, later).expect("pane listing exists");
        assert_eq!(metrics.cpu, 10.0);
        assert_eq!(metrics.memory, 35 << 20);
        assert_eq!(metrics.elapsed, Duration::from_secs(125));
        assert_eq!(first_seen.elapsed("pane-9", later), Duration::ZERO);
        assert!(metrics_for_pane("pane-9", &listings, &first_seen, start).is_none());
    }
}
