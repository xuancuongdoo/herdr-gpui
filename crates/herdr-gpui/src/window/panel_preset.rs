#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PanelPreset {
    TreeOnly,
    TreeTerminal,
    #[default]
    All,
}

impl PanelPreset {
    pub(crate) fn show_terminal(self) -> bool {
        self != Self::TreeOnly
    }

    pub(crate) fn show_inspector(self) -> bool {
        self == Self::All
    }
}
