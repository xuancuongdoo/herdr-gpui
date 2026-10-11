//! Live CPU, memory and age of the inspected agent's pane: the same
//! `processes::Watch` the pane menu opens, on that one pane, for as long as
//! the Metrics tab shows it.

use crate::processes::{Daemon, Listing, Update, Watch};
use crate::sidebar::agent_metrics::FirstSeen;
use crate::sidebar::inspector::InspectorTab;
use crate::window::HerdrWindow;
use gpui::Context;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Default)]
pub(crate) struct AgentLive {
    followed: Option<(String, Daemon)>,
    watch: Option<Watch>,
    pub(crate) listings: HashMap<String, Listing>,
    pub(crate) first_seen: FirstSeen,
}

impl AgentLive {
    /// Moves the watch to `wanted`, dropping the old one, and takes what the
    /// worker has listed since. True when a listing changed.
    fn follow(&mut self, wanted: Option<(String, Daemon)>) -> bool {
        let mut changed = false;
        if wanted != self.followed {
            changed = !self.listings.is_empty();
            self.listings.clear();
            self.watch = wanted
                .clone()
                .and_then(|(pane, daemon)| Watch::start(daemon, pane).ok());
            self.followed = wanted;
        }
        let (Some(watch), Some((pane, _))) = (&self.watch, &self.followed) else {
            return changed;
        };
        while let Ok(Some(update)) = watch.try_update() {
            match update {
                Update::Listing(Ok(listing)) => {
                    self.listings.insert(pane.clone(), listing);
                }
                Update::Listing(Err(_)) => {
                    self.listings.remove(pane);
                }
                Update::Killed(_) => continue,
            }
            changed = true;
        }
        changed
    }
}

impl HerdrWindow {
    /// Each tick: notes when every agent was first seen, and watches the
    /// inspected agent's pane while the Metrics tab shows it.
    pub(crate) fn poll_agent_live(&mut self, cx: &mut Context<Self>) {
        if let Some(snapshot) = self.live.snapshot.as_deref() {
            let now = Instant::now();
            for agent in &snapshot.agents {
                self.agent_live.first_seen.mark(&agent.pane_id, now);
            }
        }
        let wanted = (self.inspector_tab == InspectorTab::Metrics)
            .then(|| {
                self.selected_agent_view()
                    .or_else(|| self.last_inspected.clone())
            })
            .flatten()
            .zip(
                self.endpoints
                    .get(self.selected_endpoint)
                    .and_then(|endpoint| Daemon::for_target(&endpoint.connection.target)),
            )
            .map(|(agent, daemon)| (agent.pane_id, daemon));
        if self.agent_live.follow(wanted) {
            cx.notify();
        }
    }
}
