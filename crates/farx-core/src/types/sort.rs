#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum SortField {
    #[default]
    Name,
    Extension,
    Size,
    Modified,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum PanelViewMode {
    Brief,
    Medium,
    #[default]
    Full,
    Wide,
}
