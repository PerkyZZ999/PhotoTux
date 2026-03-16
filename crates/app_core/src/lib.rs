use anyhow::Context;
use std::collections::VecDeque;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use common::{CanvasRaster, CanvasRect};
use doc_model::{BlendMode, Document, LayerEditTarget, RasterMask};
use file_io::{
    export_jpeg_to_path, export_png_to_path, export_webp_to_path, flatten_document_rgba,
    import_jpeg_from_path, import_png_from_path, import_webp_from_path, load_document_from_path,
    recovery_path_for_project_path, remove_file_if_exists, save_document_to_path,
    PROJECT_FILE_EXTENSION,
};
use history_engine::HistoryStack;
use tool_system::{
    BrushChange, BrushStrokeRecord, BrushSettings, BrushTool, BrushToolMode, MoveLayerRecord,
    RectangularMarqueeTool, RectangularSelectionRecord, SimpleTransformTool, LayerTransformRecord,
};
use ui_shell::{LayerPanelItem, ShellController, ShellSnapshot, ShellToolKind};

pub fn build_shell_controller() -> Rc<RefCell<dyn ShellController>> {
    Rc::new(RefCell::new(PhotoTuxController::new()))
}

const AUTOSAVE_IDLE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug)]
struct PhotoTuxController {
    document: Document,
    history: HistoryStack<EditorHistoryEntry>,
    foreground_color: [u8; 4],
    background_color: [u8; 4],
    status_message: String,
    document_title: String,
    document_path: Option<PathBuf>,
    recovery_path: Option<PathBuf>,
    recovery_offer_pending: bool,
    working_directory: PathBuf,
    next_layer_number: usize,
    active_tool: ShellToolKind,
    canvas_revision: u64,
    dirty_since_primary_save: bool,
    dirty_since_autosave: bool,
    last_change_at: Option<Instant>,
    pending_primary_save_job: Option<u64>,
    pending_autosave_job: Option<u64>,
    pending_recovery_load_job: Option<u64>,
    pending_document_load_job: Option<u64>,
    pending_export_job: Option<u64>,
    jobs: JobSystem,
    cached_canvas_raster: Option<Vec<u8>>,
    transform_session: Option<TransformSession>,
    interaction: Option<CanvasInteraction>,
}

#[derive(Debug, Clone)]
struct TransformSession {
    layer_id: common::LayerId,
    translate_x: i32,
    translate_y: i32,
    scale: f32,
}

#[derive(Debug, Clone)]
enum CanvasInteraction {
    Move {
        layer_id: common::LayerId,
        start_canvas_x: i32,
        start_canvas_y: i32,
        start_offset_x: i32,
        start_offset_y: i32,
    },
    Marquee {
        before: Option<common::CanvasRect>,
        before_inverted: bool,
        start_canvas_x: i32,
        start_canvas_y: i32,
    },
    Brush {
        mode: BrushToolMode,
        last_canvas_x: i32,
        last_canvas_y: i32,
        aggregate: Option<BrushStrokeRecord>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EditorHistoryEntry {
    label: String,
    operation: Option<EditorOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EditorOperation {
    BrushStroke(BrushStrokeRecord),
    TransformLayer(LayerTransformRecord),
    MoveLayer(MoveLayerRecord),
    Selection(RectangularSelectionRecord),
    MaskState(MaskStateRecord),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaskStateRecord {
    layer_id: common::LayerId,
    before_mask: Option<RasterMask>,
    after_mask: Option<RasterMask>,
    before_target: LayerEditTarget,
    after_target: LayerEditTarget,
}

impl MaskStateRecord {
    fn undo(&self, document: &mut Document) {
        Self::apply_state(document, self.layer_id, self.before_mask.clone(), self.before_target);
    }

    fn redo(&self, document: &mut Document) {
        Self::apply_state(document, self.layer_id, self.after_mask.clone(), self.after_target);
    }

    fn apply_state(
        document: &mut Document,
        layer_id: common::LayerId,
        mask: Option<RasterMask>,
        target: LayerEditTarget,
    ) {
        let Some(layer_index) = document.layer_index_by_id(layer_id) else {
            return;
        };
        if let Some(layer) = document.layer_mut(layer_index) {
            layer.mask = mask;
            if let Some(mask) = layer.mask.as_mut() {
                mask.dirty_tiles = mask.tiles.keys().copied().collect();
            }
        }
        let _ = document.set_active_layer(layer_index);
        let _ = document.set_active_edit_target(target);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobPriority {
    UserVisible,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveKind {
    Primary,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocumentLoadKind {
    Project,
    RasterImport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RasterFileFormat {
    Png,
    Jpeg,
    Webp,
}

#[derive(Debug)]
enum JobRequest {
    SaveDocument {
        job_id: u64,
        path: PathBuf,
        document: Document,
        kind: SaveKind,
        cleanup_recovery_path: Option<PathBuf>,
    },
    #[allow(dead_code)]
    LoadRecovery {
        job_id: u64,
        recovery_path: PathBuf,
        document_path: Option<PathBuf>,
        document_title: String,
    },
    LoadDocument {
        job_id: u64,
        path: PathBuf,
        kind: DocumentLoadKind,
    },
    ExportDocument {
        job_id: u64,
        path: PathBuf,
        document: Document,
        format: RasterFileFormat,
    },
}

#[derive(Debug)]
enum JobResult {
    SaveCompleted {
        job_id: u64,
        path: PathBuf,
        kind: SaveKind,
    },
    SaveFailed {
        job_id: u64,
        path: PathBuf,
        kind: SaveKind,
        error: String,
    },
    RecoveryLoaded {
        job_id: u64,
        recovery_path: PathBuf,
        document_path: Option<PathBuf>,
        document_title: String,
        document: Document,
    },
    RecoveryLoadFailed {
        job_id: u64,
        recovery_path: PathBuf,
        error: String,
    },
    DocumentLoaded {
        job_id: u64,
        path: PathBuf,
        kind: DocumentLoadKind,
        document: Document,
    },
    DocumentLoadFailed {
        job_id: u64,
        path: PathBuf,
        kind: DocumentLoadKind,
        error: String,
    },
    ExportCompleted {
        job_id: u64,
        path: PathBuf,
    },
    ExportFailed {
        job_id: u64,
        path: PathBuf,
        format: RasterFileFormat,
        error: String,
    },
}

fn project_file_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case(PROJECT_FILE_EXTENSION))
        .unwrap_or(false)
}

fn raster_format_from_path(path: &Path) -> Option<RasterFileFormat> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some(RasterFileFormat::Png),
        "jpg" | "jpeg" => Some(RasterFileFormat::Jpeg),
        "webp" => Some(RasterFileFormat::Webp),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct JobQueues {
    user_visible: VecDeque<JobRequest>,
    background: VecDeque<JobRequest>,
    shutdown: bool,
}

#[derive(Debug)]
struct JobSystem {
    queues: Arc<(Mutex<JobQueues>, Condvar)>,
    result_receiver: mpsc::Receiver<JobResult>,
    worker: Option<thread::JoinHandle<()>>,
    next_job_id: u64,
}

impl JobSystem {
    fn new() -> Self {
        let queues = Arc::new((Mutex::new(JobQueues::default()), Condvar::new()));
        let (result_sender, result_receiver) = mpsc::channel();
        let worker_queues = queues.clone();
        let worker = thread::spawn(move || worker_main(worker_queues, result_sender));

        Self {
            queues,
            result_receiver,
            worker: Some(worker),
            next_job_id: 1,
        }
    }

    fn enqueue(&mut self, priority: JobPriority, make_request: impl FnOnce(u64) -> JobRequest) -> u64 {
        let job_id = self.next_job_id;
        self.next_job_id += 1;

        let request = make_request(job_id);
        let (lock, condition) = &*self.queues;
        let mut queues = lock.lock().expect("job queue lock should not be poisoned");
        match priority {
            JobPriority::UserVisible => queues.user_visible.push_back(request),
            JobPriority::Background => queues.background.push_back(request),
        }
        condition.notify_one();
        job_id
    }

    fn try_recv(&self) -> Option<JobResult> {
        self.result_receiver.try_recv().ok()
    }
}

impl Drop for JobSystem {
    fn drop(&mut self) {
        let (lock, condition) = &*self.queues;
        if let Ok(mut queues) = lock.lock() {
            queues.shutdown = true;
            condition.notify_all();
        }

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn worker_main(
    queues: Arc<(Mutex<JobQueues>, Condvar)>,
    result_sender: mpsc::Sender<JobResult>,
) {
    loop {
        let request = {
            let (lock, condition) = &*queues;
            let mut queues = lock.lock().expect("job queue lock should not be poisoned");
            loop {
                if queues.shutdown {
                    return;
                }

                if let Some(request) = queues.user_visible.pop_front() {
                    break request;
                }

                if let Some(request) = queues.background.pop_front() {
                    break request;
                }

                queues = condition
                    .wait(queues)
                    .expect("job queue lock should not be poisoned while waiting");
            }
        };

        let result = match request {
            JobRequest::SaveDocument {
                job_id,
                path,
                document,
                kind,
                cleanup_recovery_path,
            } => match save_document_to_path(&path, &document)
                .with_context(|| format!("failed to save document to {}", path.display()))
            {
                Ok(()) => {
                    if let Some(recovery_path) = cleanup_recovery_path {
                        if let Err(error) = remove_file_if_exists(&recovery_path) {
                            tracing::warn!(%error, path = %recovery_path.display(), "failed to remove stale recovery file after save");
                        }
                    }
                    JobResult::SaveCompleted { job_id, path, kind }
                }
                Err(error) => JobResult::SaveFailed {
                    job_id,
                    path,
                    kind,
                    error: error.to_string(),
                },
            },
            JobRequest::LoadRecovery {
                job_id,
                recovery_path,
                document_path,
                document_title,
            } => match load_document_from_path(&recovery_path)
                .with_context(|| format!("failed to load recovery document from {}", recovery_path.display()))
            {
                Ok(document) => JobResult::RecoveryLoaded {
                    job_id,
                    recovery_path,
                    document_path,
                    document_title,
                    document,
                },
                Err(error) => JobResult::RecoveryLoadFailed {
                    job_id,
                    recovery_path,
                    error: error.to_string(),
                },
            },
            JobRequest::LoadDocument { job_id, path, kind } => {
                let result = match kind {
                    DocumentLoadKind::Project => load_document_from_path(&path)
                        .with_context(|| format!("failed to open project from {}", path.display())),
                    DocumentLoadKind::RasterImport => match raster_format_from_path(&path) {
                        Some(RasterFileFormat::Png) => import_png_from_path(&path),
                        Some(RasterFileFormat::Jpeg) => import_jpeg_from_path(&path),
                        Some(RasterFileFormat::Webp) => import_webp_from_path(&path),
                        None => Err(anyhow::anyhow!(
                            "unsupported import format for {}",
                            path.display()
                        )),
                    },
                };

                match result {
                    Ok(document) => JobResult::DocumentLoaded {
                        job_id,
                        path,
                        kind,
                        document,
                    },
                    Err(error) => JobResult::DocumentLoadFailed {
                        job_id,
                        path,
                        kind,
                        error: error.to_string(),
                    },
                }
            }
            JobRequest::ExportDocument {
                job_id,
                path,
                document,
                format,
            } => {
                let result = match format {
                    RasterFileFormat::Png => export_png_to_path(&path, &document),
                    RasterFileFormat::Jpeg => export_jpeg_to_path(&path, &document),
                    RasterFileFormat::Webp => export_webp_to_path(&path, &document),
                }
                .with_context(|| format!("failed to export document to {}", path.display()));

                match result {
                    Ok(()) => JobResult::ExportCompleted {
                        job_id,
                        path,
                    },
                    Err(error) => JobResult::ExportFailed {
                        job_id,
                        path,
                        format,
                        error: error.to_string(),
                    },
                }
            },
        };

        if result_sender.send(result).is_err() {
            return;
        }
    }
}

impl PhotoTuxController {
    fn new() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::new_with_working_directory(working_directory)
    }

    fn new_with_working_directory(working_directory: PathBuf) -> Self {
        let mut document = Document::new(1920, 1080);
        document.rename_layer(0, "Background");
        let background_tile = document
            .ensure_tile_for_pixel(0, 32, 32)
            .expect("background tile should be created");
        background_tile.pixels[3] = 255;
        document.add_layer("Sketch");
        let sketch_index = document.active_layer_index();
        let sketch_tile = document
            .ensure_tile_for_pixel(sketch_index, 180, 140)
            .expect("sketch tile should be created");
        sketch_tile.pixels[0] = 120;
        sketch_tile.pixels[3] = 255;
        document.add_layer("Highlights");
        let highlights_index = document.active_layer_index();
        let highlights_tile = document
            .ensure_tile_for_pixel(highlights_index, 260, 180)
            .expect("highlights tile should be created");
        highlights_tile.pixels[0] = 220;
        highlights_tile.pixels[1] = 220;
        highlights_tile.pixels[3] = 255;

        let mut history = HistoryStack::default();
        history.push(EditorHistoryEntry {
            label: "Open Document".to_string(),
            operation: None,
        });

        let mut controller = Self {
            document,
            history,
            foreground_color: [232, 236, 243, 255],
            background_color: [27, 29, 33, 255],
            status_message: "Ready".to_string(),
            document_title: "untitled.ptx".to_string(),
            document_path: None,
            recovery_path: None,
            recovery_offer_pending: false,
            working_directory,
            next_layer_number: 4,
            active_tool: ShellToolKind::Brush,
            canvas_revision: 1,
            dirty_since_primary_save: false,
            dirty_since_autosave: false,
            last_change_at: None,
            pending_primary_save_job: None,
            pending_autosave_job: None,
            pending_recovery_load_job: None,
            pending_document_load_job: None,
            pending_export_job: None,
            jobs: JobSystem::new(),
            cached_canvas_raster: None,
            transform_session: None,
            interaction: None,
        };
        controller.refresh_recovery_path();
        controller.recovery_offer_pending = controller
            .recovery_path
            .as_ref()
            .map(|path| path.exists())
            .unwrap_or(false);
        controller
    }

    fn active_layer_name(&self) -> String {
        self.document.active_layer().name.clone()
    }

    fn push_history(&mut self, entry: impl Into<String>) {
        self.history.push(EditorHistoryEntry {
            label: entry.into(),
            operation: None,
        });
    }

    fn push_operation(&mut self, label: impl Into<String>, operation: EditorOperation) {
        self.history.push(EditorHistoryEntry {
            label: label.into(),
            operation: Some(operation),
        });
    }

    fn bump_canvas_revision(&mut self) {
        self.canvas_revision = self.canvas_revision.saturating_add(1);
    }

    fn current_brush_settings(&self, mode: BrushToolMode) -> BrushSettings {
        BrushSettings {
            radius: 12.0,
            hardness: 0.8,
            opacity: 1.0,
            spacing: 6.0,
            color: match mode {
                BrushToolMode::Paint => self.foreground_color,
                BrushToolMode::Erase => [0, 0, 0, 255],
            },
        }
    }

    fn apply_active_layer_stroke_segment(
        &mut self,
        mode: BrushToolMode,
        points: &[(f32, f32)],
    ) -> Option<BrushStrokeRecord> {
        let layer_index = self.document.active_layer_index();
        let settings = self.current_brush_settings(mode);
        let target = self.document.active_edit_target();
        
        if self.cached_canvas_raster.is_none() {
            self.cached_canvas_raster = Some(file_io::flatten_document_rgba(&self.document));
        }

        let record = BrushTool::apply_stroke(&mut self.document, layer_index, points, settings, mode, target)?;
        
        if let Some(ref mut cached) = self.cached_canvas_raster {
            let layer = &self.document.layers[layer_index];
            for change in &record.changes {
                let (tx, ty) = self.document.tile_origin(change.coord());
                let rect = common::CanvasRect {
                    x: tx as i32 + layer.offset_x,
                    y: ty as i32 + layer.offset_y,
                    width: self.document.tile_size,
                    height: self.document.tile_size,
                };
                file_io::update_flattened_region_rgba(&self.document, cached, rect);
            }
        }

        self.bump_canvas_revision();
        Some(record)
    }

    fn merge_brush_records(aggregate: &mut BrushStrokeRecord, segment: BrushStrokeRecord) {
        aggregate.dab_count += segment.dab_count;

        for change in segment.changes {
            if let Some(existing) = aggregate
                .changes
                .iter_mut()
                .find(|existing| existing.layer_id() == change.layer_id() && existing.coord() == change.coord())
            {
                match (existing, change) {
                    (
                        BrushChange::Pixels { after, .. },
                        BrushChange::Pixels { after: next_after, .. },
                    ) => {
                        *after = next_after;
                    }
                    (
                        BrushChange::Mask { after, .. },
                        BrushChange::Mask { after: next_after, .. },
                    ) => {
                        *after = next_after;
                    }
                    (_, change) => aggregate.changes.push(change),
                }
            } else {
                aggregate.changes.push(change);
            }
        }
    }

    fn active_edit_target_name(&self) -> &'static str {
        match self.document.active_edit_target() {
            LayerEditTarget::LayerPixels => "Layer Pixels",
            LayerEditTarget::LayerMask => "Layer Mask",
        }
    }

    fn push_mask_state_operation(
        &mut self,
        label: impl Into<String>,
        layer_id: common::LayerId,
        before_mask: Option<RasterMask>,
        after_mask: Option<RasterMask>,
        before_target: LayerEditTarget,
        after_target: LayerEditTarget,
    ) {
        self.push_operation(
            label,
            EditorOperation::MaskState(MaskStateRecord {
                layer_id,
                before_mask,
                after_mask,
                before_target,
                after_target,
            }),
        );
    }

    fn layer_items(&self) -> Vec<LayerPanelItem> {
        self.document
            .layers
            .iter()
            .enumerate()
            .rev()
            .map(|(index, layer)| LayerPanelItem {
                index,
                name: layer.name.clone(),
                visible: layer.visible,
                opacity_percent: layer.opacity_percent,
                has_mask: layer.mask.is_some(),
                mask_enabled: layer.mask.as_ref().map(|mask| mask.enabled).unwrap_or(false),
                mask_target_active: index == self.document.active_layer_index()
                    && self.document.active_edit_target() == LayerEditTarget::LayerMask,
                is_active: index == self.document.active_layer_index(),
            })
            .collect()
    }

    fn move_active_layer_by(&mut self, delta: isize) {
        let current = self.document.active_layer_index() as isize;
        let target = (current + delta).clamp(0, self.document.layer_count().saturating_sub(1) as isize);
        if current == target {
            return;
        }

        let active_name = self.active_layer_name();
        if self.document.move_layer(current as usize, target as usize) {
            self.bump_canvas_revision();
            self.mark_document_dirty();
            self.push_history(format!("Move Layer {}", active_name));
        }
    }

    fn active_layer_bounds(&self) -> Option<CanvasRect> {
        self.document.layer_canvas_bounds(self.document.active_layer_index())
    }

    fn primary_document_path(&self) -> PathBuf {
        self.document_path
            .clone()
            .unwrap_or_else(|| self.working_directory.join(self.save_file_name()))
    }

    fn refresh_recovery_path(&mut self) {
        self.recovery_path = Some(recovery_path_for_project_path(&self.primary_document_path()));
    }

    #[allow(dead_code)]
    fn enqueue_recovery_load_if_present(&mut self) {
        let Some(recovery_path) = self.recovery_path.clone() else {
            return;
        };

        if !recovery_path.exists() || self.pending_recovery_load_job.is_some() {
            return;
        }

        self.status_message = "Recovery file detected".to_string();
        let document_path = self.document_path.clone();
        let document_title = self.document_title.clone();
        self.pending_recovery_load_job = Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
            JobRequest::LoadRecovery {
                job_id,
                recovery_path,
                document_path,
                document_title,
            }
        }));
    }

    fn mark_document_dirty_at(&mut self, now: Instant) {
        self.dirty_since_primary_save = true;
        self.dirty_since_autosave = true;
        self.last_change_at = Some(now);
        self.cached_canvas_raster = None; // Invalidate cached render
        if self.pending_primary_save_job.is_none() {
            self.status_message = "Modified".to_string();
        }
    }

    fn mark_document_dirty(&mut self) {
        self.mark_document_dirty_at(Instant::now());
    }

    fn has_pending_user_visible_file_job(&self) -> bool {
        self.pending_primary_save_job.is_some()
            || self.pending_recovery_load_job.is_some()
            || self.pending_document_load_job.is_some()
            || self.pending_export_job.is_some()
    }

    fn reset_history_to(&mut self, label: impl Into<String>) {
        self.history = HistoryStack::default();
        self.history.push(EditorHistoryEntry {
            label: label.into(),
            operation: None,
        });
    }

    fn recompute_next_layer_number(&mut self) {
        let highest_explicit_layer = self
            .document
            .layers
            .iter()
            .filter_map(|layer| {
                layer
                    .name
                    .strip_prefix("Layer ")
                    .and_then(|suffix| suffix.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(self.document.layer_count());

        self.next_layer_number = highest_explicit_layer
            .saturating_add(1)
            .max(self.document.layer_count().saturating_add(1));
    }

    fn replace_document_after_load(
        &mut self,
        document: Document,
        document_title: String,
        document_path: Option<PathBuf>,
        working_directory: PathBuf,
        dirty_since_primary_save: bool,
        dirty_since_autosave: bool,
        last_change_at: Option<Instant>,
        history_label: &str,
        status_message: String,
    ) {
        self.document = document;
        self.document_title = document_title;
        self.document_path = document_path;
        self.working_directory = working_directory;
        self.transform_session = None;
        self.interaction = None;
        self.active_tool = ShellToolKind::Brush;
        self.refresh_recovery_path();
        self.recovery_offer_pending = false;
        self.dirty_since_primary_save = dirty_since_primary_save;
        self.dirty_since_autosave = dirty_since_autosave;
        self.last_change_at = last_change_at;
        self.recompute_next_layer_number();
        self.reset_history_to(history_label);
        self.bump_canvas_revision();
        self.status_message = status_message;
    }

    fn enqueue_primary_save(&mut self, path: PathBuf) {
        if self.pending_primary_save_job.is_some() {
            return;
        }

        let recovery_path = self.recovery_path.clone();
        let document = self.document.clone();
        self.status_message = format!("Saving {}", path.display());
        self.pending_primary_save_job = Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
            JobRequest::SaveDocument {
                job_id,
                path,
                document,
                kind: SaveKind::Primary,
                cleanup_recovery_path: recovery_path,
            }
        }));
    }

    fn enqueue_autosave(&mut self) {
        if self.pending_autosave_job.is_some() || !self.dirty_since_autosave {
            return;
        }

        self.refresh_recovery_path();
        let Some(recovery_path) = self.recovery_path.clone() else {
            return;
        };

        let document = self.document.clone();
        self.status_message = format!("Autosaving {}", recovery_path.display());
        self.pending_autosave_job = Some(self.jobs.enqueue(JobPriority::Background, move |job_id| {
            JobRequest::SaveDocument {
                job_id,
                path: recovery_path,
                document,
                kind: SaveKind::Recovery,
                cleanup_recovery_path: None,
            }
        }));
    }

    fn apply_job_result(&mut self, result: JobResult) {
        match result {
            JobResult::SaveCompleted { job_id, path, kind } => match kind {
                SaveKind::Primary => {
                    if self.pending_primary_save_job == Some(job_id) {
                        self.pending_primary_save_job = None;
                        self.document_path = Some(path.clone());
                        if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                            self.document_title = file_name.to_string();
                        }
                        self.refresh_recovery_path();
                        self.recovery_offer_pending = false;
                        self.dirty_since_primary_save = false;
                        self.dirty_since_autosave = false;
                        self.last_change_at = None;
                        self.status_message = format!("Saved {}", path.display());
                    }
                }
                SaveKind::Recovery => {
                    if self.pending_autosave_job == Some(job_id) {
                        self.pending_autosave_job = None;
                        self.dirty_since_autosave = false;
                        self.status_message = format!("Recovered state written to {}", path.display());
                    }
                }
            },
            JobResult::SaveFailed { job_id, path, kind, error } => {
                tracing::error!(%error, path = %path.display(), ?kind, "background save failed");
                match kind {
                    SaveKind::Primary if self.pending_primary_save_job == Some(job_id) => {
                        self.pending_primary_save_job = None;
                    }
                    SaveKind::Recovery if self.pending_autosave_job == Some(job_id) => {
                        self.pending_autosave_job = None;
                    }
                    _ => {}
                }
                self.status_message = format!("Save failed: {}", error);
            }
            JobResult::RecoveryLoaded {
                job_id,
                recovery_path,
                document_path,
                document_title,
                document,
            } => {
                if self.pending_recovery_load_job == Some(job_id) {
                    self.pending_recovery_load_job = None;
                    self.recovery_offer_pending = false;
                    self.document = document;
                    self.document_path = document_path;
                    self.document_title = document_title;
                    self.recovery_path = Some(recovery_path.clone());
                    self.dirty_since_primary_save = true;
                    self.dirty_since_autosave = false;
                    self.last_change_at = None;
                    self.bump_canvas_revision();
                    self.push_history("Recovered Autosave");
                    self.status_message = format!("Recovered document from {}", recovery_path.display());
                }
            }
            JobResult::RecoveryLoadFailed {
                job_id,
                recovery_path,
                error,
            } => {
                if self.pending_recovery_load_job == Some(job_id) {
                    self.pending_recovery_load_job = None;
                    self.recovery_offer_pending = false;
                    tracing::error!(%error, path = %recovery_path.display(), "recovery load failed");
                    self.status_message = format!("Recovery load failed: {}", error);
                }
            }
            JobResult::DocumentLoaded {
                job_id,
                path,
                kind,
                document,
            } => {
                if self.pending_document_load_job == Some(job_id) {
                    self.pending_document_load_job = None;
                    let working_directory = path
                        .parent()
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| self.working_directory.clone());

                    match kind {
                        DocumentLoadKind::Project => {
                            let document_title = path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("untitled.ptx")
                                .to_string();
                            self.replace_document_after_load(
                                document,
                                document_title,
                                Some(path.clone()),
                                working_directory,
                                false,
                                false,
                                None,
                                "Open Document",
                                format!("Opened {}", path.display()),
                            );
                        }
                        DocumentLoadKind::RasterImport => {
                            let stem = path
                                .file_stem()
                                .and_then(|name| name.to_str())
                                .unwrap_or("imported");
                            self.replace_document_after_load(
                                document,
                                format!("{}.{}", stem, PROJECT_FILE_EXTENSION),
                                None,
                                working_directory,
                                true,
                                true,
                                Some(Instant::now()),
                                "Import Image",
                                format!("Imported {}", path.display()),
                            );
                        }
                    }
                }
            }
            JobResult::DocumentLoadFailed {
                job_id,
                path,
                kind,
                error,
            } => {
                if self.pending_document_load_job == Some(job_id) {
                    self.pending_document_load_job = None;
                    tracing::error!(%error, path = %path.display(), ?kind, "document load failed");
                    self.status_message = match kind {
                        DocumentLoadKind::Project => format!("Open failed: {}", error),
                        DocumentLoadKind::RasterImport => format!("Import failed: {}", error),
                    };
                }
            }
            JobResult::ExportCompleted {
                job_id,
                path,
            } => {
                if self.pending_export_job == Some(job_id) {
                    self.pending_export_job = None;
                    self.status_message = format!("Exported {}", path.display());
                }
            }
            JobResult::ExportFailed {
                job_id,
                path,
                format,
                error,
            } => {
                if self.pending_export_job == Some(job_id) {
                    self.pending_export_job = None;
                    tracing::error!(%error, path = %path.display(), ?format, "document export failed");
                    self.status_message = format!("Export failed: {}", error);
                }
            }
        }
    }

    fn poll_background_tasks_at(&mut self, now: Instant) {
        while let Some(result) = self.jobs.try_recv() {
            self.apply_job_result(result);
        }

        if self.pending_primary_save_job.is_none()
            && self.pending_recovery_load_job.is_none()
            && self.pending_document_load_job.is_none()
            && self.dirty_since_autosave
            && self.pending_autosave_job.is_none()
            && self
                .last_change_at
                .map(|last_change| now.duration_since(last_change) >= AUTOSAVE_IDLE_INTERVAL)
                .unwrap_or(false)
        {
            self.enqueue_autosave();
        }
    }

    fn save_file_name(&self) -> String {
        if self.document_title.ends_with(&format!(".{PROJECT_FILE_EXTENSION}")) {
            self.document_title.clone()
        } else {
            format!("{}.{}", self.document_title, PROJECT_FILE_EXTENSION)
        }
    }

    #[cfg(test)]
    fn save_document_in_directory(&mut self, base_dir: &std::path::Path) -> anyhow::Result<PathBuf> {
        let target_path = self
            .document_path
            .clone()
            .unwrap_or_else(|| base_dir.join(self.save_file_name()));
        save_document_to_path(&target_path, &self.document)
            .with_context(|| format!("failed to save document to {}", target_path.display()))?;
        self.document_path = Some(target_path.clone());
        if let Some(file_name) = target_path.file_name().and_then(|name| name.to_str()) {
            self.document_title = file_name.to_string();
        }
        self.working_directory = base_dir.to_path_buf();
        self.refresh_recovery_path();
        if let Some(recovery_path) = &self.recovery_path {
            remove_file_if_exists(recovery_path)?;
        }
        self.dirty_since_primary_save = false;
        self.dirty_since_autosave = false;
        self.last_change_at = None;
        self.status_message = format!("Saved {}", target_path.display());
        Ok(target_path)
    }

    fn cycle_active_layer_blend_mode(&mut self, step: isize) {
        const MODES: [BlendMode; 6] = [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Darken,
            BlendMode::Lighten,
        ];

        let active_index = self.document.active_layer_index();
        let current_mode = self.document.active_layer().blend_mode;
        let current_index = MODES
            .iter()
            .position(|mode| *mode == current_mode)
            .unwrap_or(0) as isize;
        let next_index = (current_index + step).rem_euclid(MODES.len() as isize) as usize;
        self.document.set_layer_blend_mode(active_index, MODES[next_index]);
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_history(format!("Set Blend Mode {:?}", MODES[next_index]));
    }

    fn begin_transform_session_if_needed(&mut self) {
        if self.transform_session.is_some() {
            return;
        }

        let layer_index = self.document.active_layer_index();
        let Some(layer) = self.document.layer(layer_index) else {
            return;
        };
        if self.document.layer_canvas_bounds(layer_index).is_none() {
            return;
        }

        self.transform_session = Some(TransformSession {
            layer_id: layer.id,
            translate_x: 0,
            translate_y: 0,
            scale: 1.0,
        });
        self.bump_canvas_revision();
    }

    fn transform_preview_rect(&self) -> Option<CanvasRect> {
        let session = self.transform_session.as_ref()?;
        let layer_index = self.document.layer_index_by_id(session.layer_id)?;
        SimpleTransformTool::preview_bounds(
            &self.document,
            layer_index,
            session.scale,
            session.translate_x,
            session.translate_y,
        )
    }

    fn preview_canvas_raster(&self) -> CanvasRaster {
        let mut preview_document = self.document.clone();
        if let Some(session) = &self.transform_session {
            if let Some(layer_index) = preview_document.layer_index_by_id(session.layer_id) {
                let _ = SimpleTransformTool::transform_layer(
                    &mut preview_document,
                    layer_index,
                    session.scale,
                    session.translate_x,
                    session.translate_y,
                );
            }
        }

        CanvasRaster {
            size: preview_document.canvas_size,
            pixels: flatten_document_rgba(&preview_document),
        }
    }
}

impl ShellController for PhotoTuxController {
    fn snapshot(&self) -> ShellSnapshot {
        let active_layer = self.document.active_layer();
        ShellSnapshot {
            document_title: self.document_title.clone(),
            project_path: self.document_path.clone(),
            dirty: self.dirty_since_primary_save,
            recovery_offer_pending: self.recovery_offer_pending,
            recovery_path: self.recovery_path.clone(),
            status_message: self.status_message.clone(),
            canvas_size: self.document.canvas_size,
            canvas_revision: self.canvas_revision,
            active_tool_name: self.active_tool.label().to_string(),
            active_tool: self.active_tool,
            layers: self.layer_items(),
            active_layer_name: active_layer.name.clone(),
            active_layer_opacity_percent: active_layer.opacity_percent,
            active_layer_visible: active_layer.visible,
            active_layer_blend_mode: format!("{:?}", active_layer.blend_mode),
            active_layer_has_mask: active_layer.mask.is_some(),
            active_layer_mask_enabled: active_layer.mask.as_ref().map(|mask| mask.enabled).unwrap_or(false),
            active_edit_target_name: self.active_edit_target_name().to_string(),
            active_layer_bounds: self.active_layer_bounds(),
            transform_preview_rect: self.transform_preview_rect(),
            transform_active: self.transform_session.is_some(),
            transform_scale_percent: self
                .transform_session
                .as_ref()
                .map(|session| (session.scale * 100.0).round() as u32)
                .unwrap_or(100),
            selection_rect: self.document.selection(),
            selection_inverted: self.document.selection_inverted(),
            foreground_color: self.foreground_color,
            background_color: self.background_color,
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            history_entries: self
                .history
                .undo_entries()
                .iter()
                .rev()
                .map(|entry| entry.label.clone())
                .collect(),
        }
    }

    fn canvas_raster(&self) -> CanvasRaster {
        if self.transform_session.is_some() {
            self.preview_canvas_raster()
        } else {
            if let Some(ref cached) = self.cached_canvas_raster {
                return CanvasRaster {
                    size: self.document.canvas_size,
                    pixels: cached.clone(),
                };
            }
            // Fallback for non-brush cases where cache was invalidated but not yet rebuilt
            CanvasRaster {
                size: self.document.canvas_size,
                pixels: flatten_document_rgba(&self.document),
            }
        }
    }

    fn add_layer(&mut self) {
        let layer_name = format!("Layer {}", self.next_layer_number);
        self.next_layer_number += 1;
        self.document.add_layer(layer_name.clone());
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_history(format!("Add Layer {}", layer_name));
    }

    fn duplicate_active_layer(&mut self) {
        let active_index = self.document.active_layer_index();
        let active_name = self.active_layer_name();
        if self.document.duplicate_layer(active_index).is_some() {
            self.bump_canvas_revision();
            self.mark_document_dirty();
            self.push_history(format!("Duplicate Layer {}", active_name));
        }
    }

    fn delete_active_layer(&mut self) {
        let active_index = self.document.active_layer_index();
        let active_name = self.active_layer_name();
        if self.document.delete_layer(active_index) {
            self.bump_canvas_revision();
            self.mark_document_dirty();
            self.push_history(format!("Delete Layer {}", active_name));
        }
    }

    fn add_active_layer_mask(&mut self) {
        let layer_index = self.document.active_layer_index();
        let layer_id = self.document.active_layer().id;
        let before_target = self.document.active_edit_target();
        let before_mask = self.document.layer_mask(layer_index).cloned();
        if !self.document.add_layer_mask(layer_index) {
            return;
        }
        let _ = self.document.set_active_edit_target(LayerEditTarget::LayerMask);
        let after_target = self.document.active_edit_target();
        let after_mask = self.document.layer_mask(layer_index).cloned();
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_mask_state_operation(
            format!("Add Layer Mask {}", self.active_layer_name()),
            layer_id,
            before_mask,
            after_mask,
            before_target,
            after_target,
        );
    }

    fn remove_active_layer_mask(&mut self) {
        let layer_index = self.document.active_layer_index();
        let layer_id = self.document.active_layer().id;
        let before_target = self.document.active_edit_target();
        let before_mask = self.document.layer_mask(layer_index).cloned();
        if !self.document.remove_layer_mask(layer_index) {
            return;
        }
        let after_target = self.document.active_edit_target();
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_mask_state_operation(
            format!("Delete Layer Mask {}", self.active_layer_name()),
            layer_id,
            before_mask,
            None,
            before_target,
            after_target,
        );
    }

    fn toggle_active_layer_mask_enabled(&mut self) {
        let layer_index = self.document.active_layer_index();
        let layer_id = self.document.active_layer().id;
        let Some(mask) = self.document.layer_mask(layer_index) else {
            return;
        };
        let before_mask = Some(mask.clone());
        let enabled = !mask.enabled;
        let before_target = self.document.active_edit_target();
        if !self.document.set_layer_mask_enabled(layer_index, enabled) {
            return;
        }
        let after_mask = self.document.layer_mask(layer_index).cloned();
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_mask_state_operation(
            format!(
                "{} Layer Mask {}",
                if enabled { "Enable" } else { "Disable" },
                self.active_layer_name()
            ),
            layer_id,
            before_mask,
            after_mask,
            before_target,
            before_target,
        );
    }

    fn edit_active_layer_pixels(&mut self) {
        if self.document.active_edit_target() == LayerEditTarget::LayerPixels {
            return;
        }
        if self.document.set_active_edit_target(LayerEditTarget::LayerPixels) {
            self.mark_document_dirty();
            self.status_message = format!("Editing layer pixels for {}", self.active_layer_name());
        }
    }

    fn edit_active_layer_mask(&mut self) {
        if self.document.active_edit_target() == LayerEditTarget::LayerMask {
            return;
        }
        if self.document.set_active_edit_target(LayerEditTarget::LayerMask) {
            self.mark_document_dirty();
            self.status_message = format!("Editing layer mask for {}", self.active_layer_name());
        }
    }

    fn select_layer(&mut self, index: usize) {
        let _ = self.document.set_active_layer(index);
    }

    fn toggle_layer_visibility(&mut self, index: usize) {
        if let Some(layer) = self.document.layer(index) {
            let visible = !layer.visible;
            let layer_name = layer.name.clone();
            self.document.set_layer_visibility(index, visible);
            self.bump_canvas_revision();
            self.mark_document_dirty();
            self.push_history(format!("Toggle Visibility {}", layer_name));
        }
    }

    fn increase_active_layer_opacity(&mut self) {
        let active_index = self.document.active_layer_index();
        let next_opacity = (self.document.active_layer().opacity_percent + 10).min(100);
        self.document.set_layer_opacity(active_index, next_opacity);
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_history(format!("Increase Opacity {}", self.active_layer_name()));
    }

    fn decrease_active_layer_opacity(&mut self) {
        let active_index = self.document.active_layer_index();
        let next_opacity = self.document.active_layer().opacity_percent.saturating_sub(10);
        self.document.set_layer_opacity(active_index, next_opacity);
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_history(format!("Decrease Opacity {}", self.active_layer_name()));
    }

    fn next_active_layer_blend_mode(&mut self) {
        self.cycle_active_layer_blend_mode(1);
    }

    fn previous_active_layer_blend_mode(&mut self) {
        self.cycle_active_layer_blend_mode(-1);
    }

    fn move_active_layer_up(&mut self) {
        self.move_active_layer_by(1);
    }

    fn move_active_layer_down(&mut self) {
        self.move_active_layer_by(-1);
    }

    fn swap_colors(&mut self) {
        std::mem::swap(&mut self.foreground_color, &mut self.background_color);
        self.push_history("Swap Colors");
    }

    fn reset_colors(&mut self) {
        self.foreground_color = [232, 236, 243, 255];
        self.background_color = [27, 29, 33, 255];
        self.push_history("Reset Colors");
    }

    fn clear_selection(&mut self) {
        let before = self.document.selection();
        let before_inverted = self.document.selection_inverted();
        if before.is_none() {
            return;
        }

        self.document.clear_selection();
        self.mark_document_dirty();
        self.push_operation(
            "Clear Selection",
            EditorOperation::Selection(RectangularSelectionRecord {
                before,
                before_inverted,
                after: None,
                after_inverted: false,
            }),
        );
    }

    fn invert_selection(&mut self) {
        let before = self.document.selection();
        let before_inverted = self.document.selection_inverted();
        if !self.document.invert_selection() {
            return;
        }

        self.mark_document_dirty();
        self.push_operation(
            "Invert Selection",
            EditorOperation::Selection(RectangularSelectionRecord {
                before,
                before_inverted,
                after: self.document.selection(),
                after_inverted: self.document.selection_inverted(),
            }),
        );
    }

    fn begin_transform(&mut self) {
        self.begin_transform_session_if_needed();
    }

    fn scale_transform_up(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.scale = (session.scale + 0.1).min(4.0);
        self.bump_canvas_revision();
    }

    fn scale_transform_down(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.scale = (session.scale - 0.1).max(0.1);
        self.bump_canvas_revision();
    }

    fn commit_transform(&mut self) {
        let Some(session) = self.transform_session.take() else {
            return;
        };
        let Some(layer_index) = self.document.layer_index_by_id(session.layer_id) else {
            return;
        };

        if let Some(record) = SimpleTransformTool::transform_layer(
            &mut self.document,
            layer_index,
            session.scale,
            session.translate_x,
            session.translate_y,
        ) {
            self.bump_canvas_revision();
            self.mark_document_dirty();
            self.push_operation("Transform Layer", EditorOperation::TransformLayer(record));
        } else {
            self.bump_canvas_revision();
        }
    }

    fn cancel_transform(&mut self) {
        if self.transform_session.take().is_some() {
            self.bump_canvas_revision();
        }
    }

    fn undo(&mut self) {
        let Some(entry) = self.history.undo().cloned() else {
            return;
        };

        if let Some(operation) = entry.operation {
            match operation {
                EditorOperation::BrushStroke(record) => {
                    record.undo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::TransformLayer(record) => {
                    record.undo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::MoveLayer(record) => {
                    record.undo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::Selection(record) => {
                    record.undo(&mut self.document);
                    self.mark_document_dirty();
                }
                EditorOperation::MaskState(record) => {
                    record.undo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
            }
        }
    }

    fn redo(&mut self) {
        let Some(entry) = self.history.redo().cloned() else {
            return;
        };

        if let Some(operation) = entry.operation {
            match operation {
                EditorOperation::BrushStroke(record) => {
                    record.redo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::TransformLayer(record) => {
                    record.redo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::MoveLayer(record) => {
                    record.redo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::Selection(record) => {
                    record.redo(&mut self.document);
                    self.mark_document_dirty();
                }
                EditorOperation::MaskState(record) => {
                    record.redo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
            }
        }
    }

    fn save_document(&mut self) {
        let Some(target_path) = self.document_path.clone() else {
            self.status_message = "Save As required before the first project save".to_string();
            return;
        };

        self.enqueue_primary_save(target_path);
    }

    fn save_document_as(&mut self, path: PathBuf) {
        if self.has_pending_user_visible_file_job() {
            self.status_message = "Another file operation is already in progress".to_string();
            return;
        }

        if !project_file_path(&path) {
            self.status_message = format!(
                "Save failed: expected .{} project path",
                PROJECT_FILE_EXTENSION
            );
            return;
        }

        self.enqueue_primary_save(path);
    }

    fn load_recovery_document(&mut self) {
        let Some(recovery_path) = self.recovery_path.clone() else {
            self.recovery_offer_pending = false;
            self.status_message = "No recovery file available".to_string();
            return;
        };

        if self.pending_recovery_load_job.is_some() {
            return;
        }

        if !recovery_path.exists() {
            self.recovery_offer_pending = false;
            self.status_message = "Recovery file no longer exists".to_string();
            return;
        }

        self.recovery_offer_pending = false;
        self.status_message = format!("Loading recovery from {}", recovery_path.display());
        let document_path = self.document_path.clone();
        let document_title = self.document_title.clone();
        self.pending_recovery_load_job = Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
            JobRequest::LoadRecovery {
                job_id,
                recovery_path,
                document_path,
                document_title,
            }
        }));
    }

    fn discard_recovery_document(&mut self) {
        self.recovery_offer_pending = false;

        let Some(recovery_path) = self.recovery_path.clone() else {
            return;
        };

        match remove_file_if_exists(&recovery_path) {
            Ok(()) => {
                self.status_message = format!("Discarded recovery file {}", recovery_path.display());
            }
            Err(error) => {
                tracing::warn!(%error, path = %recovery_path.display(), "failed to discard recovery file");
                self.status_message = format!("Failed to discard recovery file: {}", error);
            }
        }
    }

    fn open_document(&mut self, path: PathBuf) {
        if self.has_pending_user_visible_file_job() {
            self.status_message = "Another file operation is already in progress".to_string();
            return;
        }

        if !project_file_path(&path) {
            self.status_message = format!("Open failed: expected .{} project file", PROJECT_FILE_EXTENSION);
            return;
        }

        self.status_message = format!("Opening {}", path.display());
        self.pending_document_load_job = Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
            JobRequest::LoadDocument {
                job_id,
                path,
                kind: DocumentLoadKind::Project,
            }
        }));
    }

    fn import_image(&mut self, path: PathBuf) {
        if self.has_pending_user_visible_file_job() {
            self.status_message = "Another file operation is already in progress".to_string();
            return;
        }

        if raster_format_from_path(&path).is_none() {
            self.status_message = "Import failed: unsupported image format".to_string();
            return;
        }

        self.status_message = format!("Importing {}", path.display());
        self.pending_document_load_job = Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
            JobRequest::LoadDocument {
                job_id,
                path,
                kind: DocumentLoadKind::RasterImport,
            }
        }));
    }

    fn export_document(&mut self, path: PathBuf) {
        if self.has_pending_user_visible_file_job() {
            self.status_message = "Another file operation is already in progress".to_string();
            return;
        }

        let Some(format) = raster_format_from_path(&path) else {
            self.status_message = "Export failed: unsupported image format".to_string();
            return;
        };

        let document = self.document.clone();
        self.status_message = format!("Exporting {}", path.display());
        self.pending_export_job = Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
            JobRequest::ExportDocument {
                job_id,
                path,
                document,
                format,
            }
        }));
    }

    fn poll_background_tasks(&mut self) {
        self.poll_background_tasks_at(Instant::now());
    }

    fn select_tool(&mut self, tool: ShellToolKind) {
        self.active_tool = tool;
    }

    fn begin_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32) {
        match self.active_tool {
            ShellToolKind::Move => {
                let (start_offset_x, start_offset_y) = self
                    .document
                    .layer_offset(self.document.active_layer_index())
                    .unwrap_or((0, 0));
                self.interaction = Some(CanvasInteraction::Move {
                    layer_id: self.document.active_layer().id,
                    start_canvas_x: canvas_x,
                    start_canvas_y: canvas_y,
                    start_offset_x,
                    start_offset_y,
                });
            }
            ShellToolKind::RectangularMarquee => {
                self.interaction = Some(CanvasInteraction::Marquee {
                    before: self.document.selection(),
                    before_inverted: self.document.selection_inverted(),
                    start_canvas_x: canvas_x,
                    start_canvas_y: canvas_y,
                });
            }
            ShellToolKind::Transform => {
                self.begin_transform_session_if_needed();
                if let Some(session) = &self.transform_session {
                    self.interaction = Some(CanvasInteraction::Move {
                        layer_id: session.layer_id,
                        start_canvas_x: canvas_x,
                        start_canvas_y: canvas_y,
                        start_offset_x: session.translate_x,
                        start_offset_y: session.translate_y,
                    });
                }
            }
            ShellToolKind::Brush | ShellToolKind::Eraser => {
                let mode = if self.active_tool == ShellToolKind::Brush {
                    BrushToolMode::Paint
                } else {
                    BrushToolMode::Erase
                };
                let aggregate =
                    self.apply_active_layer_stroke_segment(mode, &[(canvas_x as f32, canvas_y as f32)]);
                if aggregate.is_some() {
                    self.dirty_since_primary_save = true;
                    self.dirty_since_autosave = true;
                    self.last_change_at = Some(std::time::Instant::now());
                }
                self.interaction = Some(CanvasInteraction::Brush {
                    mode,
                    last_canvas_x: canvas_x,
                    last_canvas_y: canvas_y,
                    aggregate,
                });
            }
            _ => {}
        }
    }

    fn update_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32) {
        let Some(interaction) = self.interaction.take() else {
            return;
        };

        self.interaction = match interaction {
            CanvasInteraction::Move {
                layer_id,
                start_canvas_x,
                start_canvas_y,
                start_offset_x,
                start_offset_y,
            } => {
                let delta_x = canvas_x - start_canvas_x;
                let delta_y = canvas_y - start_canvas_y;
                if self.active_tool == ShellToolKind::Transform {
                    if let Some(session) = &mut self.transform_session {
                        session.translate_x = start_offset_x + delta_x;
                        session.translate_y = start_offset_y + delta_y;
                    }
                } else {
                    let layer_index = self.document.active_layer_index();
                    let old_bounds = self.document.layer_canvas_bounds(layer_index);

                    let _ = self.document.set_layer_offset(
                        layer_index,
                        start_offset_x + delta_x,
                        start_offset_y + delta_y,
                    );
                    
                    let new_bounds = self.document.layer_canvas_bounds(layer_index);

                    if self.cached_canvas_raster.is_none() {
                        self.cached_canvas_raster = Some(file_io::flatten_document_rgba(&self.document));
                    } else if let Some(ref mut cached) = self.cached_canvas_raster {
                        if let Some(rect) = old_bounds {
                            file_io::update_flattened_region_rgba(&self.document, cached, rect);
                        }
                        if let Some(rect) = new_bounds {
                            file_io::update_flattened_region_rgba(&self.document, cached, rect);
                        }
                    }

                    self.dirty_since_primary_save = true;
                    self.dirty_since_autosave = true;
                    self.last_change_at = Some(std::time::Instant::now());
                }
                self.bump_canvas_revision();
                Some(CanvasInteraction::Move {
                    layer_id,
                    start_canvas_x,
                    start_canvas_y,
                    start_offset_x,
                    start_offset_y,
                })
            }
            CanvasInteraction::Marquee {
                before,
                before_inverted,
                start_canvas_x,
                start_canvas_y,
            } => {
                if let Some(rect) =
                    RectangularMarqueeTool::preview_rect(start_canvas_x, start_canvas_y, canvas_x, canvas_y)
                {
                    self.document.set_selection(rect);
                } else {
                    self.document.clear_selection();
                }
                self.dirty_since_primary_save = true;
                self.dirty_since_autosave = true;
                self.last_change_at = Some(std::time::Instant::now());
                Some(CanvasInteraction::Marquee {
                    before,
                    before_inverted,
                    start_canvas_x,
                    start_canvas_y,
                })
            }
            CanvasInteraction::Brush {
                mode,
                last_canvas_x,
                last_canvas_y,
                mut aggregate,
            } => {
                if last_canvas_x != canvas_x || last_canvas_y != canvas_y {
                    if let Some(segment) = self.apply_active_layer_stroke_segment(
                        mode,
                        &[
                            (last_canvas_x as f32, last_canvas_y as f32),
                            (canvas_x as f32, canvas_y as f32),
                        ],
                    ) {
                        if let Some(existing) = &mut aggregate {
                            Self::merge_brush_records(existing, segment);
                        } else {
                            aggregate = Some(segment);
                        }
                        self.dirty_since_primary_save = true;
                        self.dirty_since_autosave = true;
                        self.last_change_at = Some(std::time::Instant::now());
                    }
                }

                Some(CanvasInteraction::Brush {
                    mode,
                    last_canvas_x: canvas_x,
                    last_canvas_y: canvas_y,
                    aggregate,
                })
            }
        };
    }

    fn end_canvas_interaction(&mut self) {
        match self.interaction.take() {
            Some(CanvasInteraction::Move {
                layer_id,
                start_offset_x,
                start_offset_y,
                ..
            }) => {
                if self.active_tool == ShellToolKind::Transform {
                    return;
                }
                let (current_x, current_y) = self
                    .document
                    .layer_offset(self.document.active_layer_index())
                    .unwrap_or((0, 0));
                let delta_x = current_x - start_offset_x;
                let delta_y = current_y - start_offset_y;
                if delta_x != 0 || delta_y != 0 {
                    self.push_operation(
                        format!("Move Layer {} ({}, {})", self.active_layer_name(), delta_x, delta_y),
                        EditorOperation::MoveLayer(MoveLayerRecord {
                            layer_id,
                            before_offset: (start_offset_x, start_offset_y),
                            after_offset: (current_x, current_y),
                        }),
                    );
                }
            }
            Some(CanvasInteraction::Marquee {
                before,
                before_inverted,
                start_canvas_x,
                start_canvas_y,
            }) => {
                if let Some(selection) = self.document.selection() {
                    self.push_operation(
                        "Rectangular Selection",
                        EditorOperation::Selection(RectangularSelectionRecord {
                            before,
                            before_inverted,
                            after: Some(selection),
                            after_inverted: self.document.selection_inverted(),
                        }),
                    );
                } else if before.is_some() {
                    let _ = RectangularMarqueeTool::apply_selection(
                        &mut self.document,
                        start_canvas_x,
                        start_canvas_y,
                        start_canvas_x,
                        start_canvas_y,
                    );
                    self.mark_document_dirty();
                }
            }
            Some(CanvasInteraction::Brush { mode, aggregate, .. }) => {
                if let Some(record) = aggregate {
                    let label = match (record.target, mode) {
                        (LayerEditTarget::LayerPixels, BrushToolMode::Paint) => "Brush Stroke",
                        (LayerEditTarget::LayerPixels, BrushToolMode::Erase) => "Erase Stroke",
                        (LayerEditTarget::LayerMask, BrushToolMode::Paint) => "Mask Hide Stroke",
                        (LayerEditTarget::LayerMask, BrushToolMode::Erase) => "Mask Reveal Stroke",
                    };
                    self.push_operation(label, EditorOperation::BrushStroke(record));
                }
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PhotoTuxController, AUTOSAVE_IDLE_INTERVAL};
    use common::CanvasRect;
    use file_io::{
        flatten_document_rgba, load_document_from_path,
        recovery_path_for_project_path, save_document_to_path,
    };
    use std::fs;
    use std::thread;
    use std::time::{Duration, Instant};
    use ui_shell::{ShellController, ShellToolKind};

    fn set_pixel(document: &mut doc_model::Document, layer_index: usize, x: u32, y: u32, rgba: [u8; 4]) {
        let tile_size = document.tile_size as usize;
        let coord = document
            .tile_coord_for_pixel(x, y)
            .expect("pixel should lie inside representative controller scene");
        let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
        let tile = document
            .ensure_tile_for_pixel(layer_index, x, y)
            .expect("tile should exist for representative controller scene");
        let local_x = (x - tile_origin_x) as usize;
        let local_y = (y - tile_origin_y) as usize;
        let pixel_index = (local_y * tile_size + local_x) * 4;
        tile.pixels[pixel_index..pixel_index + 4].copy_from_slice(&rgba);
    }

    fn set_mask_alpha(document: &mut doc_model::Document, layer_index: usize, x: u32, y: u32, alpha: u8) {
        let tile_size = document.tile_size as usize;
        let tile_size_u32 = document.tile_size;
        let coord = document
            .tile_coord_for_pixel(x, y)
            .expect("mask pixel should lie inside representative controller scene");
        let (tile_origin_x, tile_origin_y) = document.tile_origin(coord);
        let local_x = (x - tile_origin_x) as usize;
        let local_y = (y - tile_origin_y) as usize;
        let pixel_index = local_y * tile_size + local_x;
        let mask = document
            .layer_mask_mut(layer_index)
            .expect("mask should exist for masked controller scene");
        let tile = mask.ensure_tile(coord, tile_size_u32);
        tile.alpha[pixel_index] = alpha;
    }

    fn flattened_pixel(raster: &common::CanvasRaster, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * raster.size.width + x) * 4) as usize;
        [
            raster.pixels[index],
            raster.pixels[index + 1],
            raster.pixels[index + 2],
            raster.pixels[index + 3],
        ]
    }

    fn build_representative_controller_document() -> doc_model::Document {
        let mut document = doc_model::Document::new(64, 64);
        document.rename_layer(0, "Background");
        for y in 0..20 {
            for x in 0..20 {
                set_pixel(&mut document, 0, x, y, [60, 90, 140, 255]);
            }
        }

        document.add_layer("Overlay");
        let overlay_index = document.active_layer_index();
        document.set_layer_blend_mode(overlay_index, doc_model::BlendMode::Overlay);
        for y in 8..28 {
            for x in 8..28 {
                set_pixel(&mut document, overlay_index, x, y, [220, 120, 60, 220]);
            }
        }

        document.add_layer("Lighten");
        let lighten_index = document.active_layer_index();
        document.set_layer_blend_mode(lighten_index, doc_model::BlendMode::Lighten);
        document.set_layer_opacity(lighten_index, 70);
        assert!(document.set_layer_offset(lighten_index, 6, 10));
        for y in 4..18 {
            for x in 24..40 {
                set_pixel(&mut document, lighten_index, x, y, [80, 220, 200, 255]);
            }
        }

        document
    }

    fn build_masked_controller_document() -> doc_model::Document {
        let mut document = build_representative_controller_document();
        let masked_index = document.active_layer_index();
        assert!(document.add_layer_mask(masked_index));

        for y in 4..18 {
            for x in 24..40 {
                let alpha = if x < 30 {
                    0
                } else if x < 35 {
                    128
                } else {
                    255
                };
                set_mask_alpha(&mut document, masked_index, x, y, alpha);
            }
        }

        document
    }

    fn wait_for_background_jobs(controller: &mut PhotoTuxController) {
        for _ in 0..50 {
            controller.poll_background_tasks_at(Instant::now());
            if controller.pending_primary_save_job.is_none()
                && controller.pending_autosave_job.is_none()
                && controller.pending_recovery_load_job.is_none()
                && controller.pending_document_load_job.is_none()
                && controller.pending_export_job.is_none()
            {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }

        panic!("background jobs did not complete in time");
    }

    #[test]
    fn layer_actions_update_snapshot() {
        let mut controller = PhotoTuxController::new();
        let initial_count = controller.snapshot().layers.len();

        controller.add_layer();
        controller.duplicate_active_layer();

        let snapshot = controller.snapshot();
        assert!(snapshot.layers.len() >= initial_count + 2);
        assert!(snapshot.history_entries.iter().any(|entry| entry.contains("Add Layer")));
        assert!(snapshot
            .history_entries
            .iter()
            .any(|entry| entry.contains("Duplicate Layer")));
    }

    #[test]
    fn color_actions_update_snapshot() {
        let mut controller = PhotoTuxController::new();
        let before = controller.snapshot();

        controller.swap_colors();
        let swapped = controller.snapshot();
        assert_eq!(swapped.foreground_color, before.background_color);
        assert_eq!(swapped.background_color, before.foreground_color);

        controller.reset_colors();
        let reset = controller.snapshot();
        assert_eq!(reset.foreground_color, [232, 236, 243, 255]);
        assert_eq!(reset.background_color, [27, 29, 33, 255]);
    }

    #[test]
    fn blend_mode_actions_update_snapshot_and_canvas() {
        let mut controller = PhotoTuxController::new();
        let tile_size = controller.document.tile_size as usize;
        let base_tile = controller
            .document
            .ensure_tile_for_pixel(1, 180, 140)
            .expect("base tile should exist");
        let base_index = (140 % tile_size * tile_size + (180 % tile_size)) * 4;
        base_tile.pixels[base_index..base_index + 4].copy_from_slice(&[128, 128, 128, 255]);

        let top_tile = controller
            .document
            .ensure_tile_for_pixel(2, 180, 140)
            .expect("top tile should exist");
        top_tile.pixels[base_index..base_index + 4].copy_from_slice(&[128, 64, 255, 255]);

        let before_mode = controller.snapshot().active_layer_blend_mode;
        let before_canvas = controller.canvas_raster();

        controller.next_active_layer_blend_mode();

        let after_snapshot = controller.snapshot();
        let after_canvas = controller.canvas_raster();
        assert_ne!(after_snapshot.active_layer_blend_mode, before_mode);
        assert_ne!(after_canvas.pixels, before_canvas.pixels);
    }

    #[test]
    fn save_document_in_directory_writes_a_project_file() {
        let mut controller = PhotoTuxController::new();
        let output_dir = std::env::temp_dir().join(format!("phototux-save-{}", std::process::id()));
        fs::create_dir_all(&output_dir).expect("temporary output directory should be created");

        let saved_path = controller
            .save_document_in_directory(&output_dir)
            .expect("document should save into temp directory");
        let restored = load_document_from_path(&saved_path).expect("saved project should load");

        assert_eq!(restored.canvas_size, controller.document.canvas_size);
        assert_eq!(restored.layers.len(), controller.document.layers.len());

        fs::remove_file(&saved_path).expect("saved project file should be removed");
        fs::remove_dir(&output_dir).expect("temporary output directory should be removed");
    }

    #[test]
    fn save_document_without_existing_path_requires_save_as() {
        let mut controller = PhotoTuxController::new();

        controller.save_document();

        assert!(controller.document_path.is_none());
        assert!(controller.status_message.contains("Save As required"));
        assert!(controller.pending_primary_save_job.is_none());
    }

    #[test]
    fn save_document_as_persists_to_selected_project_path() {
        let working_directory = std::env::temp_dir().join(format!("phototux-save-as-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary save-as directory should exist");

        let mut controller = PhotoTuxController::new_with_working_directory(working_directory.clone());
        controller.add_layer();
        let project_path = working_directory.join("custom-name.ptx");

        controller.save_document_as(project_path.clone());
        wait_for_background_jobs(&mut controller);

        assert_eq!(controller.document_path.as_ref(), Some(&project_path));
        assert_eq!(controller.document_title, "custom-name.ptx");
        assert!(!controller.dirty_since_primary_save);
        assert!(!controller.dirty_since_autosave);
        assert!(controller.status_message.contains("Saved"));
        assert!(project_path.exists());

        fs::remove_file(&project_path).expect("saved project file should be removed");
        fs::remove_dir(&working_directory).expect("temporary save-as directory should be removed");
    }

    #[test]
    fn canvas_raster_matches_flattened_document_for_representative_scene() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_representative_controller_document();

        let viewport_pixels = controller.canvas_raster().pixels;
        let exported_pixels = flatten_document_rgba(&controller.document);

        assert_eq!(viewport_pixels, exported_pixels);
    }

    #[test]
    fn canvas_raster_matches_flattened_document_for_masked_scene() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_masked_controller_document();

        let viewport_pixels = controller.canvas_raster().pixels;
        let exported_pixels = flatten_document_rgba(&controller.document);

        assert_eq!(viewport_pixels, exported_pixels);
    }

    #[test]
    fn save_does_not_block_undo_of_previous_edit() {
        let working_directory = std::env::temp_dir().join(format!("phototux-save-undo-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary save+undo directory should exist");

        let mut controller = PhotoTuxController::new_with_working_directory(working_directory.clone());
        let before = controller.canvas_raster();
        controller.select_tool(ShellToolKind::Brush);
        controller.begin_canvas_interaction(120, 120);
        controller.update_canvas_interaction(144, 132);
        controller.end_canvas_interaction();
        let painted = controller.canvas_raster();
        assert_ne!(before.pixels, painted.pixels);

        let saved_path = working_directory.join("untitled.ptx");
        controller.save_document_as(saved_path.clone());
        wait_for_background_jobs(&mut controller);
        controller.undo();

        assert_eq!(controller.canvas_raster().pixels, before.pixels);

        if saved_path.exists() {
            fs::remove_file(&saved_path).expect("saved project file should be removed");
        }
        fs::remove_dir(&working_directory).expect("temporary save+undo directory should be removed");
    }

    #[test]
    fn autosave_writes_recovery_file_after_idle_period() {
        let working_directory = std::env::temp_dir().join(format!("phototux-autosave-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary autosave directory should exist");

        let mut controller = PhotoTuxController::new_with_working_directory(working_directory.clone());
        controller.add_layer();
        controller.last_change_at = Some(Instant::now() - AUTOSAVE_IDLE_INTERVAL - Duration::from_millis(1));

        controller.poll_background_tasks_at(Instant::now());
        wait_for_background_jobs(&mut controller);

        let recovery_path = recovery_path_for_project_path(&working_directory.join("untitled.ptx"));
        let recovered = load_document_from_path(&recovery_path).expect("autosave recovery file should load");
        assert_eq!(recovered.layers.len(), controller.document.layers.len());
        assert!(!controller.dirty_since_autosave);

        fs::remove_file(&recovery_path).expect("autosave recovery file should be removed");
        fs::remove_dir(&working_directory).expect("temporary autosave directory should be removed");
    }

    #[test]
    fn startup_recovery_offer_requires_explicit_load() {
        let working_directory = std::env::temp_dir().join(format!("phototux-recovery-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary recovery directory should exist");

        let mut recovered_document = doc_model::Document::new(320, 240);
        recovered_document.add_layer("Recovered Layer");
        let recovery_path = recovery_path_for_project_path(&working_directory.join("untitled.ptx"));
        save_document_to_path(&recovery_path, &recovered_document).expect("recovery document should save");

        let mut controller = PhotoTuxController::new_with_working_directory(working_directory.clone());
        wait_for_background_jobs(&mut controller);

        assert!(controller.recovery_offer_pending);
        assert_eq!(controller.document.canvas_size.width, 1920);
        assert_eq!(controller.document.canvas_size.height, 1080);

        controller.load_recovery_document();
        wait_for_background_jobs(&mut controller);

        assert_eq!(controller.document.canvas_size.width, 320);
        assert_eq!(controller.document.canvas_size.height, 240);
        assert_eq!(controller.document.layers.len(), 2);
        assert!(!controller.recovery_offer_pending);
        assert!(controller.snapshot().history_entries.iter().any(|entry| entry.contains("Recovered Autosave")));

        fs::remove_file(&recovery_path).expect("recovery file should be removed");
        fs::remove_dir(&working_directory).expect("temporary recovery directory should be removed");
    }

    #[test]
    fn discard_recovery_offer_removes_recovery_file() {
        let working_directory = std::env::temp_dir().join(format!("phototux-recovery-discard-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary recovery directory should exist");

        let mut recovered_document = doc_model::Document::new(320, 240);
        recovered_document.add_layer("Recovered Layer");
        let recovery_path = recovery_path_for_project_path(&working_directory.join("untitled.ptx"));
        save_document_to_path(&recovery_path, &recovered_document).expect("recovery document should save");

        let mut controller = PhotoTuxController::new_with_working_directory(working_directory.clone());
        wait_for_background_jobs(&mut controller);

        assert!(controller.recovery_offer_pending);
        assert!(recovery_path.exists());

        controller.discard_recovery_document();

        assert!(!controller.recovery_offer_pending);
        assert!(!recovery_path.exists());

        fs::remove_dir(&working_directory).expect("temporary recovery directory should be removed");
    }

    #[test]
    fn open_document_loads_saved_project_into_controller_state() {
        let working_directory = std::env::temp_dir().join(format!("phototux-open-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary open directory should exist");

        let document = build_representative_controller_document();
        let project_path = working_directory.join("scene.ptx");
        save_document_to_path(&project_path, &document).expect("project should save for reopen test");

        let mut controller = PhotoTuxController::new();
        controller.add_layer();
        controller.open_document(project_path.clone());
        wait_for_background_jobs(&mut controller);

        assert_eq!(controller.document.canvas_size, document.canvas_size);
        assert_eq!(controller.document.layers.len(), document.layers.len());
        assert_eq!(controller.document_path.as_ref(), Some(&project_path));
        assert!(!controller.dirty_since_primary_save);
        assert!(!controller.dirty_since_autosave);
        assert!(controller.status_message.contains("Opened"));
        assert_eq!(controller.snapshot().history_entries, vec!["Open Document".to_string()]);

        fs::remove_file(&project_path).expect("project file should be removed");
        fs::remove_dir(&working_directory).expect("temporary open directory should be removed");
    }

    #[test]
    fn export_and_import_commands_roundtrip_through_background_jobs() {
        let working_directory = std::env::temp_dir().join(format!("phototux-io-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary io directory should exist");

        let mut export_controller = PhotoTuxController::new_with_working_directory(working_directory.clone());
        export_controller.document = build_representative_controller_document();
        let export_path = working_directory.join("scene.png");

        export_controller.export_document(export_path.clone());
        wait_for_background_jobs(&mut export_controller);

        assert!(export_path.exists());
        assert!(export_controller.status_message.contains("Exported"));

        let mut import_controller = PhotoTuxController::new();
        import_controller.import_image(export_path.clone());
        wait_for_background_jobs(&mut import_controller);

        assert_eq!(import_controller.document.canvas_size, export_controller.document.canvas_size);
        assert_eq!(import_controller.document.layer_count(), 1);
        assert!(import_controller.document_path.is_none());
        assert!(import_controller.dirty_since_primary_save);
        assert!(import_controller.dirty_since_autosave);
        assert!(import_controller.status_message.contains("Imported"));
        assert_eq!(import_controller.snapshot().history_entries, vec!["Import Image".to_string()]);

        fs::remove_file(&export_path).expect("exported png should be removed");
        fs::remove_dir(&working_directory).expect("temporary io directory should be removed");
    }

    #[test]
    fn move_interaction_updates_active_layer_bounds() {
        let mut controller = PhotoTuxController::new();
        let before = controller
            .snapshot()
            .active_layer_bounds
            .map(|rect| (rect.x, rect.y))
            .expect("active layer should have bounds");
        controller.select_tool(ShellToolKind::Move);
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(30, 15);
        controller.end_canvas_interaction();

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.active_layer_bounds.map(|rect| (rect.x, rect.y)),
            Some((before.0 + 30, before.1 + 15))
        );
        assert!(snapshot.history_entries.iter().any(|entry| entry.contains("Move Layer")));
    }

    #[test]
    fn marquee_interaction_sets_selection_rect() {
        let mut controller = PhotoTuxController::new();
        controller.select_tool(ShellToolKind::RectangularMarquee);
        controller.begin_canvas_interaction(10, 20);
        controller.update_canvas_interaction(50, 70);
        controller.end_canvas_interaction();

        let snapshot = controller.snapshot();
        assert_eq!(snapshot.selection_rect, Some(CanvasRect::new(10, 20, 40, 50)));
    }

    #[test]
    fn brush_interaction_updates_canvas_and_history() {
        let mut controller = PhotoTuxController::new();
        let before = controller.canvas_raster();

        controller.select_tool(ShellToolKind::Brush);
        controller.begin_canvas_interaction(120, 120);
        controller.update_canvas_interaction(144, 132);
        controller.end_canvas_interaction();

        let after = controller.canvas_raster();
        assert_ne!(before.pixels, after.pixels);
        assert!(controller
            .snapshot()
            .history_entries
            .iter()
            .any(|entry| entry.contains("Brush Stroke")));
    }

    #[test]
    fn mask_commands_update_snapshot_and_history() {
        let mut controller = PhotoTuxController::new();
        assert!(!controller.snapshot().active_layer_has_mask);

        controller.add_active_layer_mask();
        let with_mask = controller.snapshot();
        assert!(with_mask.active_layer_has_mask);
        assert!(with_mask.active_layer_mask_enabled);
        assert_eq!(with_mask.active_edit_target_name, "Layer Mask");
        assert!(with_mask
            .history_entries
            .iter()
            .any(|entry| entry.contains("Add Layer Mask")));

        controller.toggle_active_layer_mask_enabled();
        assert!(!controller.snapshot().active_layer_mask_enabled);

        controller.edit_active_layer_pixels();
        assert_eq!(controller.snapshot().active_edit_target_name, "Layer Pixels");

        controller.remove_active_layer_mask();
        assert!(!controller.snapshot().active_layer_has_mask);
    }

    #[test]
    fn mask_brush_interaction_updates_canvas_and_undo_redo() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_masked_controller_document();
        controller.edit_active_layer_mask();
        let sample_x = 44;
        let sample_y = 16;
        let before = controller.canvas_raster();
        let before_pixel = flattened_pixel(&before, sample_x, sample_y);

        controller.select_tool(ShellToolKind::Brush);
        controller.begin_canvas_interaction(sample_x as i32, sample_y as i32);
        controller.update_canvas_interaction(sample_x as i32 + 4, sample_y as i32 + 2);
        controller.end_canvas_interaction();

        let hidden = controller.canvas_raster();
        let hidden_pixel = flattened_pixel(&hidden, sample_x, sample_y);
        assert_ne!(hidden_pixel, before_pixel);
        assert!(controller
            .snapshot()
            .history_entries
            .iter()
            .any(|entry| entry.contains("Mask Hide Stroke")));

        controller.undo();
        assert_eq!(flattened_pixel(&controller.canvas_raster(), sample_x, sample_y), before_pixel);

        controller.redo();
        assert_eq!(flattened_pixel(&controller.canvas_raster(), sample_x, sample_y), hidden_pixel);
    }

    #[test]
    fn undo_redo_restores_brush_move_and_selection_state() {
        let mut controller = PhotoTuxController::new();
        let original_canvas = controller.canvas_raster();

        controller.select_tool(ShellToolKind::Brush);
        controller.begin_canvas_interaction(120, 120);
        controller.update_canvas_interaction(144, 132);
        controller.end_canvas_interaction();
        let painted_canvas = controller.canvas_raster();
        let painted_bounds = controller.snapshot().active_layer_bounds;
        assert_ne!(painted_canvas.pixels, original_canvas.pixels);

        controller.undo();
        assert_eq!(controller.canvas_raster().pixels, original_canvas.pixels);
        controller.redo();
        assert_eq!(controller.canvas_raster().pixels, painted_canvas.pixels);

        controller.select_tool(ShellToolKind::Move);
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(25, 10);
        controller.end_canvas_interaction();
        let moved_bounds = controller.snapshot().active_layer_bounds;
        assert_ne!(moved_bounds, painted_bounds);

        controller.undo();
        assert_eq!(controller.snapshot().active_layer_bounds, painted_bounds);
        controller.redo();
        assert_eq!(controller.snapshot().active_layer_bounds, moved_bounds);

        controller.select_tool(ShellToolKind::RectangularMarquee);
        controller.begin_canvas_interaction(10, 10);
        controller.update_canvas_interaction(30, 40);
        controller.end_canvas_interaction();
        assert_eq!(controller.snapshot().selection_rect, Some(CanvasRect::new(10, 10, 20, 30)));

        controller.undo();
        assert_eq!(controller.snapshot().selection_rect, None);
        controller.redo();
        assert_eq!(controller.snapshot().selection_rect, Some(CanvasRect::new(10, 10, 20, 30)));
    }

    #[test]
    fn transform_preview_and_commit_update_canvas_and_history() {
        let mut controller = PhotoTuxController::new();
        let before = controller.canvas_raster();

        controller.select_tool(ShellToolKind::Transform);
        controller.begin_transform();
        controller.scale_transform_up();
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(20, 10);
        controller.end_canvas_interaction();

        let preview = controller.canvas_raster();
        assert_ne!(before.pixels, preview.pixels);
        assert!(controller.snapshot().transform_active);
        assert!(controller.snapshot().transform_preview_rect.is_some());

        controller.commit_transform();
        assert!(!controller.snapshot().transform_active);
        assert!(controller
            .snapshot()
            .history_entries
            .iter()
            .any(|entry| entry.contains("Transform Layer")));
    }
}