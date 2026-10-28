pub struct RightPane;

pub enum RightPaneVariant {
    Preview(PreviewVariant),
}

pub enum PreviewVariant {
    Content,
    Metadata,
}
