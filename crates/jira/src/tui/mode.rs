#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Mode {
    Browse,
    Search,
    Transition,
    Help,
    ProjectVersionBrowser,
    ColumnPicker,
    AssigneePicker,
    ComponentPicker,
    FixVersionPicker,
    SprintPicker,
    SavedJqlPicker,
    ServerInfo,
    ConfigView,
    ThemePicker,
    Modal,
}
