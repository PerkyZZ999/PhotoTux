#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Brush,
    Eraser,
    Move,
    RectangularMarquee,
    Hand,
    Zoom,
}
