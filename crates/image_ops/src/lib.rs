use common::{CanvasRect, DestructiveFilterKind};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushTileContext {
    pub tile_size: u32,
    pub tile_origin_x: i32,
    pub tile_origin_y: i32,
    pub center_x: f32,
    pub center_y: f32,
    pub clip_rect: Option<CanvasRect>,
    pub clip_inverted: bool,
}

impl BrushTileContext {
    pub fn new(
        tile_size: u32,
        tile_origin_x: i32,
        tile_origin_y: i32,
        center_x: f32,
        center_y: f32,
    ) -> Self {
        Self {
            tile_size,
            tile_origin_x,
            tile_origin_y,
            center_x,
            center_y,
            clip_rect: None,
            clip_inverted: false,
        }
    }

    pub fn with_clip(mut self, clip_rect: Option<CanvasRect>, clip_inverted: bool) -> Self {
        self.clip_rect = clip_rect;
        self.clip_inverted = clip_inverted;
        self
    }
}

pub fn apply_round_brush_dab(
    tile_pixels: &mut [u8],
    context: BrushTileContext,
    dab: BrushDab,
) -> bool {
    apply_round_brush_dab_clipped(tile_pixels, context.with_clip(None, false), dab)
}

pub fn apply_round_mask_hide_dab_clipped(
    tile_alpha: &mut [u8],
    context: BrushTileContext,
    dab: BrushDab,
) -> bool {
    apply_round_mask_dab(tile_alpha, context, dab, false)
}

pub fn apply_round_mask_reveal_dab_clipped(
    tile_alpha: &mut [u8],
    context: BrushTileContext,
    dab: BrushDab,
) -> bool {
    apply_round_mask_dab(tile_alpha, context, dab, true)
}

pub fn apply_round_brush_dab_clipped(
    tile_pixels: &mut [u8],
    context: BrushTileContext,
    dab: BrushDab,
) -> bool {
    apply_round_dab(tile_pixels, context, dab, BrushBlendMode::Paint)
}

pub fn apply_round_eraser_dab(
    tile_pixels: &mut [u8],
    context: BrushTileContext,
    dab: BrushDab,
) -> bool {
    apply_round_eraser_dab_clipped(tile_pixels, context.with_clip(None, false), dab)
}

pub fn apply_round_eraser_dab_clipped(
    tile_pixels: &mut [u8],
    context: BrushTileContext,
    dab: BrushDab,
) -> bool {
    apply_round_dab(tile_pixels, context, dab, BrushBlendMode::Erase)
}

pub fn apply_destructive_filter_rgba(pixels: &mut [u8], filter: DestructiveFilterKind) -> bool {
    if !pixels.len().is_multiple_of(4) {
        return false;
    }

    let mut changed = false;
    for rgba in pixels.chunks_exact_mut(4) {
        if rgba[3] == 0 {
            continue;
        }

        let before = [rgba[0], rgba[1], rgba[2]];
        match filter {
            DestructiveFilterKind::InvertColors => {
                rgba[0] = 255_u8.saturating_sub(rgba[0]);
                rgba[1] = 255_u8.saturating_sub(rgba[1]);
                rgba[2] = 255_u8.saturating_sub(rgba[2]);
            }
            DestructiveFilterKind::Desaturate => {
                let luminance =
                    (rgba[0] as f32 * 0.299 + rgba[1] as f32 * 0.587 + rgba[2] as f32 * 0.114)
                        .round() as u8;
                rgba[0] = luminance;
                rgba[1] = luminance;
                rgba[2] = luminance;
            }
        }

        changed |= before != [rgba[0], rgba[1], rgba[2]];
    }

    changed
}

fn apply_round_dab(
    tile_pixels: &mut [u8],
    context: BrushTileContext,
    dab: BrushDab,
    blend_mode: BrushBlendMode,
) -> bool {
    let Some(setup) = RoundDabSetup::new(context, dab, tile_pixels.len(), 4) else {
        return false;
    };
    let mut changed = false;

    for canvas_y in setup.start_y..=setup.end_y {
        for canvas_x in setup.start_x..=setup.end_x {
            let Some(coverage) = setup.coverage_at(canvas_x, canvas_y) else {
                continue;
            };
            let alpha =
                (dab.color[3] as f32 / 255.0) * dab.opacity * dab.flow * coverage.clamp(0.0, 1.0);

            if alpha <= 0.0 {
                continue;
            }

            let index = setup.pixel_index(canvas_x, canvas_y, 4);

            blend_pixel(
                &mut tile_pixels[index..index + 4],
                dab.color,
                alpha,
                blend_mode,
            );
            changed = true;
        }
    }

    changed
}

fn apply_round_mask_dab(
    tile_alpha: &mut [u8],
    context: BrushTileContext,
    dab: BrushDab,
    reveal: bool,
) -> bool {
    let Some(setup) = RoundDabSetup::new(context, dab, tile_alpha.len(), 1) else {
        return false;
    };
    let mut changed = false;

    for canvas_y in setup.start_y..=setup.end_y {
        for canvas_x in setup.start_x..=setup.end_x {
            let Some(coverage) = setup.coverage_at(canvas_x, canvas_y) else {
                continue;
            };
            let alpha = (dab.opacity * dab.flow * coverage.clamp(0.0, 1.0)).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }

            let index = setup.pixel_index(canvas_x, canvas_y, 1);
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

#[derive(Debug, Clone, Copy)]
struct RoundDabSetup {
    tile_size: u32,
    tile_origin_x: i32,
    tile_origin_y: i32,
    center_x: f32,
    center_y: f32,
    clip_rect: Option<CanvasRect>,
    clip_inverted: bool,
    radius: f32,
    hard_radius: f32,
    soft_width: f32,
    start_x: i32,
    end_x: i32,
    start_y: i32,
    end_y: i32,
}

impl RoundDabSetup {
    fn new(
        context: BrushTileContext,
        dab: BrushDab,
        buffer_len: usize,
        pixel_stride: usize,
    ) -> Option<Self> {
        let BrushTileContext {
            tile_size,
            tile_origin_x,
            tile_origin_y,
            center_x,
            center_y,
            clip_rect,
            clip_inverted,
        } = context;
        if buffer_len != tile_size as usize * tile_size as usize * pixel_stride || dab.radius <= 0.0
        {
            return None;
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
            return None;
        }

        Some(Self {
            tile_size,
            tile_origin_x,
            tile_origin_y,
            center_x,
            center_y,
            clip_rect,
            clip_inverted,
            radius: dab.radius,
            hard_radius: dab.radius * dab.hardness,
            soft_width: (dab.radius - dab.radius * dab.hardness).max(0.000_1),
            start_x: ((center_x - dab.radius).floor().max(tile_min_x)) as i32,
            end_x: ((center_x + dab.radius).ceil().min(tile_max_x - 1.0)) as i32,
            start_y: ((center_y - dab.radius).floor().max(tile_min_y)) as i32,
            end_y: ((center_y + dab.radius).ceil().min(tile_max_y - 1.0)) as i32,
        })
    }

    fn coverage_at(&self, canvas_x: i32, canvas_y: i32) -> Option<f32> {
        if !pixel_is_within_clip(canvas_x, canvas_y, self.clip_rect, self.clip_inverted) {
            return None;
        }

        let pixel_center_x = canvas_x as f32 + 0.5;
        let pixel_center_y = canvas_y as f32 + 0.5;
        let delta_x = pixel_center_x - self.center_x;
        let delta_y = pixel_center_y - self.center_y;
        let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();

        if distance > self.radius {
            return None;
        }

        if distance <= self.hard_radius {
            return Some(1.0);
        }

        let t = ((distance - self.hard_radius) / self.soft_width).clamp(0.0, 1.0);
        Some(1.0 - (t * t * (3.0 - 2.0 * t)))
    }

    fn pixel_index(&self, canvas_x: i32, canvas_y: i32, pixel_stride: usize) -> usize {
        let local_x = (canvas_x - self.tile_origin_x) as usize;
        let local_y = (canvas_y - self.tile_origin_y) as usize;
        (local_y * self.tile_size as usize + local_x) * pixel_stride
    }
}

fn pixel_is_within_clip(
    pixel_x: i32,
    pixel_y: i32,
    clip_rect: Option<CanvasRect>,
    clip_inverted: bool,
) -> bool {
    let Some(clip_rect) = clip_rect else {
        return true;
    };

    let right = clip_rect.x + clip_rect.width as i32;
    let bottom = clip_rect.y + clip_rect.height as i32;
    let inside =
        pixel_x >= clip_rect.x && pixel_x < right && pixel_y >= clip_rect.y && pixel_y < bottom;
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
        BrushDab, BrushTileContext, apply_destructive_filter_rgba, apply_round_brush_dab,
        apply_round_brush_dab_clipped, apply_round_eraser_dab, apply_round_eraser_dab_clipped,
        apply_round_mask_hide_dab_clipped, apply_round_mask_reveal_dab_clipped,
    };
    use common::{CanvasRect, DestructiveFilterKind};

    #[test]
    fn brush_dab_changes_pixels_inside_radius() {
        let mut pixels = vec![0_u8; 16 * 16 * 4];
        let changed = apply_round_brush_dab(
            &mut pixels,
            BrushTileContext::new(16, 0, 0, 8.0, 8.0),
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
            BrushTileContext::new(16, 0, 0, 40.0, 40.0),
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
            BrushTileContext::new(16, 16, 0, 18.0, 4.0),
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
            BrushTileContext::new(16, 0, 0, 8.0, 8.0),
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
            BrushTileContext::new(16, 0, 0, 8.0, 8.0),
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
            BrushTileContext::new(16, 0, 0, 8.0, 8.0)
                .with_clip(Some(CanvasRect::new(6, 6, 4, 4)), false),
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [255, 0, 0, 255]),
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
            BrushTileContext::new(16, 0, 0, 8.0, 8.0)
                .with_clip(Some(CanvasRect::new(6, 6, 4, 4)), true),
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [0, 0, 0, 255]),
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
            BrushTileContext::new(16, 0, 0, 8.0, 8.0),
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [0, 0, 0, 255]),
        );

        assert!(changed);
        let center = 8 * 16 + 8;
        assert_eq!(alpha[center], 0);

        let changed = apply_round_mask_reveal_dab_clipped(
            &mut alpha,
            BrushTileContext::new(16, 0, 0, 8.0, 8.0),
            BrushDab::new(4.0, 1.0, 1.0, 1.0, [0, 0, 0, 255]),
        );

        assert!(changed);
        assert_eq!(alpha[center], 255);
    }

    #[test]
    fn invert_filter_changes_visible_rgb_without_touching_alpha() {
        let mut pixels = vec![10_u8, 20, 30, 255, 0, 0, 0, 0];

        let changed =
            apply_destructive_filter_rgba(&mut pixels, DestructiveFilterKind::InvertColors);

        assert!(changed);
        assert_eq!(&pixels[0..4], &[245, 235, 225, 255]);
        assert_eq!(&pixels[4..8], &[0, 0, 0, 0]);
    }

    #[test]
    fn desaturate_filter_converts_visible_pixels_to_luminance() {
        let mut pixels = vec![50_u8, 150, 200, 255];

        let changed = apply_destructive_filter_rgba(&mut pixels, DestructiveFilterKind::Desaturate);

        assert!(changed);
        assert_eq!(&pixels[0..3], &[126, 126, 126]);
        assert_eq!(pixels[3], 255);
    }
}
