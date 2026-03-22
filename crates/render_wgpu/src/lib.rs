use common::{CanvasRaster, CanvasRect, CanvasSize};
use std::num::NonZeroU64;

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportRendererConfig {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl Default for ViewportRendererConfig {
    fn default() -> Self {
        Self {
            width: 1600,
            height: 900,
            scale_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportSize {
    pub width: f32,
    pub height: f32,
}

impl ViewportSize {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportState {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

pub struct CanvasFrame {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasOverlayRect {
    pub rect: CanvasRect,
    pub stroke_rgba: [u8; 4],
    pub fill_rgba: Option<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasOverlayPath {
    pub points: Vec<(i32, i32)>,
    pub stroke_rgba: [u8; 4],
    pub closed: bool,
}

pub struct OffscreenCanvasRenderer {
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CanvasUniforms {
    viewport_size: [f32; 2],
    canvas_size: [f32; 2],
    zoom: f32,
    _pad0: f32,
    pan: [f32; 2],
    _pad1: [f32; 2],
}

impl OffscreenCanvasRenderer {
    pub async fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .context("failed to find a GPU adapter for PhotoTux")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("PhotoTux Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("failed to create a logical wgpu device")?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PhotoTux Canvas Shader"),
            source: wgpu::ShaderSource::Wgsl(CANVAS_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PhotoTux Canvas Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<CanvasUniforms>() as u64),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PhotoTux Canvas Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PhotoTux Canvas Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            instance,
            device,
            queue,
            render_pipeline,
            bind_group_layout,
        })
    }

    pub fn new_blocking() -> Result<Self> {
        pollster::block_on(Self::new())
    }

    pub fn render(
        &self,
        canvas_size: CanvasSize,
        viewport_state: ViewportState,
        logical_width: u32,
        logical_height: u32,
        scale_factor: f64,
        canvas_raster: Option<&CanvasRaster>,
        overlays: &[CanvasOverlayRect],
        overlay_paths: &[CanvasOverlayPath],
    ) -> Result<CanvasFrame> {
        let physical_width = ((logical_width as f64 * scale_factor).round() as u32).max(1);
        let physical_height = ((logical_height as f64 * scale_factor).round() as u32).max(1);
        let bytes_per_row = physical_width * 4;
        let padded_bytes_per_row = align_to(bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer_size = padded_bytes_per_row as u64 * physical_height as u64;

        let uniforms = CanvasUniforms {
            viewport_size: [physical_width as f32, physical_height as f32],
            canvas_size: [canvas_size.width as f32, canvas_size.height as f32],
            zoom: viewport_state.zoom,
            _pad0: 0.0,
            pan: [viewport_state.pan_x * scale_factor as f32, viewport_state.pan_y * scale_factor as f32],
            _pad1: [0.0, 0.0],
        };

        let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("PhotoTux Canvas Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PhotoTux Canvas Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("PhotoTux Offscreen Canvas Texture"),
            size: wgpu::Extent3d {
                width: physical_width,
                height: physical_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PhotoTux Offscreen Readback Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PhotoTux Offscreen Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("PhotoTux Offscreen Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(physical_height),
                },
            },
            wgpu::Extent3d {
                width: physical_width,
                height: physical_height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit([encoder.finish()]);

        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.instance.poll_all(true);
        receiver
            .recv()
            .context("failed to receive wgpu readback completion")?
            .context("wgpu readback mapping failed")?;

        let mapped = buffer_slice.get_mapped_range();
        let mut pixels = vec![0_u8; (bytes_per_row * physical_height) as usize];
        for row in 0..physical_height as usize {
            let src_offset = row * padded_bytes_per_row as usize;
            let dst_offset = row * bytes_per_row as usize;
            pixels[dst_offset..dst_offset + bytes_per_row as usize]
                .copy_from_slice(&mapped[src_offset..src_offset + bytes_per_row as usize]);
        }
        drop(mapped);
        output_buffer.unmap();

        if let Some(canvas_raster) = canvas_raster {
            draw_canvas_raster(
                &mut pixels,
                physical_width,
                physical_height,
                viewport_state,
                scale_factor as f32,
                canvas_raster,
            );
        }

        draw_overlay_rects(
            &mut pixels,
            physical_width,
            physical_height,
            viewport_state,
            scale_factor as f32,
            overlays,
        );
        draw_overlay_paths(
            &mut pixels,
            physical_width,
            physical_height,
            viewport_state,
            scale_factor as f32,
            overlay_paths,
        );

        Ok(CanvasFrame {
            width: physical_width,
            height: physical_height,
            stride: bytes_per_row as usize,
            pixels,
        })
    }
}

fn draw_canvas_raster(
    pixels: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    viewport_state: ViewportState,
    scale_factor: f32,
    canvas_raster: &CanvasRaster,
) {
    let left = (viewport_state.pan_x * scale_factor).round() as i32;
    let top = (viewport_state.pan_y * scale_factor).round() as i32;
    let canvas_width = (canvas_raster.size.width as f32 * viewport_state.zoom).round().max(1.0) as i32;
    let canvas_height = (canvas_raster.size.height as f32 * viewport_state.zoom).round().max(1.0) as i32;
    let right = left + canvas_width;
    let bottom = top + canvas_height;

    for y in top.max(0)..bottom.min(frame_height as i32) {
        for x in left.max(0)..right.min(frame_width as i32) {
            let source_x = (((x - left) as f32) / viewport_state.zoom).floor() as i32;
            let source_y = (((y - top) as f32) / viewport_state.zoom).floor() as i32;
            if source_x < 0
                || source_y < 0
                || source_x >= canvas_raster.size.width as i32
                || source_y >= canvas_raster.size.height as i32
            {
                continue;
            }

            let source_index = ((source_y as u32 * canvas_raster.size.width + source_x as u32) * 4) as usize;
            blend_overlay_pixel(
                pixels,
                frame_width,
                x as u32,
                y as u32,
                [
                    canvas_raster.pixels[source_index],
                    canvas_raster.pixels[source_index + 1],
                    canvas_raster.pixels[source_index + 2],
                    canvas_raster.pixels[source_index + 3],
                ],
            );
        }
    }
}

fn draw_overlay_rects(
    pixels: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    viewport_state: ViewportState,
    scale_factor: f32,
    overlays: &[CanvasOverlayRect],
) {
    for overlay in overlays {
        draw_overlay_rect(pixels, frame_width, frame_height, viewport_state, scale_factor, *overlay);
    }
}

fn draw_overlay_rect(
    pixels: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    viewport_state: ViewportState,
    scale_factor: f32,
    overlay: CanvasOverlayRect,
) {
    let left = (viewport_state.pan_x * scale_factor + overlay.rect.x as f32 * viewport_state.zoom).round() as i32;
    let top = (viewport_state.pan_y * scale_factor + overlay.rect.y as f32 * viewport_state.zoom).round() as i32;
    let width = (overlay.rect.width as f32 * viewport_state.zoom).round().max(1.0) as i32;
    let height = (overlay.rect.height as f32 * viewport_state.zoom).round().max(1.0) as i32;
    let right = left + width;
    let bottom = top + height;

    if let Some(fill) = overlay.fill_rgba {
        for y in top.max(0)..bottom.min(frame_height as i32) {
            for x in left.max(0)..right.min(frame_width as i32) {
                blend_overlay_pixel(pixels, frame_width, x as u32, y as u32, fill);
            }
        }
    }

    for y in top.max(0)..bottom.min(frame_height as i32) {
        for x in left.max(0)..right.min(frame_width as i32) {
            if x == left.max(0) || x == right - 1 || y == top.max(0) || y == bottom - 1 {
                blend_overlay_pixel(pixels, frame_width, x as u32, y as u32, overlay.stroke_rgba);
            }
        }
    }
}

fn draw_overlay_paths(
    pixels: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    viewport_state: ViewportState,
    scale_factor: f32,
    overlays: &[CanvasOverlayPath],
) {
    for overlay in overlays {
        draw_overlay_path(
            pixels,
            frame_width,
            frame_height,
            viewport_state,
            scale_factor,
            overlay,
        );
    }
}

fn draw_overlay_path(
    pixels: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    viewport_state: ViewportState,
    scale_factor: f32,
    overlay: &CanvasOverlayPath,
) {
    if overlay.points.len() < 2 {
        return;
    }

    let mut transformed = overlay
        .points
        .iter()
        .map(|&(x, y)| {
            (
                (viewport_state.pan_x * scale_factor + x as f32 * viewport_state.zoom).round() as i32,
                (viewport_state.pan_y * scale_factor + y as f32 * viewport_state.zoom).round() as i32,
            )
        })
        .collect::<Vec<_>>();

    if overlay.closed {
        transformed.push(transformed[0]);
    }

    for segment in transformed.windows(2) {
        draw_overlay_line(
            pixels,
            frame_width,
            frame_height,
            segment[0].0,
            segment[0].1,
            segment[1].0,
            segment[1].1,
            overlay.stroke_rgba,
        );
    }
}

fn draw_overlay_line(
    pixels: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    rgba: [u8; 4],
) {
    let mut x0 = start_x;
    let mut y0 = start_y;
    let x1 = end_x;
    let y1 = end_y;
    let delta_x = (x1 - x0).abs();
    let step_x = if x0 < x1 { 1 } else { -1 };
    let delta_y = -(y1 - y0).abs();
    let step_y = if y0 < y1 { 1 } else { -1 };
    let mut error = delta_x + delta_y;

    loop {
        if x0 >= 0 && y0 >= 0 && x0 < frame_width as i32 && y0 < frame_height as i32 {
            blend_overlay_pixel(pixels, frame_width, x0 as u32, y0 as u32, rgba);
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let doubled_error = error * 2;
        if doubled_error >= delta_y {
            error += delta_y;
            x0 += step_x;
        }
        if doubled_error <= delta_x {
            error += delta_x;
            y0 += step_y;
        }
    }
}

fn blend_overlay_pixel(pixels: &mut [u8], frame_width: u32, x: u32, y: u32, rgba: [u8; 4]) {
    let index = ((y * frame_width + x) * 4) as usize;
    let alpha = rgba[3] as f32 / 255.0;
    for channel in 0..3 {
        let dst = pixels[index + channel] as f32;
        let src = rgba[channel] as f32;
        pixels[index + channel] = (src * alpha + dst * (1.0 - alpha)).round() as u8;
    }
    pixels[index + 3] = 255;
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

const CANVAS_SHADER: &str = r#"
struct CanvasUniforms {
    viewport_size: vec2<f32>,
    canvas_size: vec2<f32>,
    zoom: f32,
    _pad0: f32,
    pan: vec2<f32>,
    _pad1: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: CanvasUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );

    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5, 0.5);
    return output;
}

fn checker_value(cell: vec2<f32>) -> f32 {
    let parity = i32(cell.x + cell.y) & 1;
    if parity == 0 {
        return 0.78;
    }
    return 0.88;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen = in.uv * uniforms.viewport_size;
    let canvas_extent = uniforms.canvas_size * uniforms.zoom;
    let canvas_min = uniforms.pan;
    let canvas_max = canvas_min + canvas_extent;

    var color = vec3<f32>(0.10, 0.11, 0.13);

    if screen.x >= canvas_min.x && screen.y >= canvas_min.y && screen.x <= canvas_max.x && screen.y <= canvas_max.y {
        let local = (screen - canvas_min) / uniforms.zoom;
        let cell = floor(local / 24.0);
        let checker = checker_value(cell);
        color = vec3<f32>(checker, checker, checker);

        let border_distance = min(
            min(screen.x - canvas_min.x, canvas_max.x - screen.x),
            min(screen.y - canvas_min.y, canvas_max.y - screen.y),
        );

        if border_distance < 2.0 {
            color = vec3<f32>(0.94, 0.96, 0.99);
        }
    }

    return vec4<f32>(color, 1.0);
}
"#;

impl ViewportState {
    pub fn pan_by(&mut self, delta_x: f32, delta_y: f32) {
        self.pan_x += delta_x;
        self.pan_y += delta_y;
    }

    pub fn zoom_towards(&mut self, zoom_factor: f32, focal_x: f32, focal_y: f32) {
        let old_zoom = self.zoom;
        let new_zoom = (self.zoom * zoom_factor).clamp(0.05, 64.0);

        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return;
        }

        self.pan_x = focal_x - ((focal_x - self.pan_x) / old_zoom) * new_zoom;
        self.pan_y = focal_y - ((focal_y - self.pan_y) / old_zoom) * new_zoom;
        self.zoom = new_zoom;
    }

    pub fn fit_canvas(canvas_size: CanvasSize, viewport_size: ViewportSize) -> Self {
        let zoom_x = viewport_size.width / canvas_size.width as f32;
        let zoom_y = viewport_size.height / canvas_size.height as f32;
        let zoom = zoom_x.min(zoom_y).clamp(0.05, 64.0);

        let content_width = canvas_size.width as f32 * zoom;
        let content_height = canvas_size.height as f32 * zoom;
        let pan_x = (viewport_size.width - content_width) * 0.5;
        let pan_y = (viewport_size.height - content_height) * 0.5;

        Self { zoom, pan_x, pan_y }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        draw_canvas_raster, draw_overlay_paths, draw_overlay_rects, CanvasOverlayPath,
        CanvasOverlayRect, ViewportSize, ViewportState,
    };
    use common::{CanvasRaster, CanvasRect, CanvasSize};

    #[test]
    fn pan_by_offsets_the_viewport() {
        let mut state = ViewportState::default();
        state.pan_by(24.0, -12.0);

        assert_eq!(state.pan_x, 24.0);
        assert_eq!(state.pan_y, -12.0);
    }

    #[test]
    fn zoom_towards_keeps_focus_point_stable() {
        let mut state = ViewportState::default();
        let focal_x = 200.0;
        let focal_y = 100.0;

        let world_x_before = (focal_x - state.pan_x) / state.zoom;
        let world_y_before = (focal_y - state.pan_y) / state.zoom;

        state.zoom_towards(2.0, focal_x, focal_y);

        let world_x_after = (focal_x - state.pan_x) / state.zoom;
        let world_y_after = (focal_y - state.pan_y) / state.zoom;

        assert!((world_x_before - world_x_after).abs() < 0.001);
        assert!((world_y_before - world_y_after).abs() < 0.001);
    }

    #[test]
    fn fit_canvas_centers_content_in_viewport() {
        let state = ViewportState::fit_canvas(CanvasSize::new(1000, 500), ViewportSize::new(2000.0, 1000.0));

        assert_eq!(state.zoom, 2.0);
        assert_eq!(state.pan_x, 0.0);
        assert_eq!(state.pan_y, 0.0);
    }

    #[test]
    fn fit_canvas_preserves_centering_when_aspect_ratios_differ() {
        let state = ViewportState::fit_canvas(CanvasSize::new(1000, 500), ViewportSize::new(1200.0, 1200.0));

        assert!((state.zoom - 1.2).abs() < 0.001);
        assert!((state.pan_x - 0.0).abs() < 0.001);
        assert!((state.pan_y - 300.0).abs() < 0.001);
    }

    #[test]
    fn overlay_rect_draws_visible_stroke() {
        let mut pixels = vec![0_u8; 64 * 64 * 4];
        draw_overlay_rects(
            &mut pixels,
            64,
            64,
            ViewportState::default(),
            1.0,
            &[CanvasOverlayRect {
                rect: CanvasRect::new(10, 10, 20, 20),
                stroke_rgba: [255, 0, 0, 255],
                fill_rgba: None,
            }],
        );

        let top_left = ((10 * 64 + 10) * 4) as usize;
        assert_eq!(&pixels[top_left..top_left + 4], &[255, 0, 0, 255]);
    }

    #[test]
    fn overlay_path_draws_visible_stroke() {
        let mut pixels = vec![0_u8; 64 * 64 * 4];
        draw_overlay_paths(
            &mut pixels,
            64,
            64,
            ViewportState::default(),
            1.0,
            &[CanvasOverlayPath {
                points: vec![(10, 10), (20, 10), (20, 20)],
                stroke_rgba: [0, 255, 0, 255],
                closed: false,
            }],
        );

        let point = ((10 * 64 + 15) * 4) as usize;
        assert_eq!(&pixels[point..point + 4], &[0, 255, 0, 255]);
    }

    #[test]
    fn canvas_raster_draws_into_frame() {
        let mut pixels = vec![0_u8; 32 * 32 * 4];
        let mut raster_pixels = vec![0_u8; 4 * 4 * 4];
        raster_pixels[0..4].copy_from_slice(&[10, 20, 30, 255]);

        draw_canvas_raster(
            &mut pixels,
            32,
            32,
            ViewportState::default(),
            1.0,
            &CanvasRaster {
                size: CanvasSize::new(4, 4),
                pixels: raster_pixels,
            },
        );

        assert_eq!(&pixels[0..4], &[10, 20, 30, 255]);
    }
}
