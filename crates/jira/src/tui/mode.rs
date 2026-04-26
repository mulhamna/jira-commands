#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Mode {
    Browse,
    Search,
    Transition,
    Help,
    ColumnPicker,
    AssigneePicker,
    ComponentPicker,
    SavedJqlPicker,
    ServerInfo,
    ConfigView,
    ThemePicker,
    Modal,
}
