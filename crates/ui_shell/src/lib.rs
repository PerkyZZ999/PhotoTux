use anyhow::Result;
use common::{CanvasSize, APP_NAME};
use glib::ControlFlow;
use gtk4::prelude::*;
use gtk4::{
    gdk, Application, ApplicationWindow, Box as GtkBox, Button, EventControllerScroll,
    EventControllerScrollFlags, GestureDrag, HeaderBar, Label, Orientation, Paned, Picture,
};
use render_wgpu::{OffscreenCanvasRenderer, ViewportSize, ViewportState};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn run() -> Result<()> {
    let application = Application::builder()
        .application_id("com.phototux.app")
        .build();

    application.connect_activate(build_ui);
    let _exit_code = application.run();

    Ok(())
}

fn build_ui(application: &Application) {
    let window = ApplicationWindow::builder()
        .application(application)
        .title(APP_NAME)
        .default_width(1600)
        .default_height(900)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&build_header_bar());
    root.append(&build_menu_bar_placeholder());
    root.append(&build_tool_options_bar());
    root.append(&build_workspace_body());
    root.append(&build_status_bar());

    window.set_child(Some(&root));
    window.present();
}

fn build_header_bar() -> HeaderBar {
    let header = HeaderBar::new();
    header.set_title_widget(Some(&Label::new(Some(APP_NAME))));
    header.pack_start(&Button::with_label("Essentials"));
    header.pack_end(&Button::with_label("Search"));
    header
}

fn build_menu_bar_placeholder() -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.set_margin_start(8);
    bar.set_margin_end(8);
    bar.set_margin_top(4);
    bar.set_margin_bottom(4);

    for title in ["File", "Edit", "Image", "Layer", "Select", "Filter", "View", "Window", "Help"] {
        bar.append(&Button::with_label(title));
    }

    bar
}

fn build_tool_options_bar() -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 6);
    bar.set_margin_start(8);
    bar.set_margin_end(8);
    bar.set_margin_top(4);
    bar.set_margin_bottom(4);

    for title in ["Tool: Brush", "Preset", "Size", "Hardness", "Opacity", "Flow"] {
        bar.append(&Button::with_label(title));
    }

    bar
}

fn build_workspace_body() -> Paned {
    let outer = Paned::new(Orientation::Horizontal);
    outer.set_wide_handle(true);
    outer.set_start_child(Some(&build_left_tool_rail()));

    let inner = Paned::new(Orientation::Horizontal);
    inner.set_wide_handle(true);
    inner.set_start_child(Some(&build_document_region()));
    inner.set_end_child(Some(&build_right_dock()));
    inner.set_position(1180);

    outer.set_end_child(Some(&inner));
    outer.set_position(76);
    outer
}

fn build_left_tool_rail() -> GtkBox {
    let rail = GtkBox::new(Orientation::Vertical, 4);
    rail.set_margin_start(8);
    rail.set_margin_end(8);
    rail.set_margin_top(8);
    rail.set_margin_bottom(8);

    for tool in ["Move", "Marquee", "Brush", "Eraser", "Hand", "Zoom"] {
        rail.append(&Button::with_label(tool));
    }

    rail.append(&Label::new(Some("FG/BG")));
    rail
}

fn build_document_region() -> GtkBox {
    let region = GtkBox::new(Orientation::Vertical, 6);
    region.set_margin_start(8);
    region.set_margin_end(8);
    region.set_margin_top(8);
    region.set_margin_bottom(8);

    let tabs = GtkBox::new(Orientation::Horizontal, 4);
    tabs.append(&Button::with_label("untitled.ptx ●"));
    tabs.append(&Button::with_label("+"));

    let canvas = build_canvas_host();

    region.append(&tabs);
    region.append(&canvas);
    region
}

fn build_right_dock() -> GtkBox {
    let dock = GtkBox::new(Orientation::Vertical, 8);
    dock.set_margin_start(8);
    dock.set_margin_end(8);
    dock.set_margin_top(8);
    dock.set_margin_bottom(8);
    dock.set_size_request(320, -1);

    for panel in ["Properties", "Color", "Layers", "History"] {
        let panel_box = GtkBox::new(Orientation::Vertical, 4);
        panel_box.append(&Label::new(Some(panel)));
        panel_box.append(&Label::new(Some("Panel placeholder")));
        dock.append(&panel_box);
    }

    dock
}

fn build_status_bar() -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 8);
    bar.set_margin_start(8);
    bar.set_margin_end(8);
    bar.set_margin_top(4);
    bar.set_margin_bottom(8);
    bar.append(&Label::new(Some("100% | 1920x1080 px | Cursor 0,0")));
    bar
}

fn build_canvas_host() -> Picture {
    let picture = Picture::new();
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    picture.set_can_shrink(true);
    picture.add_css_class("frame");

    let state = Rc::new(RefCell::new(CanvasHostState::new(picture.clone())));
    wire_canvas_drag(&picture, state.clone());
    wire_canvas_scroll(&picture, state.clone());

    glib::timeout_add_local(Duration::from_millis(16), move || {
        state.borrow_mut().tick();
        ControlFlow::Continue
    });

    picture
}

fn wire_canvas_drag(picture: &Picture, state: Rc<RefCell<CanvasHostState>>) {
    let drag = GestureDrag::new();

    {
        let state = state.clone();
        drag.connect_drag_begin(move |_, _, _| {
            state.borrow_mut().begin_drag();
        });
    }

    {
        let state = state.clone();
        drag.connect_drag_update(move |_, offset_x, offset_y| {
            state.borrow_mut().drag_to(offset_x as f32, offset_y as f32);
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

struct CanvasHostState {
    picture: Picture,
    renderer: Option<OffscreenCanvasRenderer>,
    viewport_state: ViewportState,
    canvas_size: CanvasSize,
    drag_origin_pan: Option<(f32, f32)>,
    viewport_fitted: bool,
    last_logical_size: (u32, u32),
    dirty: bool,
}

impl CanvasHostState {
    fn new(picture: Picture) -> Self {
        let renderer = match OffscreenCanvasRenderer::new_blocking() {
            Ok(renderer) => Some(renderer),
            Err(error) => {
                tracing::error!(%error, "failed to initialize offscreen canvas renderer");
                None
            }
        };

        Self {
            picture,
            renderer,
            viewport_state: ViewportState::default(),
            canvas_size: CanvasSize::new(1920, 1080),
            drag_origin_pan: None,
            viewport_fitted: false,
            last_logical_size: (0, 0),
            dirty: true,
        }
    }

    fn tick(&mut self) {
        let logical_width = self.picture.width().max(1) as u32;
        let logical_height = self.picture.height().max(1) as u32;

        if logical_width == 0 || logical_height == 0 {
            return;
        }

        if !self.viewport_fitted || self.last_logical_size != (logical_width, logical_height) {
            self.viewport_state = ViewportState::fit_canvas(
                self.canvas_size,
                ViewportSize::new(logical_width as f32, logical_height as f32),
            );
            self.viewport_fitted = true;
            self.last_logical_size = (logical_width, logical_height);
            self.dirty = true;
        }

        if !self.dirty {
            return;
        }

        let Some(renderer) = &self.renderer else {
            return;
        };

        let scale_factor = self.picture.scale_factor() as f64;
        match renderer.render(
            self.canvas_size,
            self.viewport_state,
            logical_width,
            logical_height,
            scale_factor,
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
            }
            Err(error) => {
                tracing::error!(%error, "failed to render offscreen canvas frame");
            }
        }
    }

    fn begin_drag(&mut self) {
        self.drag_origin_pan = Some((self.viewport_state.pan_x, self.viewport_state.pan_y));
    }

    fn drag_to(&mut self, offset_x: f32, offset_y: f32) {
        if let Some((origin_x, origin_y)) = self.drag_origin_pan {
            self.viewport_state.pan_x = origin_x + offset_x;
            self.viewport_state.pan_y = origin_y + offset_y;
            self.dirty = true;
        }
    }

    fn end_drag(&mut self) {
        self.drag_origin_pan = None;
    }

    fn zoom(&mut self, delta_y: f64, focal_x: f32, focal_y: f32) {
        let zoom_factor = if delta_y < 0.0 { 1.1 } else { 1.0 / 1.1 };
        self.viewport_state.zoom_towards(zoom_factor, focal_x, focal_y);
        self.dirty = true;
    }
}
