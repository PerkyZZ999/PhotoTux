pub fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendModeMath {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
}

pub fn blend_rgba_over(
    destination: [u8; 4],
    source: [u8; 4],
    layer_opacity: f32,
    blend_mode: BlendModeMath,
) -> [u8; 4] {
    let source_alpha = (source[3] as f32 / 255.0) * layer_opacity.clamp(0.0, 1.0);
    if source_alpha <= 0.0 {
        return destination;
    }

    let destination_alpha = destination[3] as f32 / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if output_alpha <= 0.0 {
        return [0, 0, 0, 0];
    }

    let mut output = [0_u8; 4];
    for channel in 0..3 {
        let backdrop = destination[channel] as f32 / 255.0;
        let source_value = source[channel] as f32 / 255.0;
        let blended = blend_channel(backdrop, source_value, blend_mode);
        let premultiplied = (1.0 - source_alpha) * backdrop * destination_alpha
            + source_alpha * ((1.0 - destination_alpha) * source_value + destination_alpha * blended);
        let unmultiplied = (premultiplied / output_alpha).clamp(0.0, 1.0);
        output[channel] = (unmultiplied * 255.0).round() as u8;
    }

    output[3] = (output_alpha * 255.0).round() as u8;
    output
}

fn blend_channel(backdrop: f32, source: f32, blend_mode: BlendModeMath) -> f32 {
    match blend_mode {
        BlendModeMath::Normal => source,
        BlendModeMath::Multiply => backdrop * source,
        BlendModeMath::Screen => 1.0 - (1.0 - backdrop) * (1.0 - source),
        BlendModeMath::Overlay => {
            if backdrop <= 0.5 {
                2.0 * backdrop * source
            } else {
                1.0 - 2.0 * (1.0 - backdrop) * (1.0 - source)
            }
        }
        BlendModeMath::Darken => backdrop.min(source),
        BlendModeMath::Lighten => backdrop.max(source),
    }
}

#[cfg(test)]
mod tests {
    use super::{blend_rgba_over, BlendModeMath};

    #[test]
    fn normal_blend_over_transparent_matches_source() {
        let result = blend_rgba_over([0, 0, 0, 0], [200, 100, 50, 255], 1.0, BlendModeMath::Normal);
        assert_eq!(result, [200, 100, 50, 255]);
    }

    #[test]
    fn multiply_darkens_against_backdrop() {
        let result = blend_rgba_over([128, 128, 128, 255], [128, 64, 255, 255], 1.0, BlendModeMath::Multiply);
        assert_eq!(result, [64, 32, 128, 255]);
    }

    #[test]
    fn screen_lightens_against_backdrop() {
        let result = blend_rgba_over([64, 128, 32, 255], [128, 64, 128, 255], 1.0, BlendModeMath::Screen);
        assert!(result[0] > 128);
        assert!(result[1] > 128);
        assert!(result[2] > 128);
        assert_eq!(result[3], 255);
    }

    #[test]
    fn darken_and_lighten_choose_channel_extrema() {
        let darken = blend_rgba_over([90, 180, 45, 255], [100, 120, 90, 255], 1.0, BlendModeMath::Darken);
        let lighten = blend_rgba_over([90, 180, 45, 255], [100, 120, 90, 255], 1.0, BlendModeMath::Lighten);

        assert_eq!(darken, [90, 120, 45, 255]);
        assert_eq!(lighten, [100, 180, 90, 255]);
    }
}
