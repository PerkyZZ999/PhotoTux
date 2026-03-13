//! Shared numeric helpers.

/// Clamp a floating-point value into a range.
#[must_use]
pub fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    debug_assert!(min <= max, "minimum bound must be <= maximum bound");
    value.clamp(min, max)
}

/// Linearly interpolate between two floating-point values.
#[must_use]
pub fn lerp_f32(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

/// Convert a floating-point coordinate into a pixel-aligned integer.
#[must_use]
pub fn round_to_pixel(value: f32) -> i32 {
    value.round() as i32
}

#[cfg(test)]
mod tests {
    use super::{clamp_f32, lerp_f32, round_to_pixel};

    #[test]
    fn clamp_limits_value_into_range() {
        assert_eq!(clamp_f32(-2.0, 0.0, 1.0), 0.0);
        assert_eq!(clamp_f32(0.5, 0.0, 1.0), 0.5);
        assert_eq!(clamp_f32(4.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn lerp_interpolates_between_bounds() {
        assert_eq!(lerp_f32(10.0, 20.0, 0.0), 10.0);
        assert_eq!(lerp_f32(10.0, 20.0, 0.5), 15.0);
        assert_eq!(lerp_f32(10.0, 20.0, 1.0), 20.0);
    }

    #[test]
    fn round_to_pixel_uses_nearest_integer() {
        assert_eq!(round_to_pixel(12.4), 12);
        assert_eq!(round_to_pixel(12.5), 13);
    }
}
