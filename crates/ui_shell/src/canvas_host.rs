use super::*;

pub(super) fn build_canvas_host(
    controller: Rc<RefCell<dyn ShellController>>,
) -> (Picture, Rc<RefCell<CanvasHostState>>) {
    let picture = Picture::new();
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    picture.set_can_shrink(true);
    picture.set_focusable(true);
    picture.set_focus_on_click(true);
    picture.add_css_class("frame");

    let state = Rc::new(RefCell::new(CanvasHostState::new(
        picture.clone(),
        controller,
    )));
    wire_canvas_motion(&picture, state.clone());
    wire_canvas_drag(&picture, state.clone());
    wire_canvas_stylus(&picture, state.clone());
    wire_canvas_scroll(&picture, state.clone());

    let tick_state = state.clone();
    glib::timeout_add_local(Duration::from_millis(16), move || {
        tick_state.borrow_mut().tick();
        ControlFlow::Continue
    });

    (picture, state)
}

fn wire_canvas_motion(picture: &Picture, state: Rc<RefCell<CanvasHostState>>) {
    let motion = EventControllerMotion::new();

    {
        let state = state.clone();
        motion.connect_enter(move |_, x, y| {
            state.borrow_mut().update_hover_position(x as f32, y as f32);
        });
    }

    {
        let state = state.clone();
        motion.connect_motion(move |_, x, y| {
            state.borrow_mut().update_hover_position(x as f32, y as f32);
        });
    }

    motion.connect_leave(move |_| {
        state.borrow_mut().clear_hover_position();
    });

    picture.add_controller(motion);
}

fn wire_canvas_drag(picture: &Picture, state: Rc<RefCell<CanvasHostState>>) {
    let drag = GestureDrag::new();

    {
        let state = state.clone();
        drag.connect_drag_begin(move |gesture, start_x, start_y| {
            let bypass = gesture
                .current_event_state()
                .contains(gdk::ModifierType::SHIFT_MASK);
            state
                .borrow_mut()
                .begin_drag(start_x as f32, start_y as f32, bypass);
        });
    }

    {
        let state = state.clone();
        drag.connect_drag_update(move |gesture, offset_x, offset_y| {
            let bypass = gesture
                .current_event_state()
                .contains(gdk::ModifierType::SHIFT_MASK);
            state
                .borrow_mut()
                .drag_to(offset_x as f32, offset_y as f32, bypass);
        });
    }

    drag.connect_drag_end(move |_, _, _| {
        state.borrow_mut().end_drag();
    });

    picture.add_controller(drag);
}

fn wire_canvas_scroll(picture: &Picture, state: Rc<RefCell<CanvasHostState>>) {
    let picture = picture.clone();
    let picture_for_scroll = picture.clone();
    let scroll = EventControllerScroll::new(
        EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::DISCRETE,
    );

    scroll.connect_scroll(move |_, _, delta_y| {
        let width = picture_for_scroll.width().max(1) as f32;
        let height = picture_for_scroll.height().max(1) as f32;
        state.borrow_mut().zoom(delta_y, width * 0.5, height * 0.5);
        glib::Propagation::Stop
    });

    picture.add_controller(scroll);
}

fn wire_canvas_stylus(picture: &Picture, state: Rc<RefCell<CanvasHostState>>) {
    let stylus = GestureStylus::new();

    {
        let state = state.clone();
        stylus.connect_down(move |gesture, _, _| {
            state
                .borrow_mut()
                .set_pointer_pressure(stylus_pressure(gesture));
        });
    }

    {
        let state = state.clone();
        stylus.connect_motion(move |gesture, _, _| {
            state
                .borrow_mut()
                .set_pointer_pressure(stylus_pressure(gesture));
        });
    }

    {
        let state = state.clone();
        stylus.connect_up(move |_, _, _| {
            state.borrow_mut().clear_pointer_pressure();
        });
    }

    picture.add_controller(stylus);
}

fn stylus_pressure(gesture: &GestureStylus) -> f32 {
    gesture
        .axis(gdk::AxisUse::Pressure)
        .map(|pressure| pressure.clamp(0.0, 1.0) as f32)
        .unwrap_or(1.0)
}

pub(super) struct CanvasHostState {
    picture: Picture,
    controller: Rc<RefCell<dyn ShellController>>,
    renderer: Option<OffscreenCanvasRenderer>,
    viewport_state: ViewportState,
    canvas_size: CanvasSize,
    canvas_raster: Option<CanvasRaster>,
    drag_origin_pan: Option<(f32, f32)>,
    drag_start_screen: Option<(f32, f32)>,
    hovered_canvas_position: Option<(i32, i32)>,
    pointer_pressure: f32,
    viewport_fitted: bool,
    last_logical_size: (u32, u32),
    last_canvas_revision: Option<u64>,
    last_brush_preview_signature: Option<BrushPreviewSignature>,
    last_active_layer_bounds: Option<CanvasRect>,
    last_selection_rect: Option<CanvasRect>,
    last_selection_path: Option<Vec<(i32, i32)>>,
    last_selection_preview_path: Option<Vec<(i32, i32)>>,
    last_guides_visible: bool,
    last_guides: Vec<ShellGuide>,
    last_selection_inverted: bool,
    dirty: bool,
}

impl CanvasHostState {
    fn new(picture: Picture, controller: Rc<RefCell<dyn ShellController>>) -> Self {
        let renderer = match OffscreenCanvasRenderer::new_blocking() {
            Ok(renderer) => Some(renderer),
            Err(error) => {
                tracing::error!(%error, "failed to initialize offscreen canvas renderer");
                None
            }
        };

        Self {
            picture,
            controller,
            renderer,
            viewport_state: ViewportState::default(),
            canvas_size: CanvasSize::new(1920, 1080),
            canvas_raster: None,
            drag_origin_pan: None,
            drag_start_screen: None,
            hovered_canvas_position: None,
            pointer_pressure: 1.0,
            viewport_fitted: false,
            last_logical_size: (0, 0),
            last_canvas_revision: None,
            last_brush_preview_signature: None,
            last_active_layer_bounds: None,
            last_selection_rect: None,
            last_selection_path: None,
            last_selection_preview_path: None,
            last_guides_visible: true,
            last_guides: Vec::new(),
            last_selection_inverted: false,
            dirty: true,
        }
    }

    fn tick(&mut self) {
        let snapshot = self.controller.borrow().snapshot();
        if self.canvas_size != snapshot.canvas_size {
            self.canvas_size = snapshot.canvas_size;
            self.viewport_fitted = false;
            self.dirty = true;
        }

        let preview_signature = brush_preview_signature(&snapshot);
        if self.last_brush_preview_signature != preview_signature {
            self.last_brush_preview_signature = preview_signature;
            self.dirty = true;
        }

        if self.last_canvas_revision != Some(snapshot.canvas_revision) {
            self.canvas_raster = Some(self.controller.borrow().canvas_raster());
            self.last_canvas_revision = Some(snapshot.canvas_revision);
            self.dirty = true;
        }

        if self.last_active_layer_bounds != snapshot.active_layer_bounds
            || self.last_selection_rect != snapshot.selection_rect
            || self.last_selection_path != snapshot.selection_path
            || self.last_selection_preview_path != snapshot.selection_preview_path
            || self.last_guides_visible != snapshot.guides_visible
            || self.last_guides != snapshot.guides
            || self.last_selection_inverted != snapshot.selection_inverted
        {
            self.last_active_layer_bounds = snapshot.active_layer_bounds;
            self.last_selection_rect = snapshot.selection_rect;
            self.last_selection_path = snapshot.selection_path.clone();
            self.last_selection_preview_path = snapshot.selection_preview_path.clone();
            self.last_guides_visible = snapshot.guides_visible;
            self.last_guides = snapshot.guides.clone();
            self.last_selection_inverted = snapshot.selection_inverted;
            self.dirty = true;
        }

        let logical_width = self.picture.width().max(0) as u32;
        let logical_height = self.picture.height().max(0) as u32;

        if logical_width <= 1 || logical_height <= 1 {
            return;
        }

        if !self.viewport_fitted {
            self.viewport_state = ViewportState::fit_canvas(
                self.canvas_size,
                ViewportSize::new(logical_width as f32, logical_height as f32),
            );
            self.viewport_fitted = true;
            self.last_logical_size = (logical_width, logical_height);
            self.dirty = true;
        } else if self.last_logical_size != (logical_width, logical_height) {
            self.adjust_viewport_for_resize(logical_width, logical_height);
            self.last_logical_size = (logical_width, logical_height);
            self.dirty = true;
        }

        if !self.dirty {
            return;
        }

        let _ = self.render_snapshot(&snapshot, logical_width, logical_height);
    }

    pub(super) fn warm_up_startup(&mut self, logical_width: u32, logical_height: u32) -> bool {
        let snapshot = self.controller.borrow().snapshot();
        self.canvas_size = snapshot.canvas_size;
        self.canvas_raster = Some(self.controller.borrow().canvas_raster());
        self.viewport_state = ViewportState::fit_canvas(
            self.canvas_size,
            ViewportSize::new(logical_width as f32, logical_height as f32),
        );
        self.viewport_fitted = true;
        self.last_logical_size = (logical_width, logical_height);
        self.last_canvas_revision = Some(snapshot.canvas_revision);
        self.last_brush_preview_signature = brush_preview_signature(&snapshot);
        self.last_active_layer_bounds = snapshot.active_layer_bounds;
        self.last_selection_rect = snapshot.selection_rect;
        self.last_selection_path = snapshot.selection_path.clone();
        self.last_selection_preview_path = snapshot.selection_preview_path.clone();
        self.last_guides_visible = snapshot.guides_visible;
        self.last_guides = snapshot.guides.clone();
        self.last_selection_inverted = snapshot.selection_inverted;
        self.render_snapshot(&snapshot, logical_width, logical_height)
    }

    fn render_snapshot(
        &mut self,
        snapshot: &ShellSnapshot,
        logical_width: u32,
        logical_height: u32,
    ) -> bool {
        let Some(renderer) = &self.renderer else {
            return false;
        };

        let scale_factor = self.picture.scale_factor() as f64;
        let mut overlays = Vec::new();
        let mut overlay_paths = Vec::new();
        if let Some(bounds) = snapshot.transform_preview_rect {
            overlays.push(CanvasOverlayRect {
                rect: bounds,
                stroke_rgba: [255, 170, 61, 255],
                fill_rgba: Some([255, 170, 61, 28]),
            });
        }
        if snapshot.guides_visible {
            for guide in &snapshot.guides {
                match *guide {
                    ShellGuide::Horizontal { y } => overlay_paths.push(CanvasOverlayPath {
                        points: vec![(0, y), (self.canvas_size.width as i32 - 1, y)],
                        stroke_rgba: [255, 72, 72, 220],
                        closed: false,
                    }),
                    ShellGuide::Vertical { x } => overlay_paths.push(CanvasOverlayPath {
                        points: vec![(x, 0), (x, self.canvas_size.height as i32 - 1)],
                        stroke_rgba: [255, 72, 72, 220],
                        closed: false,
                    }),
                }
            }
        }
        if let Some(points) = snapshot.selection_preview_path.clone() {
            overlay_paths.push(CanvasOverlayPath {
                points,
                stroke_rgba: [116, 167, 255, 255],
                closed: false,
            });
        } else if let Some(points) = snapshot.selection_path.clone() {
            overlay_paths.push(CanvasOverlayPath {
                points,
                stroke_rgba: [116, 167, 255, 255],
                closed: true,
            });
        } else if let Some(selection) = snapshot.selection_rect {
            overlays.push(CanvasOverlayRect {
                rect: selection,
                stroke_rgba: [116, 167, 255, 255],
                fill_rgba: Some([79, 140, 255, 36]),
            });
        }
        overlay_paths.extend(self.build_active_brush_preview_paths(snapshot));
        match renderer.render(
            self.canvas_size,
            self.viewport_state,
            ViewportRendererConfig {
                width: logical_width,
                height: logical_height,
                scale_factor,
            },
            self.canvas_raster.as_ref(),
            &overlays,
            &overlay_paths,
        ) {
            Ok(frame) => {
                let bytes = glib::Bytes::from_owned(frame.pixels);
                let texture = gdk::MemoryTexture::new(
                    frame.width as i32,
                    frame.height as i32,
                    gdk::MemoryFormat::R8g8b8a8,
                    &bytes,
                    frame.stride,
                );
                self.picture.set_paintable(Some(&texture));
                self.dirty = false;
                true
            }
            Err(error) => {
                tracing::error!(%error, "failed to render offscreen canvas frame");
                false
            }
        }
    }

    fn begin_drag(&mut self, start_x: f32, start_y: f32, snap_bypass: bool) {
        self.drag_start_screen = Some((start_x, start_y));
        self.update_hover_position(start_x, start_y);
        let snapshot = self.controller.borrow().snapshot();
        match snapshot.active_tool {
            ShellToolKind::Hand => {
                self.drag_origin_pan = Some((self.viewport_state.pan_x, self.viewport_state.pan_y));
            }
            ShellToolKind::Move
            | ShellToolKind::RectangularMarquee
            | ShellToolKind::Lasso
            | ShellToolKind::Transform
            | ShellToolKind::Brush
            | ShellToolKind::Eraser => {
                let (canvas_x, canvas_y) = self.screen_to_canvas(start_x, start_y);
                self.controller
                    .borrow_mut()
                    .set_temporary_snap_bypass(snap_bypass);
                self.controller
                    .borrow_mut()
                    .begin_canvas_interaction_with_pressure(
                        canvas_x,
                        canvas_y,
                        self.pointer_pressure,
                    );
            }
            _ => {}
        }
    }

    fn drag_to(&mut self, offset_x: f32, offset_y: f32, snap_bypass: bool) {
        let snapshot = self.controller.borrow().snapshot();
        match snapshot.active_tool {
            ShellToolKind::Hand => {
                if let Some((origin_x, origin_y)) = self.drag_origin_pan {
                    self.viewport_state.pan_x = origin_x + offset_x;
                    self.viewport_state.pan_y = origin_y + offset_y;
                    self.dirty = true;
                }
            }
            ShellToolKind::Move
            | ShellToolKind::RectangularMarquee
            | ShellToolKind::Lasso
            | ShellToolKind::Transform
            | ShellToolKind::Brush
            | ShellToolKind::Eraser => {
                if let Some((start_x, start_y)) = self.drag_start_screen {
                    self.update_hover_position(start_x + offset_x, start_y + offset_y);
                    let (canvas_x, canvas_y) =
                        self.screen_to_canvas(start_x + offset_x, start_y + offset_y);
                    self.controller
                        .borrow_mut()
                        .set_temporary_snap_bypass(snap_bypass);
                    self.controller
                        .borrow_mut()
                        .update_canvas_interaction_with_pressure(
                            canvas_x,
                            canvas_y,
                            self.pointer_pressure,
                        );
                    self.dirty = true;
                    self.tick();
                }
            }
            _ => {}
        }
    }

    fn end_drag(&mut self) {
        self.drag_origin_pan = None;
        self.drag_start_screen = None;
        self.controller
            .borrow_mut()
            .set_temporary_snap_bypass(false);
        self.controller.borrow_mut().end_canvas_interaction();
        self.dirty = true;
        self.tick();
    }

    fn set_pointer_pressure(&mut self, pressure: f32) {
        let pressure = pressure.clamp(0.0, 1.0);
        if (self.pointer_pressure - pressure).abs() > f32::EPSILON {
            self.pointer_pressure = pressure;
            self.dirty = true;
        }
    }

    fn clear_pointer_pressure(&mut self) {
        if (self.pointer_pressure - 1.0).abs() > f32::EPSILON {
            self.pointer_pressure = 1.0;
            self.dirty = true;
        }
    }

    fn update_hover_position(&mut self, screen_x: f32, screen_y: f32) {
        let canvas_position = self.screen_to_canvas(screen_x, screen_y);
        if self.hovered_canvas_position != Some(canvas_position) {
            self.hovered_canvas_position = Some(canvas_position);
            self.dirty = true;
        }
    }

    fn clear_hover_position(&mut self) {
        if self.hovered_canvas_position.take().is_some() {
            self.dirty = true;
        }
    }

    fn build_active_brush_preview_paths(&self, snapshot: &ShellSnapshot) -> Vec<CanvasOverlayPath> {
        if !matches!(
            snapshot.active_tool,
            ShellToolKind::Brush | ShellToolKind::Eraser
        ) {
            return Vec::new();
        }

        let Some((center_x, center_y)) = self.hovered_canvas_position else {
            return Vec::new();
        };
        if center_x < 0
            || center_y < 0
            || center_x >= self.canvas_size.width as i32
            || center_y >= self.canvas_size.height as i32
        {
            return Vec::new();
        }

        let radius = brush_preview_radius(
            snapshot.brush_radius,
            snapshot.pressure_size_enabled,
            self.pointer_pressure,
        );
        let spacing = brush_preview_spacing(radius, snapshot.brush_spacing);
        build_brush_preview_paths(
            snapshot.active_tool,
            (center_x, center_y),
            radius,
            snapshot.brush_hardness_percent,
            spacing,
            self.viewport_state.zoom,
        )
    }

    fn zoom(&mut self, delta_y: f64, focal_x: f32, focal_y: f32) {
        let zoom_factor = if delta_y < 0.0 { 1.1 } else { 1.0 / 1.1 };
        self.viewport_state
            .zoom_towards(zoom_factor, focal_x, focal_y);
        self.dirty = true;
        self.tick();
    }

    pub(super) fn zoom_in(&mut self) {
        let width = self.picture.width().max(1) as f32;
        let height = self.picture.height().max(1) as f32;
        self.viewport_state
            .zoom_towards(1.1, width * 0.5, height * 0.5);
        self.dirty = true;
        self.tick();
    }

    pub(super) fn zoom_out(&mut self) {
        let width = self.picture.width().max(1) as f32;
        let height = self.picture.height().max(1) as f32;
        self.viewport_state
            .zoom_towards(1.0 / 1.1, width * 0.5, height * 0.5);
        self.dirty = true;
        self.tick();
    }

    pub(super) fn fit_to_view(&mut self) {
        let logical_width = self.picture.width().max(0) as u32;
        let logical_height = self.picture.height().max(0) as u32;
        if logical_width <= 1 || logical_height <= 1 {
            return;
        }
        self.viewport_state = ViewportState::fit_canvas(
            self.canvas_size,
            ViewportSize::new(logical_width as f32, logical_height as f32),
        );
        self.viewport_fitted = true;
        self.last_logical_size = (logical_width, logical_height);
        self.dirty = true;
        self.tick();
    }

    fn adjust_viewport_for_resize(&mut self, logical_width: u32, logical_height: u32) {
        let delta_x = logical_width as f32 - self.last_logical_size.0 as f32;
        let delta_y = logical_height as f32 - self.last_logical_size.1 as f32;
        self.viewport_state.pan_x += delta_x * 0.5;
        self.viewport_state.pan_y += delta_y * 0.5;
    }

    pub(super) fn zoom_percent(&self) -> u32 {
        (self.viewport_state.zoom * 100.0).round().max(1.0) as u32
    }

    fn screen_to_canvas(&self, screen_x: f32, screen_y: f32) -> (i32, i32) {
        let canvas_x =
            ((screen_x - self.viewport_state.pan_x) / self.viewport_state.zoom).round() as i32;
        let canvas_y =
            ((screen_y - self.viewport_state.pan_y) / self.viewport_state.zoom).round() as i32;
        (canvas_x, canvas_y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BrushPreviewSignature {
    active_tool: ShellToolKind,
    brush_radius: u32,
    brush_hardness_percent: u32,
    brush_spacing: u32,
    pressure_size_enabled: bool,
}

fn brush_preview_signature(snapshot: &ShellSnapshot) -> Option<BrushPreviewSignature> {
    match snapshot.active_tool {
        ShellToolKind::Brush | ShellToolKind::Eraser => Some(BrushPreviewSignature {
            active_tool: snapshot.active_tool,
            brush_radius: snapshot.brush_radius,
            brush_hardness_percent: snapshot.brush_hardness_percent,
            brush_spacing: snapshot.brush_spacing,
            pressure_size_enabled: snapshot.pressure_size_enabled,
        }),
        _ => None,
    }
}

pub(super) fn brush_preview_radius(
    base_radius: u32,
    pressure_size_enabled: bool,
    pressure: f32,
) -> f32 {
    let base_radius = base_radius.max(1) as f32;
    if pressure_size_enabled {
        base_radius * (0.35 + 0.65 * pressure.clamp(0.0, 1.0))
    } else {
        base_radius
    }
}

fn brush_preview_spacing(radius: f32, spacing: u32) -> f32 {
    (spacing.max(1) as f32).clamp(1.0, (radius * 1.5).max(1.0))
}

pub(super) fn build_brush_preview_paths(
    tool: ShellToolKind,
    center: (i32, i32),
    radius: f32,
    hardness_percent: u32,
    spacing: f32,
    zoom: f32,
) -> Vec<CanvasOverlayPath> {
    let mut paths = vec![CanvasOverlayPath {
        points: brush_preview_circle_points(center, radius, zoom),
        stroke_rgba: brush_preview_outer_color(tool),
        closed: true,
    }];

    let hardness_radius = radius * (hardness_percent.clamp(0, 100) as f32 / 100.0);
    if hardness_radius >= 2.0 && (radius - hardness_radius) >= 1.0 {
        paths.push(CanvasOverlayPath {
            points: brush_preview_circle_points(center, hardness_radius, zoom),
            stroke_rgba: brush_preview_detail_color(tool),
            closed: true,
        });
    }

    let crosshair_extent = (radius * 0.35).clamp(2.0, 8.0).round() as i32;
    paths.push(CanvasOverlayPath {
        points: vec![
            (center.0 - crosshair_extent, center.1),
            (center.0 + crosshair_extent, center.1),
        ],
        stroke_rgba: brush_preview_detail_color(tool),
        closed: false,
    });
    paths.push(CanvasOverlayPath {
        points: vec![
            (center.0, center.1 - crosshair_extent),
            (center.0, center.1 + crosshair_extent),
        ],
        stroke_rgba: brush_preview_detail_color(tool),
        closed: false,
    });

    let spacing_marker_center = (center.0 + spacing.round() as i32, center.1);
    let spacing_marker_radius = (radius * 0.16).clamp(1.0, 3.0);
    if spacing >= spacing_marker_radius * 2.0 {
        paths.push(CanvasOverlayPath {
            points: brush_preview_circle_points(spacing_marker_center, spacing_marker_radius, zoom),
            stroke_rgba: brush_preview_spacing_color(tool),
            closed: true,
        });
    }

    paths
}

fn brush_preview_circle_points(center: (i32, i32), radius: f32, zoom: f32) -> Vec<(i32, i32)> {
    let screen_radius = (radius * zoom.max(0.1)).max(1.0);
    let segments = (screen_radius.round() as usize).clamp(18, 48);
    let mut points = Vec::with_capacity(segments);
    for index in 0..segments {
        let angle = (index as f32 / segments as f32) * std::f32::consts::TAU;
        points.push((
            (center.0 as f32 + radius * angle.cos()).round() as i32,
            (center.1 as f32 + radius * angle.sin()).round() as i32,
        ));
    }
    points.dedup();
    if points.len() < 3 {
        return vec![
            (center.0 - 1, center.1 - 1),
            (center.0 + 1, center.1 - 1),
            (center.0 + 1, center.1 + 1),
            (center.0 - 1, center.1 + 1),
        ];
    }
    points
}

fn brush_preview_outer_color(tool: ShellToolKind) -> [u8; 4] {
    match tool {
        ShellToolKind::Brush => [245, 247, 250, 228],
        ShellToolKind::Eraser => [255, 212, 160, 228],
        _ => [245, 247, 250, 228],
    }
}

fn brush_preview_detail_color(tool: ShellToolKind) -> [u8; 4] {
    match tool {
        ShellToolKind::Brush => [116, 167, 255, 196],
        ShellToolKind::Eraser => [255, 150, 92, 196],
        _ => [116, 167, 255, 196],
    }
}

fn brush_preview_spacing_color(tool: ShellToolKind) -> [u8; 4] {
    match tool {
        ShellToolKind::Brush => [136, 203, 255, 170],
        ShellToolKind::Eraser => [255, 186, 120, 170],
        _ => [136, 203, 255, 170],
    }
}
