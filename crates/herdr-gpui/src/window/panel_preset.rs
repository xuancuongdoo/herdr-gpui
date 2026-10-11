#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PanelPreset {
    TreeOnly,
    #[default]
    TreeTerminal,
}

impl PanelPreset {
    pub(crate) fn show_terminal(self) -> bool {
        self != Self::TreeOnly
    }
}
