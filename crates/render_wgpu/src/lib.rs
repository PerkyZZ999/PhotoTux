//! WGPU-backed viewport rendering scaffolding for PhotoTux.

use common::{Point, Rect, Size, Vector};
use doc_model::{RasterSurface, SelectionBounds, TILE_SIZE, TileCoord};
use std::collections::BTreeMap;
use thiserror::Error;
use tracing::info;

/// Errors returned while bootstrapping the renderer.
#[derive(Debug, Error)]
pub enum RenderBootstrapError {
    /// No suitable adapter could be requested.
    #[error(transparent)]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    /// The logical device request failed.
    #[error(transparent)]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

/// Logical viewport size and scaling state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportSize {
    /// Logical width in editor units.
    pub logical_width: u32,
    /// Logical height in editor units.
    pub logical_height: u32,
    /// Current window scale factor.
    pub scale_factor: f32,
}

impl ViewportSize {
    /// Create a viewport size descriptor.
    #[must_use]
    pub const fn new(logical_width: u32, logical_height: u32, scale_factor: f32) -> Self {
        Self {
            logical_width,
            logical_height,
            scale_factor,
        }
    }

    /// Return the physical width after scale is applied.
    #[must_use]
    pub fn physical_width(self) -> u32 {
        ((self.logical_width as f32) * self.scale_factor.max(1.0)).round() as u32
    }

    /// Return the physical height after scale is applied.
    #[must_use]
    pub fn physical_height(self) -> u32 {
        ((self.logical_height as f32) * self.scale_factor.max(1.0)).round() as u32
    }
}

/// Scale-aware viewport navigation state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportState {
    /// Current viewport size.
    pub size: ViewportSize,
    /// Current zoom factor, where `1.0` means 1 document unit to 1 logical screen unit.
    pub zoom: f32,
    /// Pan offset in logical screen units.
    pub pan: Vector,
    /// Document size in document-space units.
    pub document_size: Size,
}

impl ViewportState {
    /// Minimum supported zoom level.
    pub const MIN_ZOOM: f32 = 0.125;
    /// Maximum supported zoom level.
    pub const MAX_ZOOM: f32 = 64.0;
    const ZOOM_STEP: f32 = 1.25;

    /// Create a new viewport state.
    #[must_use]
    pub fn new(size: ViewportSize, document_size: Size) -> Self {
        Self {
            size,
            zoom: 1.0,
            pan: Vector::new(0.0, 0.0),
            document_size,
        }
    }

    /// Increase zoom level by the default step.
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * Self::ZOOM_STEP).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    /// Decrease zoom level by the default step.
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / Self::ZOOM_STEP).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    /// Reset zoom and pan to the default view.
    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan = Vector::new(0.0, 0.0);
    }

    /// Apply a pan delta expressed in physical screen pixels.
    pub fn pan_by_screen_delta(&mut self, delta: Vector) {
        let scale_factor = self.size.scale_factor.max(1.0);
        self.pan.dx += delta.dx / scale_factor;
        self.pan.dy += delta.dy / scale_factor;
    }

    /// Convert a physical screen-space point into document space.
    #[must_use]
    pub fn screen_to_document(&self, screen_point: Point) -> Point {
        let scale_factor = self.size.scale_factor.max(1.0);
        let logical_point =
            Point::new(screen_point.x / scale_factor, screen_point.y / scale_factor);

        Point::new(
            (logical_point.x - self.pan.dx) / self.zoom,
            (logical_point.y - self.pan.dy) / self.zoom,
        )
    }

    /// Convert a document-space point into physical screen coordinates.
    #[must_use]
    pub fn document_to_screen(&self, document_point: Point) -> Point {
        let logical_x = document_point.x * self.zoom + self.pan.dx;
        let logical_y = document_point.y * self.zoom + self.pan.dy;
        let scale_factor = self.size.scale_factor.max(1.0);

        Point::new(logical_x * scale_factor, logical_y * scale_factor)
    }

    /// Update the tracked viewport size.
    pub fn update_size(&mut self, size: ViewportSize) {
        self.size = size;
    }

    /// Convert selection bounds in document space into a screen-space overlay rectangle.
    #[must_use]
    pub fn selection_overlay_rect(&self, bounds: SelectionBounds) -> SelectionOverlayRect {
        let top_left = self.document_to_screen(Point::new(bounds.x as f32, bounds.y as f32));
        let bottom_right = self.document_to_screen(Point::new(
            (bounds.x + bounds.width) as f32,
            (bounds.y + bounds.height) as f32,
        ));

        SelectionOverlayRect {
            screen_rect: Rect::new(
                top_left.x.min(bottom_right.x),
                top_left.y.min(bottom_right.y),
                (bottom_right.x - top_left.x).abs(),
                (bottom_right.y - top_left.y).abs(),
            ),
            fill_rgba: [79, 140, 255, 36],
            stroke_rgba: [116, 167, 255, 255],
            dash_length: 4.0,
            dash_gap: 4.0,
            stroke_width: 1.0,
        }
    }
}

/// Screen-space selection overlay geometry derived from document-space bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionOverlayRect {
    /// Screen-space rectangle in physical pixels.
    pub screen_rect: Rect,
    /// Fill color used for the selection interior.
    pub fill_rgba: [u8; 4],
    /// Stroke color used for the selection outline.
    pub stroke_rgba: [u8; 4],
    /// Dash segment length in physical pixels.
    pub dash_length: f32,
    /// Dash gap length in physical pixels.
    pub dash_gap: f32,
    /// Stroke width in physical pixels.
    pub stroke_width: f32,
}

/// CPU-side checkerboard sampling parameters for the viewport background.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckerboardBackground {
    /// Light square color.
    pub light_rgba: [u8; 4],
    /// Dark square color.
    pub dark_rgba: [u8; 4],
    /// Checker square size in pixels.
    pub cell_size: u32,
}

impl CheckerboardBackground {
    /// Create the default checkerboard background from the design tokens.
    #[must_use]
    pub const fn design_default() -> Self {
        Self {
            light_rgba: [0x50, 0x55, 0x5F, 0xFF],
            dark_rgba: [0x3E, 0x43, 0x4C, 0xFF],
            cell_size: 12,
        }
    }

    /// Sample the checkerboard color at a pixel coordinate.
    #[must_use]
    pub fn sample(self, x: u32, y: u32) -> [u8; 4] {
        let tile_x = x / self.cell_size.max(1);
        let tile_y = y / self.cell_size.max(1);

        if (tile_x + tile_y) % 2 == 0 {
            self.light_rgba
        } else {
            self.dark_rgba
        }
    }
}

/// Surface configuration that can be applied once a window-backed surface exists.
#[derive(Clone, Debug)]
pub struct SurfaceConfigState {
    /// WGPU surface configuration.
    pub config: wgpu::SurfaceConfiguration,
}

impl SurfaceConfigState {
    /// Create the default surface configuration for the active viewport.
    #[must_use]
    pub fn new(size: ViewportSize, format: wgpu::TextureFormat) -> Self {
        Self {
            config: wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.physical_width().max(1),
                height: size.physical_height().max(1),
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![format],
                desired_maximum_frame_latency: 2,
            },
        }
    }

    /// Update the surface configuration after a resize or scale-factor change.
    pub fn update_for_size(&mut self, size: ViewportSize) {
        self.config.width = size.physical_width().max(1);
        self.config.height = size.physical_height().max(1);
    }
}

/// Offscreen viewport texture state used for early smoke tests.
#[derive(Debug)]
pub struct ViewportSurface {
    size: ViewportSize,
    format: wgpu::TextureFormat,
    texture: wgpu::Texture,
}

impl ViewportSurface {
    fn new(device: &wgpu::Device, size: ViewportSize, format: wgpu::TextureFormat) -> Self {
        let texture = create_viewport_texture(device, size, format);

        Self {
            size,
            format,
            texture,
        }
    }

    /// Return the current viewport size.
    #[must_use]
    pub const fn size(&self) -> ViewportSize {
        self.size
    }

    /// Return the texture format.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Recreate the viewport texture after a resize.
    pub fn resize(&mut self, device: &wgpu::Device, new_size: ViewportSize) {
        self.size = new_size;
        self.texture = create_viewport_texture(device, new_size, self.format);
    }

    /// Create a texture view for render passes.
    #[must_use]
    pub fn create_view(&self) -> wgpu::TextureView {
        self.texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}

/// GPU-resident texture for an uploaded raster tile.
#[derive(Debug)]
pub struct GpuTile {
    /// Tile coordinate in the source raster surface.
    pub coord: TileCoord,
    texture: wgpu::Texture,
}

impl GpuTile {
    /// Create a texture view for the uploaded tile.
    #[must_use]
    pub fn create_view(&self) -> wgpu::TextureView {
        self.texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}

/// Cache of uploaded tile textures keyed by tile coordinate.
#[derive(Debug, Default)]
pub struct TileTextureCache {
    tiles: BTreeMap<TileCoord, GpuTile>,
}

impl TileTextureCache {
    /// Create an empty tile cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the number of uploaded tiles currently cached.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Return whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Upload all currently dirty tiles from a raster surface.
    pub fn upload_dirty_tiles(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &mut RasterSurface,
    ) {
        for coord in surface.take_dirty_tiles() {
            let Some(tile) = surface.tile(coord) else {
                continue;
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("phototux-raster-tile"),
                size: wgpu::Extent3d {
                    width: TILE_SIZE,
                    height: TILE_SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                tile.as_bytes(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(TILE_SIZE * 4),
                    rows_per_image: Some(TILE_SIZE),
                },
                wgpu::Extent3d {
                    width: TILE_SIZE,
                    height: TILE_SIZE,
                    depth_or_array_layers: 1,
                },
            );

            self.tiles.insert(coord, GpuTile { coord, texture });
        }
    }
}

/// Bootstrap state for the early PhotoTux renderer.
#[derive(Debug)]
pub struct RenderBootstrap {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    viewport_surface: ViewportSurface,
    surface_config: SurfaceConfigState,
    checkerboard: CheckerboardBackground,
}

impl RenderBootstrap {
    /// Create a headless renderer bootstrap using the noop backend for smoke tests.
    pub fn bootstrap_headless(size: ViewportSize) -> Result<Self, RenderBootstrapError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::NOOP,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions {
                noop: wgpu::NoopBackendOptions { enable: true },
                ..Default::default()
            },
        });

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("phototux-render-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            }))?;

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let viewport_surface = ViewportSurface::new(&device, size, format);
        let surface_config = SurfaceConfigState::new(size, format);
        let checkerboard = CheckerboardBackground::design_default();

        info!(target: "phototux::render_wgpu", width = size.logical_width, height = size.logical_height, scale_factor = size.scale_factor, "headless renderer bootstrap ready");

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            viewport_surface,
            surface_config,
            checkerboard,
        })
    }

    /// Return the active viewport surface state.
    #[must_use]
    pub fn viewport_surface(&self) -> &ViewportSurface {
        &self.viewport_surface
    }

    /// Return the current surface configuration.
    #[must_use]
    pub fn surface_config(&self) -> &SurfaceConfigState {
        &self.surface_config
    }

    /// Return the checkerboard background configuration.
    #[must_use]
    pub const fn checkerboard(&self) -> CheckerboardBackground {
        self.checkerboard
    }

    /// Handle a viewport resize event.
    pub fn resize(&mut self, logical_width: u32, logical_height: u32) {
        let new_size = ViewportSize::new(
            logical_width.max(1),
            logical_height.max(1),
            self.viewport_surface.size().scale_factor,
        );

        self.viewport_surface.resize(&self.device, new_size);
        self.surface_config.update_for_size(new_size);
    }

    /// Handle a scale-factor change event.
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        let current = self.viewport_surface.size();
        let new_size = ViewportSize::new(
            current.logical_width,
            current.logical_height,
            scale_factor.max(1.0),
        );

        self.viewport_surface.resize(&self.device, new_size);
        self.surface_config.update_for_size(new_size);
    }

    /// Clear the offscreen viewport texture to a neutral canvas-surround color.
    pub fn render_blank_viewport(&self) {
        let view = self.viewport_surface.create_view();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("phototux-blank-viewport-clear"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("phototux-blank-viewport-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.078,
                            g: 0.086,
                            b: 0.102,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
    }

    /// Return the selected adapter info.
    #[must_use]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    /// Poll all renderer devices.
    pub fn poll_devices(&self) {
        let _ = self.instance.poll_all(true);
    }
}

fn create_viewport_texture(
    device: &wgpu::Device,
    size: ViewportSize,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("phototux-viewport-surface"),
        size: wgpu::Extent3d {
            width: size.physical_width().max(1),
            height: size.physical_height().max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CheckerboardBackground, RenderBootstrap, TileTextureCache, ViewportSize, ViewportState,
    };
    use common::{Point, Rect, Size, Vector};
    use doc_model::{RasterSurface, SelectionBounds, TileCoord};

    #[test]
    fn bootstraps_headless_renderer() {
        let renderer = RenderBootstrap::bootstrap_headless(ViewportSize::new(640, 480, 1.0))
            .expect("renderer bootstrap should succeed");

        assert_eq!(renderer.viewport_surface().size().logical_width, 640);
        assert_eq!(renderer.surface_config().config.width, 640);
    }

    #[test]
    fn resize_updates_surface_and_config() {
        let mut renderer = RenderBootstrap::bootstrap_headless(ViewportSize::new(320, 240, 1.0))
            .expect("renderer bootstrap should succeed");

        renderer.resize(800, 600);

        assert_eq!(renderer.viewport_surface().size().logical_width, 800);
        assert_eq!(renderer.surface_config().config.width, 800);
        assert_eq!(renderer.surface_config().config.height, 600);
    }

    #[test]
    fn scale_factor_updates_physical_surface_size() {
        let mut renderer = RenderBootstrap::bootstrap_headless(ViewportSize::new(320, 240, 1.0))
            .expect("renderer bootstrap should succeed");

        renderer.set_scale_factor(2.0);

        assert_eq!(renderer.viewport_surface().size().physical_width(), 640);
        assert_eq!(renderer.surface_config().config.width, 640);
        assert_eq!(renderer.surface_config().config.height, 480);
    }

    #[test]
    fn checkerboard_samples_alternate_tiles() {
        let checkerboard = CheckerboardBackground::design_default();

        assert_eq!(checkerboard.sample(0, 0), checkerboard.light_rgba);
        assert_eq!(checkerboard.sample(13, 0), checkerboard.dark_rgba);
        assert_eq!(checkerboard.sample(13, 13), checkerboard.light_rgba);
    }

    #[test]
    fn blank_viewport_render_submits_without_panicking() {
        let renderer = RenderBootstrap::bootstrap_headless(ViewportSize::new(128, 128, 1.0))
            .expect("renderer bootstrap should succeed");

        renderer.render_blank_viewport();
        renderer.poll_devices();
    }

    #[test]
    fn viewport_zoom_in_and_out_changes_zoom_factor() {
        let mut viewport =
            ViewportState::new(ViewportSize::new(800, 600, 1.0), Size::new(1920.0, 1080.0));

        viewport.zoom_in();
        assert!(viewport.zoom > 1.0);

        viewport.zoom_out();
        assert_eq!(viewport.zoom, 1.0);
    }

    #[test]
    fn viewport_pan_uses_scale_aware_screen_delta() {
        let mut viewport =
            ViewportState::new(ViewportSize::new(800, 600, 2.0), Size::new(1920.0, 1080.0));

        viewport.pan_by_screen_delta(Vector::new(40.0, 20.0));

        assert_eq!(viewport.pan, Vector::new(20.0, 10.0));
    }

    #[test]
    fn coordinate_mapping_roundtrips_between_screen_and_document_space() {
        let mut viewport =
            ViewportState::new(ViewportSize::new(1000, 800, 1.5), Size::new(2048.0, 2048.0));
        viewport.zoom_in();
        viewport.pan = Vector::new(32.0, 48.0);

        let document_point = Point::new(120.0, 64.0);
        let screen_point = viewport.document_to_screen(document_point);
        let recovered_point = viewport.screen_to_document(screen_point);

        assert!((recovered_point.x - document_point.x).abs() < 0.001);
        assert!((recovered_point.y - document_point.y).abs() < 0.001);
    }

    #[test]
    fn reset_view_restores_default_zoom_and_pan() {
        let mut viewport =
            ViewportState::new(ViewportSize::new(800, 600, 1.25), Size::new(1024.0, 768.0));
        viewport.zoom_in();
        viewport.pan_by_screen_delta(Vector::new(50.0, 25.0));

        viewport.reset_view();

        assert_eq!(viewport.zoom, 1.0);
        assert_eq!(viewport.pan, Vector::new(0.0, 0.0));
    }

    #[test]
    fn uploads_dirty_tiles_into_gpu_cache() {
        let renderer = RenderBootstrap::bootstrap_headless(ViewportSize::new(256, 256, 1.0))
            .expect("renderer bootstrap should succeed");
        let mut cache = TileTextureCache::new();
        let mut surface = RasterSurface::new(512, 512);

        assert!(surface.write_pixel(10, 10, [255, 0, 0, 255]));
        assert!(surface.write_pixel(300, 300, [0, 255, 0, 255]));

        cache.upload_dirty_tiles(&renderer.device, &renderer.queue, &mut surface);

        assert_eq!(cache.len(), 2);
        assert!(surface.tile(TileCoord::new(0, 0)).is_some());
        assert_eq!(surface.dirty_tile_count(), 0);
    }

    #[test]
    fn selection_overlay_rect_maps_bounds_into_screen_space() {
        let mut viewport =
            ViewportState::new(ViewportSize::new(800, 600, 1.5), Size::new(1920.0, 1080.0));
        viewport.zoom = 2.0;
        viewport.pan = Vector::new(10.0, 20.0);

        let overlay = viewport.selection_overlay_rect(SelectionBounds::new(4, 5, 6, 8));

        assert_eq!(overlay.screen_rect, Rect::new(27.0, 45.0, 18.0, 24.0));
        assert_eq!(overlay.dash_length, 4.0);
        assert_eq!(overlay.dash_gap, 4.0);
        assert_eq!(overlay.stroke_width, 1.0);
    }

    #[test]
    fn selection_overlay_rect_keeps_screen_dash_metrics_stable_across_zoom() {
        let mut viewport =
            ViewportState::new(ViewportSize::new(800, 600, 1.0), Size::new(1024.0, 768.0));
        let baseline = viewport.selection_overlay_rect(SelectionBounds::new(10, 10, 20, 20));

        viewport.zoom_in();
        let zoomed = viewport.selection_overlay_rect(SelectionBounds::new(10, 10, 20, 20));

        assert!(zoomed.screen_rect.width > baseline.screen_rect.width);
        assert_eq!(zoomed.dash_length, baseline.dash_length);
        assert_eq!(zoomed.dash_gap, baseline.dash_gap);
    }
}
