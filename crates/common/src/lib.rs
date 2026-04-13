use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const APP_NAME: &str = "PhotoTux";
pub const DEFAULT_TILE_SIZE: u32 = 256;

macro_rules! define_uuid_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

define_uuid_id!(DocumentId);
define_uuid_id!(LayerId);
define_uuid_id!(GroupId);

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
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasRaster {
    pub size: CanvasSize,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestructiveFilterKind {
    InvertColors,
    Desaturate,
}

impl DestructiveFilterKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::InvertColors => "Invert Colors",
            Self::Desaturate => "Desaturate",
        }
    }
}
