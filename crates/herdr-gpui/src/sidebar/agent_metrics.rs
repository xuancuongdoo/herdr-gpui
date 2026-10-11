use super::inspector_fields::format_elapsed;
use crate::pane_menu::processes::{cpu_text, memory_text};
use crate::processes::Listing;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub(crate) struct AgentMetrics {
    pub(crate) cpu: f32,
    pub(crate) memory: u64,
    pub(crate) elapsed: Duration,
}
#[derive(Default)]
pub(crate) struct HostMetrics {
    pub(crate) cpu: Option<f32>,
    pub(crate) memory_used: Option<u64>,
    pub(crate) memory_total: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MetricKind {
    Cpu,
    Mem,
    Elapsed,
}
pub(crate) struct MetricRow {
    pub(crate) label: &'static str,
    pub(crate) value: String,
    pub(crate) kind: MetricKind,
    pub(crate) fill: Option<f32>,
}
const DASH: &str = "—";
fn share(value: f32) -> f32 {
    (value / 100.).clamp(0., 1.)
}
fn memory_share(used: Option<u64>, total: Option<u64>) -> Option<f32> {
    used.zip(total)
        .filter(|(_, total)| *total > 0)
        .map(|(used, total)| (used as f32 / total as f32).clamp(0., 1.))
}
pub(crate) fn metric_rows(metrics: Option<&AgentMetrics>, host: &HostMetrics) -> Vec<MetricRow> {
    let agent_cpu = metrics.map(|metrics| metrics.cpu);
    let agent_memory = metrics.map(|metrics| metrics.memory);
    vec![
        MetricRow {
            label: "Agent CPU",
            value: agent_cpu
                .map(|cpu| cpu_text(Some(cpu)))
                .unwrap_or_else(|| DASH.into()),
            kind: MetricKind::Cpu,
            fill: agent_cpu.map(share),
        },
        MetricRow {
            label: "Agent MEM",
            value: agent_memory.map(memory_text).unwrap_or_else(|| DASH.into()),
            kind: MetricKind::Mem,
            fill: memory_share(agent_memory, host.memory_total),
        },
        MetricRow {
            label: "Elapsed",
            value: metrics
                .map(|metrics| format_elapsed(metrics.elapsed))
                .unwrap_or_else(|| DASH.into()),
            kind: MetricKind::Elapsed,
            fill: None,
        },
        MetricRow {
            label: "Host CPU",
            value: host
                .cpu
                .map(|cpu| cpu_text(Some(cpu)))
                .unwrap_or_else(|| DASH.into()),
            kind: MetricKind::Cpu,
            fill: host.cpu.map(share),
        },
        MetricRow {
            label: "Host MEM",
            value: host
                .memory_used
                .map(memory_text)
                .unwrap_or_else(|| DASH.into()),
            kind: MetricKind::Mem,
            fill: memory_share(host.memory_used, host.memory_total),
        },
    ]
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
        if self.seen.contains_key(pane_id) {
            return;
        }
        self.seen.insert(pane_id.to_owned(), now);
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
    #[test]
    fn metrics_tab_shows_fixture_values_and_dash_without_listing() {
        let rows = vec![Row {
            entry: entry(10, Some(10.0), 35 << 20),
            depth: 0,
            foreground: true,
        }];
        let mut listings = HashMap::new();
        listings.insert("pane-1".to_owned(), listing_from(rows, 10.0, 35 << 20));
        let mut first_seen = FirstSeen::default();
        let start = Instant::now();
        first_seen.mark("pane-1", start);
        let later = start + Duration::from_secs(125);
        let metrics =
            metrics_for_pane("pane-1", &listings, &first_seen, later).expect("pane listing exists");
        let host = HostMetrics {
            cpu: Some(12.0),
            memory_used: Some(1 << 30),
            memory_total: Some(2 << 30),
        };
        let rows = metric_rows(Some(&metrics), &host);
        let value = |label: &str| {
            rows.iter()
                .find(|row| row.label == label)
                .map(|row| row.value.clone())
                .unwrap_or_default()
        };
        assert_eq!(value("Agent CPU"), "10%");
        assert_eq!(value("Agent MEM"), "35 MB");
        assert_eq!(value("Elapsed"), "2m");
        assert_eq!(value("Host CPU"), "12%");
        assert_eq!(value("Host MEM"), "1.0 GB");
        assert!(metrics_for_pane("pane-9", &listings, &first_seen, later).is_none());
        let missing = metric_rows(None, &HostMetrics::default());
        assert_eq!(missing.len(), 5);
        assert!(missing.iter().all(|row| row.value == "—"));
        assert!(missing.iter().all(|row| row.fill.is_none()));
    }
}
