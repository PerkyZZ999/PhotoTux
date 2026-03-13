//! Color conversion and blend math scaffolding for PhotoTux.

fn blend_separable_rgba8(
    destination: [u8; 4],
    source: [u8; 4],
    layer_opacity: f32,
    blend_channel: impl Fn(f32, f32) -> f32,
) -> [u8; 4] {
    let source_alpha = (source[3] as f32 / 255.0) * layer_opacity.clamp(0.0, 1.0);
    let destination_alpha = destination[3] as f32 / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

    if output_alpha <= f32::EPSILON {
        return [0, 0, 0, 0];
    }

    let source_rgb = [
        source[0] as f32 / 255.0,
        source[1] as f32 / 255.0,
        source[2] as f32 / 255.0,
    ];
    let destination_rgb = [
        destination[0] as f32 / 255.0,
        destination[1] as f32 / 255.0,
        destination[2] as f32 / 255.0,
    ];
    let source_premultiplied = [
        source_rgb[0] * source_alpha,
        source_rgb[1] * source_alpha,
        source_rgb[2] * source_alpha,
    ];
    let destination_premultiplied = [
        destination_rgb[0] * destination_alpha,
        destination_rgb[1] * destination_alpha,
        destination_rgb[2] * destination_alpha,
    ];

    let output_premultiplied = [
        (1.0 - source_alpha) * destination_premultiplied[0]
            + (1.0 - destination_alpha) * source_premultiplied[0]
            + source_alpha * destination_alpha * blend_channel(source_rgb[0], destination_rgb[0]),
        (1.0 - source_alpha) * destination_premultiplied[1]
            + (1.0 - destination_alpha) * source_premultiplied[1]
            + source_alpha * destination_alpha * blend_channel(source_rgb[1], destination_rgb[1]),
        (1.0 - source_alpha) * destination_premultiplied[2]
            + (1.0 - destination_alpha) * source_premultiplied[2]
            + source_alpha * destination_alpha * blend_channel(source_rgb[2], destination_rgb[2]),
    ];

    [
        ((output_premultiplied[0] / output_alpha) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8,
        ((output_premultiplied[1] / output_alpha) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8,
        ((output_premultiplied[2] / output_alpha) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8,
        (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Blend a source pixel over a destination pixel using standard source-over composition.
///
/// The input pixels are straight-alpha RGBA8 values. Internally the blend is evaluated in
/// premultiplied form and converted back to straight alpha for storage.
#[must_use]
pub fn blend_normal_rgba8(destination: [u8; 4], source: [u8; 4], layer_opacity: f32) -> [u8; 4] {
    blend_separable_rgba8(destination, source, layer_opacity, |source_channel, _| {
        source_channel
    })
}

/// Blend a source pixel over a destination pixel using the Multiply blend mode.
#[must_use]
pub fn blend_multiply_rgba8(destination: [u8; 4], source: [u8; 4], layer_opacity: f32) -> [u8; 4] {
    blend_separable_rgba8(
        destination,
        source,
        layer_opacity,
        |source_channel, destination_channel| source_channel * destination_channel,
    )
}

/// Blend a source pixel over a destination pixel using the Screen blend mode.
#[must_use]
pub fn blend_screen_rgba8(destination: [u8; 4], source: [u8; 4], layer_opacity: f32) -> [u8; 4] {
    blend_separable_rgba8(
        destination,
        source,
        layer_opacity,
        |source_channel, destination_channel| {
            1.0 - (1.0 - source_channel) * (1.0 - destination_channel)
        },
    )
}

/// Blend a source pixel over a destination pixel using the Overlay blend mode.
#[must_use]
pub fn blend_overlay_rgba8(destination: [u8; 4], source: [u8; 4], layer_opacity: f32) -> [u8; 4] {
    blend_separable_rgba8(
        destination,
        source,
        layer_opacity,
        |source_channel, destination_channel| {
            if destination_channel <= 0.5 {
                2.0 * source_channel * destination_channel
            } else {
                1.0 - 2.0 * (1.0 - source_channel) * (1.0 - destination_channel)
            }
        },
    )
}

/// Blend a source pixel over a destination pixel using the Darken blend mode.
#[must_use]
pub fn blend_darken_rgba8(destination: [u8; 4], source: [u8; 4], layer_opacity: f32) -> [u8; 4] {
    blend_separable_rgba8(
        destination,
        source,
        layer_opacity,
        |source_channel, destination_channel| source_channel.min(destination_channel),
    )
}

/// Blend a source pixel over a destination pixel using the Lighten blend mode.
#[must_use]
pub fn blend_lighten_rgba8(destination: [u8; 4], source: [u8; 4], layer_opacity: f32) -> [u8; 4] {
    blend_separable_rgba8(
        destination,
        source,
        layer_opacity,
        |source_channel, destination_channel| source_channel.max(destination_channel),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        blend_darken_rgba8, blend_lighten_rgba8, blend_multiply_rgba8, blend_normal_rgba8,
        blend_overlay_rgba8, blend_screen_rgba8,
    };

    fn composite_normal_scene(layers: &[([u8; 4], f32)]) -> [u8; 4] {
        layers
            .iter()
            .fold([0, 0, 0, 0], |composited, (rgba, opacity)| {
                blend_normal_rgba8(composited, *rgba, *opacity)
            })
    }

    #[test]
    fn opaque_source_replaces_destination() {
        let output = blend_normal_rgba8([10, 20, 30, 255], [200, 150, 100, 255], 1.0);

        assert_eq!(output, [200, 150, 100, 255]);
    }

    #[test]
    fn transparent_source_leaves_destination_unchanged() {
        let output = blend_normal_rgba8([10, 20, 30, 255], [200, 150, 100, 0], 1.0);

        assert_eq!(output, [10, 20, 30, 255]);
    }

    #[test]
    fn source_alpha_composites_over_destination() {
        let output = blend_normal_rgba8([0, 0, 255, 255], [255, 0, 0, 128], 1.0);

        assert_eq!(output, [128, 0, 127, 255]);
    }

    #[test]
    fn layer_opacity_scales_source_contribution() {
        let output = blend_normal_rgba8([0, 0, 255, 255], [255, 0, 0, 255], 0.25);

        assert_eq!(output, [64, 0, 191, 255]);
    }

    #[test]
    fn multiply_with_opaque_source_and_destination_uses_channel_product() {
        let output = blend_multiply_rgba8([0, 0, 255, 255], [255, 0, 0, 255], 1.0);

        assert_eq!(output, [0, 0, 0, 255]);
    }

    #[test]
    fn multiply_respects_layer_opacity() {
        let output = blend_multiply_rgba8([128, 128, 128, 255], [255, 0, 0, 255], 0.5);

        assert_eq!(output, [128, 64, 64, 255]);
    }

    #[test]
    fn multiply_with_transparent_source_keeps_destination() {
        let output = blend_multiply_rgba8([30, 60, 90, 255], [255, 255, 255, 0], 1.0);

        assert_eq!(output, [30, 60, 90, 255]);
    }

    #[test]
    fn screen_with_opaque_source_and_destination_uses_screen_formula() {
        let output = blend_screen_rgba8([0, 0, 255, 255], [255, 0, 0, 255], 1.0);

        assert_eq!(output, [255, 0, 255, 255]);
    }

    #[test]
    fn screen_respects_layer_opacity() {
        let output = blend_screen_rgba8([128, 128, 128, 255], [255, 0, 0, 255], 0.5);

        assert_eq!(output, [192, 128, 128, 255]);
    }

    #[test]
    fn screen_with_transparent_source_keeps_destination() {
        let output = blend_screen_rgba8([30, 60, 90, 255], [255, 255, 255, 0], 1.0);

        assert_eq!(output, [30, 60, 90, 255]);
    }

    #[test]
    fn overlay_with_opaque_source_and_destination_uses_overlay_formula() {
        let output = blend_overlay_rgba8([64, 191, 128, 255], [255, 128, 32, 255], 1.0);

        assert_eq!(output, [128, 191, 33, 255]);
    }

    #[test]
    fn darken_with_opaque_source_and_destination_uses_channel_minimum() {
        let output = blend_darken_rgba8([200, 40, 120, 255], [100, 160, 90, 255], 1.0);

        assert_eq!(output, [100, 40, 90, 255]);
    }

    #[test]
    fn lighten_with_opaque_source_and_destination_uses_channel_maximum() {
        let output = blend_lighten_rgba8([200, 40, 120, 255], [100, 160, 90, 255], 1.0);

        assert_eq!(output, [200, 160, 120, 255]);
    }

    #[test]
    fn blend_mode_snapshot_matches_expected_rgba_outputs() {
        let destination = [64, 128, 192, 255];
        let source = [200, 100, 50, 255];

        assert_eq!(
            blend_normal_rgba8(destination, source, 1.0),
            [200, 100, 50, 255]
        );
        assert_eq!(
            blend_multiply_rgba8(destination, source, 1.0),
            [50, 50, 38, 255]
        );
        assert_eq!(
            blend_screen_rgba8(destination, source, 1.0),
            [214, 178, 204, 255]
        );
        assert_eq!(
            blend_overlay_rgba8(destination, source, 1.0),
            [100, 101, 154, 255]
        );
        assert_eq!(
            blend_darken_rgba8(destination, source, 1.0),
            [64, 100, 50, 255]
        );
        assert_eq!(
            blend_lighten_rgba8(destination, source, 1.0),
            [200, 128, 192, 255]
        );
    }

    #[test]
    fn alpha_regression_scene_preserves_straight_alpha_on_transparent_storage() {
        let stored_edge = composite_normal_scene(&[([255, 0, 0, 64], 1.0)]);

        assert_eq!(stored_edge, [255, 0, 0, 64]);
    }

    #[test]
    fn alpha_regression_scene_composites_cleanly_over_white_background() {
        let stored_edge = composite_normal_scene(&[([255, 0, 0, 64], 1.0)]);
        let over_white = blend_normal_rgba8([255, 255, 255, 255], stored_edge, 1.0);

        assert_eq!(over_white, [255, 191, 191, 255]);
    }

    #[test]
    fn alpha_regression_scene_composites_cleanly_over_black_background() {
        let stored_edge = composite_normal_scene(&[([255, 0, 0, 64], 1.0)]);
        let over_black = blend_normal_rgba8([0, 0, 0, 255], stored_edge, 1.0);

        assert_eq!(over_black, [64, 0, 0, 255]);
    }

    #[test]
    fn alpha_regression_scene_stacks_translucent_layers_without_dark_fringes() {
        let scene = composite_normal_scene(&[([255, 0, 0, 96], 1.0), ([0, 128, 255, 128], 1.0)]);

        assert_eq!(scene, [69, 93, 186, 176]);
    }

    #[test]
    fn alpha_regression_scene_zero_alpha_source_does_not_leak_rgb() {
        let scene = composite_normal_scene(&[([255, 0, 0, 0], 1.0), ([0, 0, 255, 128], 1.0)]);

        assert_eq!(scene, [0, 0, 255, 128]);
    }
}
