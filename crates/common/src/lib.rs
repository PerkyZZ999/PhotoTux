//! Shared cross-crate types and utilities for PhotoTux.

pub mod geometry;
pub mod ids;
pub mod math;

pub use geometry::{Point, Rect, Size, Transform, Vector};
pub use ids::{DocumentId, LayerId};
pub use math::{clamp_f32, lerp_f32, round_to_pixel};
