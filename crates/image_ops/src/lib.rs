use common::CanvasRect;

pub fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushBlendMode {
    Paint,
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushDab {
    pub radius: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub flow: f32,
    pub color: [u8; 4],
}

impl BrushDab {
    pub fn new(radius: f32, hardness: f32, opacity: f32, flow: f32, color: [u8; 4]) -> Self {
        Self {
            radius,
            hardness: hardness.clamp(0.0, 1.0),
            opacity: opacity.clamp(0.0, 1.0),
            flow: flow.clamp(0.0, 1.0),
            color,
        }
    }
}

pub fn apply_round_brush_dab(
    tile_pixels: &mut [u8],
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
) -> bool {
    apply_round_brush_dab_clipped(
        tile_pixels,
        tile_size,
        tile_origin_x,
        tile_origin_y,
        center_x,
        center_y,
        dab,
        None,
        false,
    )
}

pub fn apply_round_mask_hide_dab_clipped(
    tile_alpha: &mut [u8],
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
    clip_rect: Option<CanvasRect>,
    clip_inverted: bool,
) -> bool {
    apply_round_mask_dab(
        tile_alpha,
        tile_size,
        tile_origin_x,
        tile_origin_y,
        center_x,
        center_y,
        dab,
        false,
        clip_rect,
        clip_inverted,
    )
}

pub fn apply_round_mask_reveal_dab_clipped(
    tile_alpha: &mut [u8],
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
    clip_rect: Option<CanvasRect>,
    clip_inverted: bool,
) -> bool {
    apply_round_mask_dab(
        tile_alpha,
        tile_size,
        tile_origin_x,
        tile_origin_y,
        center_x,
        center_y,
        dab,
        true,
        clip_rect,
        clip_inverted,
    )
}

pub fn apply_round_brush_dab_clipped(
    tile_pixels: &mut [u8],
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
    clip_rect: Option<CanvasRect>,
    clip_inverted: bool,
) -> bool {
    apply_round_dab(
        tile_pixels,
        tile_size,
        tile_origin_x,
        tile_origin_y,
        center_x,
        center_y,
        dab,
        BrushBlendMode::Paint,
        clip_rect,
        clip_inverted,
    )
}

pub fn apply_round_eraser_dab(
    tile_pixels: &mut [u8],
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
) -> bool {
    apply_round_eraser_dab_clipped(
        tile_pixels,
        tile_size,
        tile_origin_x,
        tile_origin_y,
        center_x,
        center_y,
        dab,
        None,
        false,
    )
}

pub fn apply_round_eraser_dab_clipped(
    tile_pixels: &mut [u8],
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
    clip_rect: Option<CanvasRect>,
    clip_inverted: bool,
) -> bool {
    apply_round_dab(
        tile_pixels,
        tile_size,
        tile_origin_x,
        tile_origin_y,
        center_x,
        center_y,
        dab,
        BrushBlendMode::Erase,
        clip_rect,
        clip_inverted,
    )
}

fn apply_round_dab(
    tile_pixels: &mut [u8],
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
    blend_mode: BrushBlendMode,
    clip_rect: Option<CanvasRect>,
    clip_inverted: bool,
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

    let start_x = ((center_x - dab.radius).floor().max(tile_min_x)) as i32;
    let end_x = ((center_x + dab.radius).ceil().min(tile_max_x - 1.0)) as i32;
    let start_y = ((center_y - dab.radius).floor().max(tile_min_y)) as i32;
    let end_y = ((center_y + dab.radius).ceil().min(tile_max_y - 1.0)) as i32;

    let hard_radius = dab.radius * dab.hardness;
    let soft_width = (dab.radius - hard_radius).max(0.000_1);
    let mut changed = false;

    for canvas_y in start_y..=end_y {
        for canvas_x in start_x..=end_x {
            if !pixel_is_within_clip(canvas_x, canvas_y, clip_rect, clip_inverted) {
                continue;
            }

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
                let t = ((distance - hard_radius) / soft_width).clamp(0.0, 1.0);
                1.0 - (t * t * (3.0 - 2.0 * t))
            };
            let alpha =
                (dab.color[3] as f32 / 255.0) * dab.opacity * dab.flow * coverage.clamp(0.0, 1.0);

            if alpha <= 0.0 {
                continue;
            }

            let local_x = (canvas_x - tile_origin_x) as usize;
            let local_y = (canvas_y - tile_origin_y) as usize;
            let index = (local_y * tile_size as usize + local_x) * 4;

            blend_pixel(&mut tile_pixels[index..index + 4], dab.color, alpha, blend_mode);
            changed = true;
        }
    }

    changed
}

fn apply_round_mask_dab(
    tile_alpha: &mut [u8],
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    dab: BrushDab,
    reveal: bool,
    clip_rect: Option<CanvasRect>,
    clip_inverted: bool,
) -> bool {
    if tile_alpha.len() != tile_size as usize * tile_size as usize || dab.radius <= 0.0 {
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

    let start_x = ((center_x - dab.radius).floor().max(tile_min_x)) as i32;
    let end_x = ((center_x + dab.radius).ceil().min(tile_max_x - 1.0)) as i32;
    let start_y = ((center_y - dab.radius).floor().max(tile_min_y)) as i32;
    let end_y = ((center_y + dab.radius).ceil().min(tile_max_y - 1.0)) as i32;

    let hard_radius = dab.radius * dab.hardness;
    let soft_width = (dab.radius - hard_radius).max(0.000_1);
    let mut changed = false;

    for canvas_y in start_y..=end_y {
        for canvas_x in start_x..=end_x {
            if !pixel_is_within_clip(canvas_x, canvas_y, clip_rect, clip_inverted) {
                continue;
            }

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
                let t = ((distance - hard_radius) / soft_width).clamp(0.0, 1.0);
                1.0 - (t * t * (3.0 - 2.0 * t))
            };
            let alpha = (dab.opacity * dab.flow * coverage.clamp(0.0, 1.0)).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }

            let local_x = (canvas_x - tile_origin_x) as usize;
            let local_y = (canvas_y - tile_origin_y) as usize;
            let index = local_y * tile_size as usize + local_x;
            let before = tile_alpha[index];
            let target = if reveal { 255.0 } else { 0.0 };
            let after = (before as f32 * (1.0 - alpha) + target * alpha)
                .round()
                .clamp(0.0, 255.0) as u8;

            if after != before {
                tile_alpha[index] = after;
                changed = true;
            }
        }
    }

    changed
}

fn pixel_is_within_clip(pixel_x: i32, pixel_y: i32, clip_rect: Option<CanvasRect>, clip_inverted: bool) -> bool {
    let Some(clip_rect) = clip_rect else {
        return true;
    };

    let right = clip_rect.x + clip_rect.width as i32;
    let bottom = clip_rect.y + clip_rect.height as i32;
    let inside = pixel_x >= clip_rect.x && pixel_x < right && pixel_y >= clip_rect.y && pixel_y < bottom;
    inside != clip_inverted
}

fn blend_pixel(destination: &mut [u8], source: [u8; 4], alpha: f32, blend_mode: BrushBlendMode) {
    if matches!(blend_mode, BrushBlendMode::Erase) {
        erase_pixel(destination, alpha);
        return;
    }

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

fn erase_pixel(destination: &mut [u8], alpha: f32) {
    let retain = (1.0 - alpha.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    for channel in destination.iter_mut() {
        *channel = clamp_u8((*channel as f32 * retain).round() as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_round_brush_dab, apply_round_brush_dab_clipped, apply_round_eraser_dab,
        apply_round_eraser_dab_clipped, apply_round_mask_hide_dab_clipped,
        apply_round_mask_reveal_dab_clipped, BrushDab,
    };
    use common::CanvasRect;

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
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [255, 0, 0, 255]),
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
            BrushDab::new(3.0, 1.0, 1.0, 1.0, [255, 0, 0, 255]),
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
            BrushDab::new(2.0, 1.0, 1.0, 1.0, [0, 255, 0, 255]),
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
            BrushDab::new(5.0, 0.25, 1.0, 1.0, [255, 255, 255, 255]),
        );

        assert!(changed);
        let edge_index = (8 * 16 + 12) * 4 + 3;
        assert!(pixels[edge_index] > 0);
        assert!(pixels[edge_index] < 255);
    }

    #[test]
    fn eraser_dab_reduces_existing_alpha() {
        let mut pixels = vec![255_u8; 16 * 16 * 4];
        let changed = apply_round_eraser_dab(
            &mut pixels,
            16,
            0,
            0,
            8.0,
            8.0,
            BrushDab::new(3.0, 1.0, 0.5, 1.0, [0, 0, 0, 255]),
        );

        assert!(changed);
        let center_alpha = pixels[(8 * 16 + 8) * 4 + 3];
        assert!(center_alpha < 255);
    }

    #[test]
    fn clipped_brush_dab_only_affects_pixels_inside_selection() {
        let mut pixels = vec![0_u8; 16 * 16 * 4];
        let changed = apply_round_brush_dab_clipped(
            &mut pixels,
            16,
            0,
            0,
            8.0,
            8.0,
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [255, 0, 0, 255]),
            Some(CanvasRect::new(6, 6, 4, 4)),
            false,
        );

        assert!(changed);
        let inside = (8 * 16 + 8) * 4 + 3;
        let outside = (8 * 16 + 4) * 4 + 3;
        assert!(pixels[inside] > 0);
        assert_eq!(pixels[outside], 0);
    }

    #[test]
    fn clipped_eraser_dab_respects_inverted_selection() {
        let mut pixels = vec![255_u8; 16 * 16 * 4];
        let changed = apply_round_eraser_dab_clipped(
            &mut pixels,
            16,
            0,
            0,
            8.0,
            8.0,
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [0, 0, 0, 255]),
            Some(CanvasRect::new(6, 6, 4, 4)),
            true,
        );

        assert!(changed);
        let inside = (8 * 16 + 8) * 4 + 3;
        let outside = (8 * 16 + 4) * 4 + 3;
        assert_eq!(pixels[inside], 255);
        assert!(pixels[outside] < 255);
    }

    #[test]
    fn mask_hide_and_reveal_adjust_alpha() {
        let mut alpha = vec![255_u8; 16 * 16];
        let changed = apply_round_mask_hide_dab_clipped(
            &mut alpha,
            16,
            0,
            0,
            8.0,
            8.0,
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [0, 0, 0, 255]),
            None,
            false,
        );

        assert!(changed);
        let center = 8 * 16 + 8;
        assert_eq!(alpha[center], 0);

        let changed = apply_round_mask_reveal_dab_clipped(
            &mut alpha,
            16,
            0,
            0,
            8.0,
            8.0,
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [0, 0, 0, 255]),
            None,
            false,
        );

        assert!(changed);
        assert_eq!(alpha[center], 255);
    }
}
