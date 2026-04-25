#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Mode {
    List,
    View,
    Search,
    Transition,
    Help,
    ColumnPicker,
    AssigneePicker,
    ComponentPicker,
}
