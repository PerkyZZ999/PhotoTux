//! Geometry primitives used across PhotoTux crates.

/// A point in 2D space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal position.
    pub x: f32,
    /// Vertical position.
    pub y: f32,
}

impl Point {
    /// Create a new point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A directional vector in 2D space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector {
    /// Horizontal delta.
    pub dx: f32,
    /// Vertical delta.
    pub dy: f32,
}

impl Vector {
    /// Create a new vector.
    #[must_use]
    pub const fn new(dx: f32, dy: f32) -> Self {
        Self { dx, dy }
    }
}

/// A 2D size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    /// Width in pixels or logical units.
    pub width: f32,
    /// Height in pixels or logical units.
    pub height: f32,
}

impl Size {
    /// Create a new size.
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Return the right-most x coordinate.
    #[must_use]
    pub fn max_x(self) -> f32 {
        self.x + self.width
    }

    /// Return the bottom-most y coordinate.
    #[must_use]
    pub fn max_y(self) -> f32 {
        self.y + self.height
    }

    /// Check whether the rectangle contains a point.
    #[must_use]
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x && point.x <= self.max_x() && point.y >= self.y && point.y <= self.max_y()
    }
}

/// A simple 2D transform for translate-and-scale workflows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// Horizontal scale.
    pub scale_x: f32,
    /// Vertical scale.
    pub scale_y: f32,
    /// Horizontal translation.
    pub translate_x: f32,
    /// Vertical translation.
    pub translate_y: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    /// Create the identity transform.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
        }
    }

    /// Return a translated transform.
    #[must_use]
    pub const fn translated(translate_x: f32, translate_y: f32) -> Self {
        Self {
            translate_x,
            translate_y,
            ..Self::identity()
        }
    }

    /// Return a scaled transform.
    #[must_use]
    pub const fn scaled(scale_x: f32, scale_y: f32) -> Self {
        Self {
            scale_x,
            scale_y,
            ..Self::identity()
        }
    }

    /// Apply the transform to a point.
    #[must_use]
    pub fn apply_to_point(self, point: Point) -> Point {
        Point::new(
            point.x * self.scale_x + self.translate_x,
            point.y * self.scale_y + self.translate_y,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Point, Rect, Transform};

    #[test]
    fn rect_contains_points_inside_bounds() {
        let rect = Rect::new(10.0, 20.0, 30.0, 40.0);

        assert!(rect.contains(Point::new(10.0, 20.0)));
        assert!(rect.contains(Point::new(40.0, 60.0)));
        assert!(rect.contains(Point::new(25.0, 35.0)));
        assert!(!rect.contains(Point::new(9.0, 20.0)));
    }

    #[test]
    fn transform_applies_scale_and_translation() {
        let transform = Transform {
            scale_x: 2.0,
            scale_y: 3.0,
            translate_x: 4.0,
            translate_y: 5.0,
        };

        let point = transform.apply_to_point(Point::new(6.0, 7.0));

        assert_eq!(point, Point::new(16.0, 26.0));
    }
}
