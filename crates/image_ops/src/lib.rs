pub fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushDab {
    pub radius: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub color: [u8; 4],
}

impl BrushDab {
    pub fn new(radius: f32, hardness: f32, opacity: f32, color: [u8; 4]) -> Self {
        Self {
            radius,
            hardness: hardness.clamp(0.0, 1.0),
            opacity: opacity.clamp(0.0, 1.0),
            color,
        }
    }
}

pub fn apply_round_brush_dab(
    tile_pixels: &mut [u8],
    tile_size: u32,
    tile_origin_x: u32,
    tile_origin_y: u32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
) -> bool {
    if tile_pixels.len() != tile_size as usize * tile_size as usize * 4 || dab.radius <= 0.0 {
        return false;
    }

    let tile_min_x = tile_origin_x as f32;
    let tile_min_y = tile_origin_y as f32;
    let tile_max_x = tile_min_x + tile_size as f32;
    let tile_max_y = tile_min_y + tile_size as f32;

    if center_x + dab.radius < tile_min_x
        || center_y + dab.radius < tile_min_y
        || center_x - dab.radius >= tile_max_x
        || center_y - dab.radius >= tile_max_y
    {
        return false;
    }

    let start_x = ((center_x - dab.radius).floor().max(tile_min_x)) as u32;
    let end_x = ((center_x + dab.radius).ceil().min(tile_max_x - 1.0)) as u32;
    let start_y = ((center_y - dab.radius).floor().max(tile_min_y)) as u32;
    let end_y = ((center_y + dab.radius).ceil().min(tile_max_y - 1.0)) as u32;

    let hard_radius = dab.radius * dab.hardness;
    let soft_width = (dab.radius - hard_radius).max(0.000_1);
    let mut changed = false;

    for canvas_y in start_y..=end_y {
        for canvas_x in start_x..=end_x {
            let pixel_center_x = canvas_x as f32 + 0.5;
            let pixel_center_y = canvas_y as f32 + 0.5;
            let delta_x = pixel_center_x - center_x;
            let delta_y = pixel_center_y - center_y;
            let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();

            if distance > dab.radius {
                continue;
            }

            let coverage = if distance <= hard_radius {
                1.0
            } else {
                1.0 - ((distance - hard_radius) / soft_width)
            };
            let alpha = (dab.color[3] as f32 / 255.0) * dab.opacity * coverage.clamp(0.0, 1.0);

            if alpha <= 0.0 {
                continue;
            }

            let local_x = (canvas_x - tile_origin_x) as usize;
            let local_y = (canvas_y - tile_origin_y) as usize;
            let index = (local_y * tile_size as usize + local_x) * 4;

            blend_pixel(&mut tile_pixels[index..index + 4], dab.color, alpha);
            changed = true;
        }
    }

    changed
}

fn blend_pixel(destination: &mut [u8], source: [u8; 4], alpha: f32) {
    let source_alpha = alpha.clamp(0.0, 1.0);
    let destination_alpha = destination[3] as f32 / 255.0;
    let out_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);

    for channel in 0..3 {
        let src = source[channel] as f32 / 255.0;
        let dst = destination[channel] as f32 / 255.0;
        let out = src * source_alpha + dst * (1.0 - source_alpha);
        destination[channel] = clamp_u8((out * 255.0).round() as i32);
    }

    destination[3] = clamp_u8((out_alpha * 255.0).round() as i32);
}

#[cfg(test)]
mod tests {
    use super::{apply_round_brush_dab, BrushDab};

    #[test]
    fn brush_dab_changes_pixels_inside_radius() {
        let mut pixels = vec![0_u8; 16 * 16 * 4];
        let changed = apply_round_brush_dab(
            &mut pixels,
            16,
            0,
            0,
            8.0,
            8.0,
            BrushDab::new(4.0, 1.0, 1.0, [255, 0, 0, 255]),
        );

        assert!(changed);
        let center = (8 * 16 + 8) * 4;
        assert_eq!(&pixels[center..center + 4], &[255, 0, 0, 255]);
    }

    #[test]
    fn brush_dab_leaves_non_intersecting_tile_unchanged() {
        let mut pixels = vec![0_u8; 16 * 16 * 4];
        let changed = apply_round_brush_dab(
            &mut pixels,
            16,
            0,
            0,
            40.0,
            40.0,
            BrushDab::new(3.0, 1.0, 1.0, [255, 0, 0, 255]),
        );

        assert!(!changed);
        assert!(pixels.iter().all(|value| *value == 0));
    }

    #[test]
    fn brush_dab_respects_tile_origin() {
        let mut pixels = vec![0_u8; 16 * 16 * 4];
        let changed = apply_round_brush_dab(
            &mut pixels,
            16,
            16,
            0,
            18.0,
            4.0,
            BrushDab::new(2.0, 1.0, 1.0, [0, 255, 0, 255]),
        );

        assert!(changed);
        let local_index = (4 * 16 + 2) * 4;
        assert_eq!(&pixels[local_index..local_index + 4], &[0, 255, 0, 255]);
    }

    #[test]
    fn brush_dab_soft_edge_produces_partial_alpha() {
        let mut pixels = vec![0_u8; 16 * 16 * 4];
        let changed = apply_round_brush_dab(
            &mut pixels,
            16,
            0,
            0,
            8.0,
            8.0,
            BrushDab::new(5.0, 0.25, 1.0, [255, 255, 255, 255]),
        );

        assert!(changed);
        let edge_index = (8 * 16 + 12) * 4 + 3;
        assert!(pixels[edge_index] > 0);
        assert!(pixels[edge_index] < 255);
    }
}
