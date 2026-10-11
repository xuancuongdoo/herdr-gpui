//! Background monitors polled on the tick: plan usage, system load, listening
//! ports, and agent checkpoints. The first three share one host list: this
//! machine always, a remote host while it is connected, so a dropped host is
//! not dialled every few seconds.

use super::{Flash, HerdrWindow};
use gpui::Context;

impl HerdrWindow {
    /// Plan usage follows the selected host: a remote host reports its own
    /// agents' sign-ins, never this machine's.
    pub(super) fn update_usage(&mut self, cx: &mut Context<Self>) -> bool {
        let host = self
            .config
            .usage
            .show
            .then(|| self.endpoints.get(self.selected_endpoint))
            .flatten()
            .and_then(|endpoint| crate::usage::Host::of(&endpoint.connection.target));
        let granted = crate::usage::KeychainGrants::granted(cx);
        let changed = self.usage.poll(
            host,
            &self.config.usage,
            &granted,
            self.config_load_revision,
            self.active,
            std::time::Instant::now(),
        );
        // A denied prompt takes the grant back, so a later launch does not
        // ask again in the background.
        for provider in self.usage.take_denied() {
            crate::usage::KeychainGrants::revoke(provider, cx);
        }
        changed
    }

    /// CPU and memory are sampled for every enabled host, while they are
    /// shown, a Devices overview is open, or a host picker ranks hosts by them.
    pub(super) fn update_system_load(&mut self) -> bool {
        if !self.config.show_system_load
            && !self.devices_overview_open()
            && !self.dispatch_sampling()
            && !self.orchestrator_sampling
        {
            return self.system_load.poll(Vec::new());
        }
        let hosts = self.watched_hosts();
        self.system_load.poll(hosts)
    }

    /// Listening ports are scanned on the same hosts as CPU and memory.
    pub(super) fn update_listening_ports(&mut self) -> bool {
        if !self.config.show_listening_ports {
            self.tunnels = Default::default();
            return self.listening_ports.poll(Vec::new());
        }
        let hosts = self.watched_hosts();
        let changed = self.listening_ports.poll(hosts);
        // A tunnel lives as long as its remote port is listed.
        let ports = &self.listening_ports;
        self.tunnels
            .retain(|key| ports.listening(&crate::usage::Host::Ssh(key.target.clone()), key.port));
        changed
    }

    /// The machines background monitors watch: this one always, a remote
    /// host while it is connected, so a dropped host is not dialled every
    /// few seconds.
    pub(super) fn watched_hosts(&self) -> Vec<crate::usage::Host> {
        self.endpoints
            .iter()
            .enumerate()
            .filter(|(index, endpoint)| {
                let live = if *index == self.selected_endpoint {
                    &self.live
                } else {
                    &endpoint.live
                };
                endpoint.enabled
                    && (live.status.is_connected() || !endpoint.connection.target.is_remote())
            })
            .filter_map(|(_, endpoint)| crate::usage::Host::of(&endpoint.connection.target))
            .collect()
    }

    /// Agent turns are watched on every enabled, connected host this client
    /// may run Git on, and finished restores are reported wherever the user is.
    pub(super) fn update_checkpoints(&mut self, cx: &mut Context<Self>) {
        self.close_stale_checkpoints();
        let enabled = self.config.agent_checkpoints;
        let mut watched = Vec::new();
        for (index, endpoint) in self.endpoints.iter().enumerate() {
            let live = if index == self.selected_endpoint {
                &self.live
            } else {
                &endpoint.live
            };
            let (true, true, Some(host), Some(snapshot)) = (
                enabled,
                endpoint.enabled && live.status.is_connected(),
                crate::checkpoint::host_for(&endpoint.connection.target, live),
                live.snapshot.as_ref(),
            ) else {
                continue;
            };
            self.checkpoints.observe(&endpoint.id, &host, snapshot);
            watched.push(endpoint.id.as_str());
        }
        self.checkpoints
            .retain_endpoints(|endpoint| watched.contains(&endpoint));
        let (changed, restored) = self.checkpoints.poll();
        for restored in restored {
            let flash = match restored.result {
                Ok(()) => Flash::success(format!(
                    "Restored a checkpoint of {}",
                    restored.checkout.branch
                )),
                Err(error) => Flash::warning(format!(
                    "Checkpoint not restored: {}",
                    crate::checkpoint::describe(&error)
                )),
            };
            self.show_flash(flash, cx);
        }
        if changed {
            cx.notify();
        }
    }
}
