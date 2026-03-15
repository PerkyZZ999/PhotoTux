use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const APP_NAME: &str = "PhotoTux";
pub const DEFAULT_TILE_SIZE: u32 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub Uuid);

impl LayerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LayerId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasSize {
    pub width: u32,
    pub height: u32,
}

impl CanvasSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl CanvasRect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasRaster {
    pub size: CanvasSize,
    pub pixels: Vec<u8>,
}
