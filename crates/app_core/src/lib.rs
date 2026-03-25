use anyhow::Context;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use common::{CanvasRaster, CanvasRect, DestructiveFilterKind};
use doc_model::{
    BlendMode, Document, Guide, GuideOrientation, LayerEditTarget, LayerHierarchyNode,
    LayerHierarchyNodeRef, LayerStateSnapshot, RasterMask, SelectionShape,
};
use file_io::{
    PROJECT_FILE_EXTENSION, PsdImportDiagnostic, PsdImportDiagnosticSeverity, PsdImportResult,
    PsdImportSidecar, export_jpeg_to_path, export_png_to_path, export_webp_to_path,
    flatten_document_rgba, import_jpeg_from_path, import_png_from_path,
    import_psd_from_path_with_sidecar, import_webp_from_path, load_document_from_path,
    recovery_path_for_project_path, remove_file_if_exists, save_document_to_path,
};
use history_engine::HistoryStack;
use image_ops::apply_destructive_filter_rgba;
use tool_system::{
    BrushChange, BrushSample, BrushSettings, BrushStrokeRecord, BrushTool, BrushToolMode,
    LassoTool, LayerTransformRecord, MoveLayerRecord, RectangularMarqueeTool, SelectionRecord,
    SimpleTransformTool,
};
use ui_shell::{
    LayerPanelItem, ShellController, ShellGuide, ShellImportDiagnostic, ShellImportReport,
    ShellSnapshot, ShellToolKind,
};

pub fn build_shell_controller() -> Rc<RefCell<dyn ShellController>> {
    Rc::new(RefCell::new(PhotoTuxController::new()))
}

const AUTOSAVE_IDLE_INTERVAL: Duration = Duration::from_secs(2);
const GUIDE_SNAP_THRESHOLD: i32 = 8;
const PSD_IMPORT_SIDECAR_PATH_ENV: &str = "PHOTOTUX_PSD_IMPORT_SIDECAR";
const PSD_IMPORT_SIDECAR_ARGS_ENV: &str = "PHOTOTUX_PSD_IMPORT_SIDECAR_ARGS";

#[derive(Debug)]
struct PhotoTuxController {
    document: Document,
    selected_structure_target: LayerHierarchyNodeRef,
    history: HistoryStack<EditorHistoryEntry>,
    foreground_color: [u8; 4],
    background_color: [u8; 4],
    status_message: String,
    document_title: String,
    document_path: Option<PathBuf>,
    recovery_path: Option<PathBuf>,
    recovery_offer_pending: bool,
    working_directory: PathBuf,
    psd_import_sidecar: Option<PsdImportSidecar>,
    latest_import_report: Option<ShellImportReport>,
    next_import_report_id: u64,
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
    pending_filter_job: Option<PendingFilterJob>,
    jobs: JobSystem,
    cached_canvas_raster: Option<Vec<u8>>,
    transform_session: Option<TransformSession>,
    interaction: Option<CanvasInteraction>,
    snapping_enabled: bool,
    temporary_snap_bypass: bool,
    pressure_size_enabled: bool,
    pressure_opacity_enabled: bool,
    active_brush_preset: Option<BrushPreset>,
    brush_radius: f32,
    brush_hardness: f32,
    brush_spacing: f32,
    brush_flow: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrushPreset {
    BalancedRound,
    SoftShade,
    InkTaper,
}

impl BrushPreset {
    const ALL: [Self; 3] = [Self::BalancedRound, Self::SoftShade, Self::InkTaper];

    const fn label(self) -> &'static str {
        match self {
            Self::BalancedRound => "Balanced Round",
            Self::SoftShade => "Soft Shade",
            Self::InkTaper => "Ink Taper",
        }
    }

    const fn settings(self) -> (f32, f32, f32, f32, bool, bool) {
        match self {
            Self::BalancedRound => (12.0, 0.72, 5.0, 0.82, false, false),
            Self::SoftShade => (24.0, 0.35, 12.0, 0.4, true, true),
            Self::InkTaper => (7.0, 1.0, 3.0, 0.28, true, false),
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|preset| *preset == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|preset| *preset == self)
            .unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone)]
struct TransformSession {
    layer_id: common::LayerId,
    translate_x: i32,
    translate_y: i32,
    scale_x: f32,
    scale_y: f32,
    rotate_quadrants: i32,
}

#[derive(Debug, Clone)]
enum CanvasInteraction {
    Move {
        layer_id: common::LayerId,
        start_canvas_x: i32,
        start_canvas_y: i32,
        start_offset_x: i32,
        start_offset_y: i32,
        initial_state: Option<LayerStateSnapshot>,
        snapping_base_bounds: Option<CanvasRect>,
    },
    Marquee {
        before: Option<SelectionShape>,
        before_inverted: bool,
        start_canvas_x: i32,
        start_canvas_y: i32,
    },
    Lasso {
        before: Option<SelectionShape>,
        before_inverted: bool,
        points: Vec<(i32, i32)>,
    },
    Brush {
        mode: BrushToolMode,
        last_canvas_x: i32,
        last_canvas_y: i32,
        last_pressure: f32,
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
    Selection(SelectionRecord),
    Guides(GuideStateRecord),
    MaskState(MaskStateRecord),
    LayerHierarchy(LayerHierarchyRecord),
    DestructiveFilter(DestructiveFilterRecord),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuideStateRecord {
    before_guides: Vec<Guide>,
    before_visible: bool,
    after_guides: Vec<Guide>,
    after_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaskStateRecord {
    layer_id: common::LayerId,
    before_mask: Option<RasterMask>,
    after_mask: Option<RasterMask>,
    before_target: LayerEditTarget,
    after_target: LayerEditTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayerHierarchyRecord {
    before_hierarchy: Vec<LayerHierarchyNode>,
    after_hierarchy: Vec<LayerHierarchyNode>,
    before_active_layer_id: common::LayerId,
    after_active_layer_id: common::LayerId,
    before_selected_target: LayerHierarchyNodeRef,
    after_selected_target: LayerHierarchyNodeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayerHierarchySnapshot {
    hierarchy: Vec<LayerHierarchyNode>,
    active_layer_id: common::LayerId,
    selected_target: LayerHierarchyNodeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DestructiveFilterRecord {
    layer_id: common::LayerId,
    filter: DestructiveFilterKind,
    before: LayerStateSnapshot,
    after: LayerStateSnapshot,
}

#[derive(Debug)]
struct DocumentLoadState {
    document: Document,
    document_title: String,
    document_path: Option<PathBuf>,
    working_directory: PathBuf,
    dirty_since_primary_save: bool,
    dirty_since_autosave: bool,
    last_change_at: Option<Instant>,
    history_label: String,
    status_message: String,
}

impl DestructiveFilterRecord {
    fn undo(&self, document: &mut Document) {
        let _ = document.apply_layer_state_snapshot(self.layer_id, self.before.clone());
    }

    fn redo(&self, document: &mut Document) {
        let _ = document.apply_layer_state_snapshot(self.layer_id, self.after.clone());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingFilterJob {
    job_id: u64,
    requested_canvas_revision: u64,
}

#[derive(Debug, Clone)]
struct PsdImportJobReport {
    diagnostics: Vec<PsdImportDiagnostic>,
    used_flattened_fallback: bool,
}

impl LayerHierarchyRecord {
    fn undo(&self, controller: &mut PhotoTuxController) {
        self.apply(
            controller,
            self.before_hierarchy.clone(),
            self.before_active_layer_id,
            self.before_selected_target,
        );
    }

    fn redo(&self, controller: &mut PhotoTuxController) {
        self.apply(
            controller,
            self.after_hierarchy.clone(),
            self.after_active_layer_id,
            self.after_selected_target,
        );
    }

    fn apply(
        &self,
        controller: &mut PhotoTuxController,
        hierarchy: Vec<LayerHierarchyNode>,
        active_layer_id: common::LayerId,
        selected_target: LayerHierarchyNodeRef,
    ) {
        let _ = controller.document.set_layer_hierarchy(hierarchy);
        if let Some(layer_index) = controller.document.layer_index_by_id(active_layer_id) {
            let _ = controller.document.set_active_layer(layer_index);
        }
        controller.selected_structure_target = selected_target;
    }
}

impl GuideStateRecord {
    fn undo(&self, document: &mut Document) {
        document.set_guides_state(self.before_guides.clone(), self.before_visible);
    }

    fn redo(&self, document: &mut Document) {
        document.set_guides_state(self.after_guides.clone(), self.after_visible);
    }
}

impl MaskStateRecord {
    fn undo(&self, document: &mut Document) {
        Self::apply_state(
            document,
            self.layer_id,
            self.before_mask.clone(),
            self.before_target,
        );
    }

    fn redo(&self, document: &mut Document) {
        Self::apply_state(
            document,
            self.layer_id,
            self.after_mask.clone(),
            self.after_target,
        );
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
    PsdImport,
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
        psd_import_sidecar: Option<PsdImportSidecar>,
    },
    ExportDocument {
        job_id: u64,
        path: PathBuf,
        document: Document,
        format: RasterFileFormat,
    },
    ApplyDestructiveFilter {
        job_id: u64,
        layer_id: common::LayerId,
        filter: DestructiveFilterKind,
        before: LayerStateSnapshot,
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
        import_notice: Option<String>,
        psd_import_report: Option<PsdImportJobReport>,
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
    DestructiveFilterApplied {
        job_id: u64,
        layer_id: common::LayerId,
        filter: DestructiveFilterKind,
        before: LayerStateSnapshot,
        after: LayerStateSnapshot,
    },
    DestructiveFilterFailed {
        job_id: u64,
        filter: DestructiveFilterKind,
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

fn psd_file_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("psd"))
        .unwrap_or(false)
}

fn default_psd_import_sidecar() -> Option<PsdImportSidecar> {
    let executable_path = std::env::var_os(PSD_IMPORT_SIDECAR_PATH_ENV)?;
    if executable_path.is_empty() {
        return None;
    }

    let mut sidecar = PsdImportSidecar::new(PathBuf::from(executable_path));
    if let Some(base_args) = std::env::var_os(PSD_IMPORT_SIDECAR_ARGS_ENV) {
        let parsed_args = base_args
            .to_string_lossy()
            .split_whitespace()
            .filter(|argument| !argument.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !parsed_args.is_empty() {
            sidecar = sidecar.with_args(parsed_args);
        }
    }

    Some(sidecar)
}

fn apply_destructive_filter_to_layer_snapshot(
    before: LayerStateSnapshot,
    filter: DestructiveFilterKind,
) -> anyhow::Result<LayerStateSnapshot> {
    let mut after = before.clone();
    let mut changed = false;

    for tile in after.tiles.values_mut() {
        changed |= apply_destructive_filter_rgba(&mut tile.pixels, filter);
    }

    if !changed {
        anyhow::bail!("filter had no visible effect on the active layer")
    }

    Ok(after)
}

fn build_psd_import_job_report(imported: &PsdImportResult) -> Option<PsdImportJobReport> {
    let diagnostics = imported
        .diagnostics
        .iter()
        .filter(|diagnostic| !matches!(diagnostic.severity, PsdImportDiagnosticSeverity::Info))
        .cloned()
        .collect::<Vec<_>>();
    if !imported.used_flattened_fallback && diagnostics.is_empty() {
        return None;
    }

    Some(PsdImportJobReport {
        diagnostics,
        used_flattened_fallback: imported.used_flattened_fallback,
    })
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

    fn enqueue(
        &mut self,
        priority: JobPriority,
        make_request: impl FnOnce(u64) -> JobRequest,
    ) -> u64 {
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

fn worker_main(queues: Arc<(Mutex<JobQueues>, Condvar)>, result_sender: mpsc::Sender<JobResult>) {
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
                    if let Some(recovery_path) = cleanup_recovery_path
                        && let Err(error) = remove_file_if_exists(&recovery_path)
                    {
                        tracing::warn!(%error, path = %recovery_path.display(), "failed to remove stale recovery file after save");
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
            } => match load_document_from_path(&recovery_path).with_context(|| {
                format!(
                    "failed to load recovery document from {}",
                    recovery_path.display()
                )
            }) {
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
            JobRequest::LoadDocument {
                job_id,
                path,
                kind,
                psd_import_sidecar,
            } => {
                let result = match kind {
                    DocumentLoadKind::Project => load_document_from_path(&path)
                        .with_context(|| format!("failed to open project from {}", path.display()))
                        .map(|document| (document, None, None)),
                    DocumentLoadKind::RasterImport => match raster_format_from_path(&path) {
                        Some(RasterFileFormat::Png) => import_png_from_path(&path),
                        Some(RasterFileFormat::Jpeg) => import_jpeg_from_path(&path),
                        Some(RasterFileFormat::Webp) => import_webp_from_path(&path),
                        None => Err(anyhow::anyhow!(
                            "unsupported import format for {}",
                            path.display()
                        )),
                    }
                    .map(|document| (document, None, None)),
                    DocumentLoadKind::PsdImport => psd_import_sidecar.map_or_else(
                        || {
                            Err(anyhow::anyhow!(
                                "PSD import sidecar is not configured; set {} to an executable path",
                                PSD_IMPORT_SIDECAR_PATH_ENV
                            ))
                        },
                        |sidecar| {
                            import_psd_from_path_with_sidecar(&path, &sidecar)
                                .with_context(|| format!("failed to import PSD from {}", path.display()))
                                .map(|imported| {
                                    let psd_import_report = build_psd_import_job_report(&imported);
                                    let import_notice = imported
                                        .used_flattened_fallback
                                        .then(|| "flattened PSD fallback used".to_string());
                                    (imported.document, import_notice, psd_import_report)
                                })
                        },
                    ),
                };

                match result {
                    Ok((document, import_notice, psd_import_report)) => JobResult::DocumentLoaded {
                        job_id,
                        path,
                        kind,
                        document,
                        import_notice,
                        psd_import_report,
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
                    Ok(()) => JobResult::ExportCompleted { job_id, path },
                    Err(error) => JobResult::ExportFailed {
                        job_id,
                        path,
                        format,
                        error: error.to_string(),
                    },
                }
            }
            JobRequest::ApplyDestructiveFilter {
                job_id,
                layer_id,
                filter,
                before,
            } => match apply_destructive_filter_to_layer_snapshot(before.clone(), filter) {
                Ok(after) => JobResult::DestructiveFilterApplied {
                    job_id,
                    layer_id,
                    filter,
                    before,
                    after,
                },
                Err(error) => JobResult::DestructiveFilterFailed {
                    job_id,
                    filter,
                    error: error.to_string(),
                },
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
        Self::new_with_working_directory_and_psd_sidecar(
            working_directory,
            default_psd_import_sidecar(),
        )
    }

    fn new_with_working_directory_and_psd_sidecar(
        working_directory: PathBuf,
        psd_import_sidecar: Option<PsdImportSidecar>,
    ) -> Self {
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
            selected_structure_target: LayerHierarchyNodeRef::Layer(common::LayerId::new()),
            history,
            foreground_color: [232, 236, 243, 255],
            background_color: [27, 29, 33, 255],
            status_message: "Ready".to_string(),
            document_title: "untitled.ptx".to_string(),
            document_path: None,
            recovery_path: None,
            recovery_offer_pending: false,
            working_directory,
            psd_import_sidecar,
            latest_import_report: None,
            next_import_report_id: 1,
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
            pending_filter_job: None,
            jobs: JobSystem::new(),
            cached_canvas_raster: None,
            transform_session: None,
            interaction: None,
            snapping_enabled: true,
            temporary_snap_bypass: false,
            pressure_size_enabled: false,
            pressure_opacity_enabled: false,
            active_brush_preset: Some(BrushPreset::BalancedRound),
            brush_radius: 12.0,
            brush_hardness: 0.72,
            brush_spacing: 5.0,
            brush_flow: 0.82,
        };
        controller.reset_selected_structure_target_to_active_layer();
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

    fn can_apply_destructive_filters(&self) -> bool {
        self.pending_filter_job.is_none()
            && self.transform_session.is_none()
            && self.document.active_edit_target() == LayerEditTarget::LayerPixels
            && self
                .document
                .layer(self.document.active_layer_index())
                .is_some()
    }

    fn selection_path_points(&self) -> Option<Vec<(i32, i32)>> {
        match self.document.selection_shape() {
            Some(SelectionShape::Freeform(selection)) => Some(
                selection
                    .points
                    .iter()
                    .map(|point| (point.x, point.y))
                    .collect(),
            ),
            _ => None,
        }
    }

    fn selection_preview_path_points(&self) -> Option<Vec<(i32, i32)>> {
        match &self.interaction {
            Some(CanvasInteraction::Lasso { points, .. }) if points.len() >= 2 => {
                Some(points.clone())
            }
            _ => None,
        }
    }

    fn shell_guides(&self) -> Vec<ShellGuide> {
        self.document
            .guides()
            .iter()
            .map(|guide| match guide.orientation {
                GuideOrientation::Horizontal => ShellGuide::Horizontal { y: guide.position },
                GuideOrientation::Vertical => ShellGuide::Vertical { x: guide.position },
            })
            .collect()
    }

    fn snapping_temporarily_bypassed(&self) -> bool {
        self.temporary_snap_bypass
            && matches!(self.interaction, Some(CanvasInteraction::Move { .. }))
    }

    fn move_snapping_base_bounds(&self) -> Option<CanvasRect> {
        if let Some(selection_shape) = self.document.selection_shape() {
            let selection_bounds = selection_shape.bounds();
            let layer_bounds = self.active_layer_bounds()?;
            let left = layer_bounds.x.max(selection_bounds.x);
            let top = layer_bounds.y.max(selection_bounds.y);
            let right = (layer_bounds.x + layer_bounds.width as i32)
                .min(selection_bounds.x + selection_bounds.width as i32);
            let bottom = (layer_bounds.y + layer_bounds.height as i32)
                .min(selection_bounds.y + selection_bounds.height as i32);
            if left < right && top < bottom {
                return Some(CanvasRect::new(
                    left,
                    top,
                    (right - left) as u32,
                    (bottom - top) as u32,
                ));
            }
        }

        self.active_layer_bounds()
    }

    fn transform_snapping_base_bounds(&self) -> Option<CanvasRect> {
        let session = self.transform_session.as_ref()?;
        let layer_index = self.document.layer_index_by_id(session.layer_id)?;
        SimpleTransformTool::preview_bounds(
            &self.document,
            layer_index,
            session.scale_x,
            session.scale_y,
            session.rotate_quadrants,
            0,
            0,
        )
    }

    fn snapped_translation(
        &self,
        base_bounds: Option<CanvasRect>,
        translate_x: i32,
        translate_y: i32,
    ) -> (i32, i32) {
        if !self.snapping_enabled || self.temporary_snap_bypass {
            return (translate_x, translate_y);
        }
        let Some(base_bounds) = base_bounds else {
            return (translate_x, translate_y);
        };

        let moved = CanvasRect::new(
            base_bounds.x + translate_x,
            base_bounds.y + translate_y,
            base_bounds.width,
            base_bounds.height,
        );

        let mut snapped_x = translate_x;
        let mut snapped_y = translate_y;
        let mut best_x_distance = GUIDE_SNAP_THRESHOLD + 1;
        let mut best_y_distance = GUIDE_SNAP_THRESHOLD + 1;
        let right = moved.x + moved.width as i32;
        let bottom = moved.y + moved.height as i32;

        for guide in self.document.guides() {
            match guide.orientation {
                GuideOrientation::Vertical => {
                    for edge in [moved.x, right] {
                        let delta = guide.position - edge;
                        let distance = delta.abs();
                        if distance <= GUIDE_SNAP_THRESHOLD && distance < best_x_distance {
                            best_x_distance = distance;
                            snapped_x = translate_x + delta;
                        }
                    }
                }
                GuideOrientation::Horizontal => {
                    for edge in [moved.y, bottom] {
                        let delta = guide.position - edge;
                        let distance = delta.abs();
                        if distance <= GUIDE_SNAP_THRESHOLD && distance < best_y_distance {
                            best_y_distance = distance;
                            snapped_y = translate_y + delta;
                        }
                    }
                }
            }
        }

        (snapped_x, snapped_y)
    }

    fn push_guide_state_operation(
        &mut self,
        label: impl Into<String>,
        before_guides: Vec<Guide>,
        before_visible: bool,
    ) {
        self.mark_document_dirty();
        self.push_operation(
            label,
            EditorOperation::Guides(GuideStateRecord {
                before_guides,
                before_visible,
                after_guides: self.document.guides().to_vec(),
                after_visible: self.document.guides_visible(),
            }),
        );
    }

    fn active_layer_id(&self) -> common::LayerId {
        self.document.active_layer().id
    }

    fn reset_selected_structure_target_to_active_layer(&mut self) {
        self.selected_structure_target = LayerHierarchyNodeRef::Layer(self.active_layer_id());
    }

    fn selected_structure_name(&self) -> String {
        match self.selected_structure_target {
            LayerHierarchyNodeRef::Layer(layer_id) => self
                .document
                .layer_by_id(layer_id)
                .map(|layer| layer.name.clone())
                .unwrap_or_else(|| self.active_layer_name()),
            LayerHierarchyNodeRef::Group(group_id) => self
                .document
                .group(group_id)
                .map(|group| group.name.clone())
                .unwrap_or_else(|| self.active_layer_name()),
        }
    }

    fn selected_group_id(&self) -> Option<common::GroupId> {
        match self.selected_structure_target {
            LayerHierarchyNodeRef::Group(group_id) => Some(group_id),
            LayerHierarchyNodeRef::Layer(_) => None,
        }
    }

    fn push_layer_hierarchy_operation(
        &mut self,
        label: impl Into<String>,
        before: LayerHierarchySnapshot,
        after: LayerHierarchySnapshot,
    ) {
        self.push_operation(
            label,
            EditorOperation::LayerHierarchy(LayerHierarchyRecord {
                before_hierarchy: before.hierarchy,
                after_hierarchy: after.hierarchy,
                before_active_layer_id: before.active_layer_id,
                after_active_layer_id: after.active_layer_id,
                before_selected_target: before.selected_target,
                after_selected_target: after.selected_target,
            }),
        );
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

    fn clear_latest_import_report(&mut self) {
        self.latest_import_report = None;
    }

    fn shell_import_report_for_path(
        &mut self,
        path: &Path,
        report: PsdImportJobReport,
    ) -> ShellImportReport {
        let report_id = self.next_import_report_id;
        self.next_import_report_id = self.next_import_report_id.saturating_add(1);

        let title = if report.used_flattened_fallback {
            "PSD Imported As Flattened Composite".to_string()
        } else {
            "PSD Imported With Warnings".to_string()
        };
        let summary = if report.used_flattened_fallback {
            format!(
                "PhotoTux imported {} as a flattened composite because the source exceeded the current editable layered PSD subset.",
                path.display()
            )
        } else {
            format!(
                "PhotoTux imported {}, but some PSD features were not preserved as editable PhotoTux structure.",
                path.display()
            )
        };
        let diagnostics = report
            .diagnostics
            .into_iter()
            .map(|diagnostic| {
                let severity_label = match diagnostic.severity {
                    PsdImportDiagnosticSeverity::Info => "Info",
                    PsdImportDiagnosticSeverity::Warning => "Warning",
                    PsdImportDiagnosticSeverity::Error => "Error",
                    PsdImportDiagnosticSeverity::Other => "Notice",
                }
                .to_string();
                let message = match diagnostic.source_index {
                    Some(source_index) => {
                        format!("Source layer {}: {}", source_index, diagnostic.message)
                    }
                    None => diagnostic.message,
                };
                ShellImportDiagnostic {
                    severity_label,
                    code: diagnostic.code,
                    message,
                }
            })
            .collect();

        ShellImportReport {
            id: report_id,
            title,
            summary,
            diagnostics,
        }
    }

    fn current_brush_settings(&self, mode: BrushToolMode) -> BrushSettings {
        BrushSettings {
            radius: self.brush_radius,
            hardness: self.brush_hardness,
            opacity: 1.0,
            spacing: self.brush_spacing,
            flow: self.brush_flow,
            color: match mode {
                BrushToolMode::Paint => self.foreground_color,
                BrushToolMode::Erase => [0, 0, 0, 255],
            },
            pressure_size_enabled: self.pressure_size_enabled,
            pressure_opacity_enabled: self.pressure_opacity_enabled,
        }
    }

    fn clear_active_brush_preset(&mut self) {
        self.active_brush_preset = None;
    }

    fn apply_brush_preset(&mut self, preset: BrushPreset) {
        let (radius, hardness, spacing, flow, pressure_size_enabled, pressure_opacity_enabled) =
            preset.settings();
        self.brush_radius = radius;
        self.brush_hardness = hardness;
        self.brush_spacing = spacing;
        self.brush_flow = flow;
        self.pressure_size_enabled = pressure_size_enabled;
        self.pressure_opacity_enabled = pressure_opacity_enabled;
        self.active_brush_preset = Some(preset);
        self.status_message = format!("Brush preset {}", preset.label());
    }

    fn apply_active_layer_stroke_segment(
        &mut self,
        mode: BrushToolMode,
        samples: &[BrushSample],
    ) -> Option<BrushStrokeRecord> {
        let layer_index = self.document.active_layer_index();
        let settings = self.current_brush_settings(mode);
        let target = self.document.active_edit_target();

        if self.cached_canvas_raster.is_none() {
            self.cached_canvas_raster = Some(file_io::flatten_document_rgba(&self.document));
        }

        let record = BrushTool::apply_stroke_with_samples(
            &mut self.document,
            layer_index,
            samples,
            settings,
            mode,
            target,
        )?;

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
            if let Some(existing) = aggregate.changes.iter_mut().find(|existing| {
                existing.layer_id() == change.layer_id() && existing.coord() == change.coord()
            }) {
                match (existing, change) {
                    (
                        BrushChange::Pixels { after, .. },
                        BrushChange::Pixels {
                            after: next_after, ..
                        },
                    ) => {
                        *after = next_after;
                    }
                    (
                        BrushChange::Mask { after, .. },
                        BrushChange::Mask {
                            after: next_after, ..
                        },
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

    fn collect_layer_items(
        &self,
        nodes: &[LayerHierarchyNode],
        depth: usize,
        output: &mut Vec<LayerPanelItem>,
    ) {
        for node in nodes.iter().rev() {
            match node {
                LayerHierarchyNode::Layer(layer_id) => {
                    let Some(layer_index) = self.document.layer_index_by_id(*layer_id) else {
                        continue;
                    };
                    let Some(layer) = self.document.layer(layer_index) else {
                        continue;
                    };
                    output.push(LayerPanelItem {
                        index: Some(layer_index),
                        group_id: None,
                        name: layer.name.clone(),
                        depth,
                        is_group: false,
                        visible: layer.visible,
                        opacity_percent: layer.opacity_percent,
                        has_mask: layer.mask.is_some(),
                        mask_enabled: layer
                            .mask
                            .as_ref()
                            .map(|mask| mask.enabled)
                            .unwrap_or(false),
                        mask_target_active: layer_index == self.document.active_layer_index()
                            && self.document.active_edit_target() == LayerEditTarget::LayerMask,
                        is_selected: self.selected_structure_target
                            == LayerHierarchyNodeRef::Layer(*layer_id),
                        is_active: layer_index == self.document.active_layer_index(),
                    });
                }
                LayerHierarchyNode::Group(group) => {
                    output.push(LayerPanelItem {
                        index: None,
                        group_id: Some(group.id),
                        name: group.name.clone(),
                        depth,
                        is_group: true,
                        visible: group.visible,
                        opacity_percent: group.opacity_percent,
                        has_mask: false,
                        mask_enabled: false,
                        mask_target_active: false,
                        is_selected: self.selected_structure_target
                            == LayerHierarchyNodeRef::Group(group.id),
                        is_active: false,
                    });
                    self.collect_layer_items(&group.children, depth + 1, output);
                }
            }
        }
    }

    fn layer_items(&self) -> Vec<LayerPanelItem> {
        let mut items = Vec::new();
        self.collect_layer_items(self.document.layer_hierarchy(), 0, &mut items);
        items
    }

    fn create_group_from_active_layer_inner(&mut self) {
        let before_hierarchy = self.document.layer_hierarchy().to_vec();
        let before_active_layer_id = self.active_layer_id();
        let before_selected_target = self.selected_structure_target;
        let active_layer_name = self.active_layer_name();
        let group_name = format!("{} Group", active_layer_name);
        let Some(group_id) = self.document.wrap_hierarchy_node_in_group(
            LayerHierarchyNodeRef::Layer(before_active_layer_id),
            group_name,
        ) else {
            return;
        };
        self.selected_structure_target = LayerHierarchyNodeRef::Group(group_id);
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_layer_hierarchy_operation(
            format!("Group {}", active_layer_name),
            LayerHierarchySnapshot {
                hierarchy: before_hierarchy,
                active_layer_id: before_active_layer_id,
                selected_target: before_selected_target,
            },
            LayerHierarchySnapshot {
                hierarchy: self.document.layer_hierarchy().to_vec(),
                active_layer_id: self.active_layer_id(),
                selected_target: self.selected_structure_target,
            },
        );
        self.status_message = format!("Grouped {}", active_layer_name);
    }

    fn ungroup_selected_group_inner(&mut self) {
        let Some(group_id) = self.selected_group_id() else {
            return;
        };
        let group_name = self
            .document
            .group(group_id)
            .map(|group| group.name.clone())
            .unwrap_or_else(|| "Group".to_string());
        let before_hierarchy = self.document.layer_hierarchy().to_vec();
        let before_active_layer_id = self.active_layer_id();
        let before_selected_target = self.selected_structure_target;
        if !self.document.ungroup(group_id) {
            return;
        }
        self.reset_selected_structure_target_to_active_layer();
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_layer_hierarchy_operation(
            format!("Ungroup {}", group_name),
            LayerHierarchySnapshot {
                hierarchy: before_hierarchy,
                active_layer_id: before_active_layer_id,
                selected_target: before_selected_target,
            },
            LayerHierarchySnapshot {
                hierarchy: self.document.layer_hierarchy().to_vec(),
                active_layer_id: self.active_layer_id(),
                selected_target: self.selected_structure_target,
            },
        );
        self.status_message = format!("Ungrouped {}", group_name);
    }

    fn move_active_layer_into_selected_group_inner(&mut self) {
        let Some(group_id) = self.selected_group_id() else {
            return;
        };
        let active_layer_id = self.active_layer_id();
        let active_layer_name = self.active_layer_name();
        let group_name = self
            .document
            .group(group_id)
            .map(|group| group.name.clone())
            .unwrap_or_else(|| "Group".to_string());
        let before_hierarchy = self.document.layer_hierarchy().to_vec();
        let before_selected_target = self.selected_structure_target;
        if !self
            .document
            .move_node_into_group(LayerHierarchyNodeRef::Layer(active_layer_id), group_id)
        {
            return;
        }
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_layer_hierarchy_operation(
            format!("Move {} Into {}", active_layer_name, group_name),
            LayerHierarchySnapshot {
                hierarchy: before_hierarchy,
                active_layer_id,
                selected_target: before_selected_target,
            },
            LayerHierarchySnapshot {
                hierarchy: self.document.layer_hierarchy().to_vec(),
                active_layer_id: self.active_layer_id(),
                selected_target: self.selected_structure_target,
            },
        );
        self.status_message = format!("Moved {} into {}", active_layer_name, group_name);
    }

    fn move_active_layer_out_of_group_inner(&mut self) {
        let active_layer_id = self.active_layer_id();
        let active_layer_name = self.active_layer_name();
        let Some(group_id) = self.document.group_for_layer(active_layer_id) else {
            return;
        };
        let group_name = self
            .document
            .group(group_id)
            .map(|group| group.name.clone())
            .unwrap_or_else(|| "Group".to_string());
        let before_hierarchy = self.document.layer_hierarchy().to_vec();
        let before_selected_target = self.selected_structure_target;
        if !self
            .document
            .move_node_out_of_group(LayerHierarchyNodeRef::Layer(active_layer_id))
        {
            return;
        }
        self.selected_structure_target = LayerHierarchyNodeRef::Layer(active_layer_id);
        self.bump_canvas_revision();
        self.mark_document_dirty();
        self.push_layer_hierarchy_operation(
            format!("Move {} Out Of {}", active_layer_name, group_name),
            LayerHierarchySnapshot {
                hierarchy: before_hierarchy,
                active_layer_id,
                selected_target: before_selected_target,
            },
            LayerHierarchySnapshot {
                hierarchy: self.document.layer_hierarchy().to_vec(),
                active_layer_id: self.active_layer_id(),
                selected_target: self.selected_structure_target,
            },
        );
        self.status_message = format!("Moved {} out of {}", active_layer_name, group_name);
    }

    fn move_active_layer_by(&mut self, delta: isize) {
        let current = self.document.active_layer_index() as isize;
        let target =
            (current + delta).clamp(0, self.document.layer_count().saturating_sub(1) as isize);
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
        self.document
            .layer_canvas_bounds(self.document.active_layer_index())
    }

    fn primary_document_path(&self) -> PathBuf {
        self.document_path
            .clone()
            .unwrap_or_else(|| self.working_directory.join(self.save_file_name()))
    }

    fn refresh_recovery_path(&mut self) {
        self.recovery_path = Some(recovery_path_for_project_path(
            &self.primary_document_path(),
        ));
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
        self.pending_recovery_load_job =
            Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
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

    fn replace_document_after_load(&mut self, state: DocumentLoadState) {
        self.document = state.document;
        self.reset_selected_structure_target_to_active_layer();
        self.document_title = state.document_title;
        self.document_path = state.document_path;
        self.working_directory = state.working_directory;
        self.transform_session = None;
        self.interaction = None;
        self.active_tool = ShellToolKind::Brush;
        self.refresh_recovery_path();
        self.recovery_offer_pending = false;
        self.dirty_since_primary_save = state.dirty_since_primary_save;
        self.dirty_since_autosave = state.dirty_since_autosave;
        self.last_change_at = state.last_change_at;
        self.recompute_next_layer_number();
        self.reset_history_to(&state.history_label);
        self.bump_canvas_revision();
        self.status_message = state.status_message;
    }

    fn enqueue_primary_save(&mut self, path: PathBuf) {
        if self.pending_primary_save_job.is_some() {
            return;
        }

        let recovery_path = self.recovery_path.clone();
        let document = self.document.clone();
        self.status_message = format!("Saving {}", path.display());
        self.pending_primary_save_job =
            Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
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
        self.pending_autosave_job =
            Some(self.jobs.enqueue(JobPriority::Background, move |job_id| {
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
                        self.status_message =
                            format!("Recovered state written to {}", path.display());
                    }
                }
            },
            JobResult::SaveFailed {
                job_id,
                path,
                kind,
                error,
            } => {
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
                    self.reset_selected_structure_target_to_active_layer();
                    self.document_path = document_path;
                    self.document_title = document_title;
                    self.recovery_path = Some(recovery_path.clone());
                    self.dirty_since_primary_save = true;
                    self.dirty_since_autosave = false;
                    self.last_change_at = None;
                    self.bump_canvas_revision();
                    self.push_history("Recovered Autosave");
                    self.status_message =
                        format!("Recovered document from {}", recovery_path.display());
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
                import_notice,
                psd_import_report,
            } => {
                if self.pending_document_load_job == Some(job_id) {
                    self.pending_document_load_job = None;
                    let working_directory = path
                        .parent()
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| self.working_directory.clone());

                    match kind {
                        DocumentLoadKind::Project => {
                            self.clear_latest_import_report();
                            let document_title = path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("untitled.ptx")
                                .to_string();
                            self.replace_document_after_load(DocumentLoadState {
                                document,
                                document_title,
                                document_path: Some(path.clone()),
                                working_directory,
                                dirty_since_primary_save: false,
                                dirty_since_autosave: false,
                                last_change_at: None,
                                history_label: "Open Document".to_string(),
                                status_message: format!("Opened {}", path.display()),
                            });
                        }
                        DocumentLoadKind::RasterImport | DocumentLoadKind::PsdImport => {
                            self.clear_latest_import_report();
                            let stem = path
                                .file_stem()
                                .and_then(|name| name.to_str())
                                .unwrap_or("imported");
                            let history_label = match kind {
                                DocumentLoadKind::RasterImport => "Import Image",
                                DocumentLoadKind::PsdImport => "Import PSD",
                                DocumentLoadKind::Project => unreachable!("project handled above"),
                            };
                            let status_message = match import_notice {
                                Some(notice) => format!("Imported {} ({})", path.display(), notice),
                                None => format!("Imported {}", path.display()),
                            };
                            self.replace_document_after_load(DocumentLoadState {
                                document,
                                document_title: format!("{}.{}", stem, PROJECT_FILE_EXTENSION),
                                document_path: None,
                                working_directory,
                                dirty_since_primary_save: true,
                                dirty_since_autosave: true,
                                last_change_at: Some(Instant::now()),
                                history_label: history_label.to_string(),
                                status_message,
                            });
                            if let Some(psd_import_report) = psd_import_report {
                                self.latest_import_report = Some(
                                    self.shell_import_report_for_path(&path, psd_import_report),
                                );
                            }
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
                        DocumentLoadKind::RasterImport | DocumentLoadKind::PsdImport => {
                            format!("Import failed: {}", error)
                        }
                    };
                }
            }
            JobResult::ExportCompleted { job_id, path } => {
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
            JobResult::DestructiveFilterApplied {
                job_id,
                layer_id,
                filter,
                before,
                after,
            } => {
                if self
                    .pending_filter_job
                    .as_ref()
                    .map(|pending| pending.job_id)
                    == Some(job_id)
                {
                    let requested_canvas_revision = self
                        .pending_filter_job
                        .as_ref()
                        .map(|pending| pending.requested_canvas_revision)
                        .unwrap_or(self.canvas_revision);
                    self.pending_filter_job = None;

                    if self.canvas_revision != requested_canvas_revision {
                        self.status_message = format!(
                            "{} discarded because the document changed before it finished",
                            filter.label()
                        );
                        return;
                    }

                    if self
                        .document
                        .apply_layer_state_snapshot(layer_id, after.clone())
                    {
                        self.bump_canvas_revision();
                        self.mark_document_dirty();
                        self.push_operation(
                            format!("Filter {}", filter.label()),
                            EditorOperation::DestructiveFilter(DestructiveFilterRecord {
                                layer_id,
                                filter,
                                before,
                                after,
                            }),
                        );
                        self.status_message = format!("Applied {}", filter.label());
                    } else {
                        self.status_message =
                            format!("{} failed: layer is no longer available", filter.label());
                    }
                }
            }
            JobResult::DestructiveFilterFailed {
                job_id,
                filter,
                error,
            } => {
                if self
                    .pending_filter_job
                    .as_ref()
                    .map(|pending| pending.job_id)
                    == Some(job_id)
                {
                    self.pending_filter_job = None;
                    self.status_message = format!("{} failed: {}", filter.label(), error);
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
            && self.pending_filter_job.is_none()
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
        if self
            .document_title
            .ends_with(&format!(".{PROJECT_FILE_EXTENSION}"))
        {
            self.document_title.clone()
        } else {
            format!("{}.{}", self.document_title, PROJECT_FILE_EXTENSION)
        }
    }

    #[cfg(test)]
    fn save_document_in_directory(
        &mut self,
        base_dir: &std::path::Path,
    ) -> anyhow::Result<PathBuf> {
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
        self.document
            .set_layer_blend_mode(active_index, MODES[next_index]);
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
            scale_x: 1.0,
            scale_y: 1.0,
            rotate_quadrants: 0,
        });
        self.bump_canvas_revision();
    }

    fn transform_preview_rect(&self) -> Option<CanvasRect> {
        let session = self.transform_session.as_ref()?;
        let layer_index = self.document.layer_index_by_id(session.layer_id)?;
        SimpleTransformTool::preview_bounds(
            &self.document,
            layer_index,
            session.scale_x,
            session.scale_y,
            session.rotate_quadrants,
            session.translate_x,
            session.translate_y,
        )
    }

    fn preview_canvas_raster(&self) -> CanvasRaster {
        let mut preview_document = self.document.clone();
        if let Some(session) = &self.transform_session
            && let Some(layer_index) = preview_document.layer_index_by_id(session.layer_id)
        {
            let _ = SimpleTransformTool::transform_layer(
                &mut preview_document,
                layer_index,
                session.scale_x,
                session.scale_y,
                session.rotate_quadrants,
                session.translate_x,
                session.translate_y,
            );
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
            latest_import_report: self.latest_import_report.clone(),
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
            active_layer_mask_enabled: active_layer
                .mask
                .as_ref()
                .map(|mask| mask.enabled)
                .unwrap_or(false),
            active_edit_target_name: self.active_edit_target_name().to_string(),
            selected_structure_name: self.selected_structure_name(),
            selected_structure_is_group: matches!(
                self.selected_structure_target,
                LayerHierarchyNodeRef::Group(_)
            ),
            can_create_group_from_active_layer: true,
            can_ungroup_selected_group: matches!(
                self.selected_structure_target,
                LayerHierarchyNodeRef::Group(_)
            ),
            can_move_active_layer_into_selected_group: self
                .selected_group_id()
                .map(|group_id| {
                    self.document.group_for_layer(self.active_layer_id()) != Some(group_id)
                })
                .unwrap_or(false),
            can_move_active_layer_out_of_group: self
                .document
                .group_for_layer(self.active_layer_id())
                .is_some(),
            active_layer_bounds: self.active_layer_bounds(),
            transform_preview_rect: self.transform_preview_rect(),
            transform_active: self.transform_session.is_some(),
            transform_scale_percent: self
                .transform_session
                .as_ref()
                .map(|session| ((session.scale_x + session.scale_y) * 50.0).round() as u32)
                .unwrap_or(100),
            transform_scale_x_percent: self
                .transform_session
                .as_ref()
                .map(|session| (session.scale_x * 100.0).round() as u32)
                .unwrap_or(100),
            transform_scale_y_percent: self
                .transform_session
                .as_ref()
                .map(|session| (session.scale_y * 100.0).round() as u32)
                .unwrap_or(100),
            transform_rotation_degrees: self
                .transform_session
                .as_ref()
                .map(|session| session.rotate_quadrants.rem_euclid(4) * 90)
                .unwrap_or(0),
            can_apply_destructive_filters: self.can_apply_destructive_filters(),
            filter_job_active: self.pending_filter_job.is_some(),
            brush_preset_name: self
                .active_brush_preset
                .map(BrushPreset::label)
                .unwrap_or("Custom")
                .to_string(),
            brush_radius: self.brush_radius.round() as u32,
            brush_hardness_percent: (self.brush_hardness * 100.0).round() as u32,
            brush_spacing: self.brush_spacing.round() as u32,
            brush_flow_percent: (self.brush_flow * 100.0).round() as u32,
            pressure_size_enabled: self.pressure_size_enabled,
            pressure_opacity_enabled: self.pressure_opacity_enabled,
            snapping_enabled: self.snapping_enabled,
            snapping_temporarily_bypassed: self.snapping_temporarily_bypassed(),
            guides_visible: self.document.guides_visible(),
            guide_count: self.document.guides().len(),
            guides: self.shell_guides(),
            selection_rect: self.document.selection(),
            selection_path: self.selection_path_points(),
            selection_preview_path: self.selection_preview_path_points(),
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
        let _ = self
            .document
            .set_active_edit_target(LayerEditTarget::LayerMask);
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
        if self
            .document
            .set_active_edit_target(LayerEditTarget::LayerPixels)
        {
            self.mark_document_dirty();
            self.status_message = format!("Editing layer pixels for {}", self.active_layer_name());
        }
    }

    fn edit_active_layer_mask(&mut self) {
        if self.document.active_edit_target() == LayerEditTarget::LayerMask {
            return;
        }
        if self
            .document
            .set_active_edit_target(LayerEditTarget::LayerMask)
        {
            self.mark_document_dirty();
            self.status_message = format!("Editing layer mask for {}", self.active_layer_name());
        }
    }

    fn select_layer(&mut self, index: usize) {
        if self.document.set_active_layer(index) {
            self.selected_structure_target = LayerHierarchyNodeRef::Layer(self.active_layer_id());
        }
    }

    fn select_group(&mut self, group_id: common::GroupId) {
        if self.document.group(group_id).is_some() {
            self.selected_structure_target = LayerHierarchyNodeRef::Group(group_id);
            self.status_message = format!("Selected group {}", self.selected_structure_name());
        }
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

    fn toggle_group_visibility(&mut self, group_id: common::GroupId) {
        if let Some(group) = self.document.group(group_id) {
            let visible = !group.visible;
            let group_name = group.name.clone();
            if self.document.set_group_visibility(group_id, visible) {
                self.bump_canvas_revision();
                self.mark_document_dirty();
                self.push_history(format!("Toggle Group Visibility {}", group_name));
            }
        }
    }

    fn create_group_from_active_layer(&mut self) {
        self.create_group_from_active_layer_inner();
    }

    fn ungroup_selected_group(&mut self) {
        self.ungroup_selected_group_inner();
    }

    fn move_active_layer_into_selected_group(&mut self) {
        self.move_active_layer_into_selected_group_inner();
    }

    fn move_active_layer_out_of_group(&mut self) {
        self.move_active_layer_out_of_group_inner();
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
        let next_opacity = self
            .document
            .active_layer()
            .opacity_percent
            .saturating_sub(10);
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
        let before = self.document.selection_shape().cloned();
        let before_inverted = self.document.selection_inverted();
        if before.is_none() {
            return;
        }

        self.document.clear_selection();
        self.mark_document_dirty();
        self.push_operation(
            "Clear Selection",
            EditorOperation::Selection(SelectionRecord {
                before,
                before_inverted,
                after: None,
                after_inverted: false,
            }),
        );
    }

    fn invert_selection(&mut self) {
        let before = self.document.selection_shape().cloned();
        let before_inverted = self.document.selection_inverted();
        if !self.document.invert_selection() {
            return;
        }

        self.mark_document_dirty();
        self.push_operation(
            "Invert Selection",
            EditorOperation::Selection(SelectionRecord {
                before,
                before_inverted,
                after: self.document.selection_shape().cloned(),
                after_inverted: self.document.selection_inverted(),
            }),
        );
    }

    fn add_horizontal_guide(&mut self) {
        let before_guides = self.document.guides().to_vec();
        let before_visible = self.document.guides_visible();
        self.document.add_guide(Guide::horizontal(
            (self.document.canvas_size.height / 2) as i32,
        ));
        self.push_guide_state_operation("Add Horizontal Guide", before_guides, before_visible);
    }

    fn add_vertical_guide(&mut self) {
        let before_guides = self.document.guides().to_vec();
        let before_visible = self.document.guides_visible();
        self.document.add_guide(Guide::vertical(
            (self.document.canvas_size.width / 2) as i32,
        ));
        self.push_guide_state_operation("Add Vertical Guide", before_guides, before_visible);
    }

    fn remove_last_guide(&mut self) {
        let before_guides = self.document.guides().to_vec();
        let before_visible = self.document.guides_visible();
        if self.document.remove_last_guide().is_none() {
            return;
        }
        self.push_guide_state_operation("Remove Guide", before_guides, before_visible);
    }

    fn toggle_guides_visible(&mut self) {
        let before_guides = self.document.guides().to_vec();
        let before_visible = self.document.guides_visible();
        self.document.toggle_guides_visible();
        self.push_guide_state_operation("Toggle Guides", before_guides, before_visible);
    }

    fn toggle_snapping_enabled(&mut self) {
        self.snapping_enabled = !self.snapping_enabled;
        self.status_message = if self.snapping_enabled {
            "Guide snapping enabled".to_string()
        } else {
            "Guide snapping disabled".to_string()
        };
    }

    fn toggle_pressure_size_enabled(&mut self) {
        self.pressure_size_enabled = !self.pressure_size_enabled;
        self.clear_active_brush_preset();
        self.status_message = if self.pressure_size_enabled {
            "Pressure-to-size enabled".to_string()
        } else {
            "Pressure-to-size disabled".to_string()
        };
    }

    fn toggle_pressure_opacity_enabled(&mut self) {
        self.pressure_opacity_enabled = !self.pressure_opacity_enabled;
        self.clear_active_brush_preset();
        self.status_message = if self.pressure_opacity_enabled {
            "Pressure-to-opacity enabled".to_string()
        } else {
            "Pressure-to-opacity disabled".to_string()
        };
    }

    fn increase_brush_radius(&mut self) {
        self.brush_radius = (self.brush_radius + 2.0).clamp(1.0, 128.0);
        self.clear_active_brush_preset();
        self.status_message = format!("Brush radius {} px", self.brush_radius.round() as u32);
    }

    fn decrease_brush_radius(&mut self) {
        self.brush_radius = (self.brush_radius - 2.0).clamp(1.0, 128.0);
        self.clear_active_brush_preset();
        self.status_message = format!("Brush radius {} px", self.brush_radius.round() as u32);
    }

    fn increase_brush_hardness(&mut self) {
        self.brush_hardness = (self.brush_hardness + 0.05).clamp(0.0, 1.0);
        self.clear_active_brush_preset();
        self.status_message = format!(
            "Brush hardness {}%",
            (self.brush_hardness * 100.0).round() as u32
        );
    }

    fn decrease_brush_hardness(&mut self) {
        self.brush_hardness = (self.brush_hardness - 0.05).clamp(0.0, 1.0);
        self.clear_active_brush_preset();
        self.status_message = format!(
            "Brush hardness {}%",
            (self.brush_hardness * 100.0).round() as u32
        );
    }

    fn increase_brush_spacing(&mut self) {
        self.brush_spacing = (self.brush_spacing + 1.0).clamp(1.0, 64.0);
        self.clear_active_brush_preset();
        self.status_message = format!("Brush spacing {} px", self.brush_spacing.round() as u32);
    }

    fn decrease_brush_spacing(&mut self) {
        self.brush_spacing = (self.brush_spacing - 1.0).clamp(1.0, 64.0);
        self.clear_active_brush_preset();
        self.status_message = format!("Brush spacing {} px", self.brush_spacing.round() as u32);
    }

    fn increase_brush_flow(&mut self) {
        self.brush_flow = (self.brush_flow + 0.05).clamp(0.05, 1.0);
        self.clear_active_brush_preset();
        self.status_message = format!("Brush flow {}%", (self.brush_flow * 100.0).round() as u32);
    }

    fn decrease_brush_flow(&mut self) {
        self.brush_flow = (self.brush_flow - 0.05).clamp(0.05, 1.0);
        self.clear_active_brush_preset();
        self.status_message = format!("Brush flow {}%", (self.brush_flow * 100.0).round() as u32);
    }

    fn next_brush_preset(&mut self) {
        let next = self
            .active_brush_preset
            .map(BrushPreset::next)
            .unwrap_or(BrushPreset::BalancedRound);
        self.apply_brush_preset(next);
    }

    fn previous_brush_preset(&mut self) {
        let previous = self
            .active_brush_preset
            .map(BrushPreset::previous)
            .unwrap_or(BrushPreset::InkTaper);
        self.apply_brush_preset(previous);
    }

    fn set_temporary_snap_bypass(&mut self, bypassed: bool) {
        self.temporary_snap_bypass = bypassed;
    }

    fn begin_transform(&mut self) {
        self.begin_transform_session_if_needed();
    }

    fn scale_transform_up(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.scale_x = (session.scale_x + 0.1).min(4.0);
        session.scale_y = (session.scale_y + 0.1).min(4.0);
        self.bump_canvas_revision();
    }

    fn scale_transform_down(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.scale_x = (session.scale_x - 0.1).max(0.1);
        session.scale_y = (session.scale_y - 0.1).max(0.1);
        self.bump_canvas_revision();
    }

    fn scale_transform_x_up(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.scale_x = (session.scale_x + 0.1).min(4.0);
        self.bump_canvas_revision();
    }

    fn scale_transform_x_down(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.scale_x = (session.scale_x - 0.1).max(0.1);
        self.bump_canvas_revision();
    }

    fn scale_transform_y_up(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.scale_y = (session.scale_y + 0.1).min(4.0);
        self.bump_canvas_revision();
    }

    fn scale_transform_y_down(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.scale_y = (session.scale_y - 0.1).max(0.1);
        self.bump_canvas_revision();
    }

    fn rotate_transform_left(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.rotate_quadrants -= 1;
        self.bump_canvas_revision();
    }

    fn rotate_transform_right(&mut self) {
        self.begin_transform_session_if_needed();
        let Some(session) = &mut self.transform_session else {
            return;
        };
        session.rotate_quadrants += 1;
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
            session.scale_x,
            session.scale_y,
            session.rotate_quadrants,
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
                EditorOperation::Guides(record) => {
                    record.undo(&mut self.document);
                    self.mark_document_dirty();
                }
                EditorOperation::MaskState(record) => {
                    record.undo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::LayerHierarchy(record) => {
                    record.undo(self);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::DestructiveFilter(record) => {
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
                EditorOperation::Guides(record) => {
                    record.redo(&mut self.document);
                    self.mark_document_dirty();
                }
                EditorOperation::MaskState(record) => {
                    record.redo(&mut self.document);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::LayerHierarchy(record) => {
                    record.redo(self);
                    self.bump_canvas_revision();
                    self.mark_document_dirty();
                }
                EditorOperation::DestructiveFilter(record) => {
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
        self.pending_recovery_load_job =
            Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
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
                self.status_message =
                    format!("Discarded recovery file {}", recovery_path.display());
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
            self.status_message = format!(
                "Open failed: expected .{} project file",
                PROJECT_FILE_EXTENSION
            );
            return;
        }

        self.status_message = format!("Opening {}", path.display());
        self.clear_latest_import_report();
        self.pending_document_load_job =
            Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
                JobRequest::LoadDocument {
                    job_id,
                    path,
                    kind: DocumentLoadKind::Project,
                    psd_import_sidecar: None,
                }
            }));
    }

    fn import_image(&mut self, path: PathBuf) {
        if self.has_pending_user_visible_file_job() {
            self.status_message = "Another file operation is already in progress".to_string();
            return;
        }

        let kind = if raster_format_from_path(&path).is_some() {
            DocumentLoadKind::RasterImport
        } else if psd_file_path(&path) {
            if self.psd_import_sidecar.is_none() {
                self.status_message = format!(
                    "Import failed: PSD import sidecar is not configured; set {}",
                    PSD_IMPORT_SIDECAR_PATH_ENV
                );
                return;
            }
            DocumentLoadKind::PsdImport
        } else {
            self.status_message = "Import failed: unsupported import format".to_string();
            return;
        };

        self.status_message = format!("Importing {}", path.display());
        self.clear_latest_import_report();
        let psd_import_sidecar = self.psd_import_sidecar.clone();
        self.pending_document_load_job =
            Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
                JobRequest::LoadDocument {
                    job_id,
                    path,
                    kind,
                    psd_import_sidecar,
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
        self.pending_export_job =
            Some(self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
                JobRequest::ExportDocument {
                    job_id,
                    path,
                    document,
                    format,
                }
            }));
    }

    fn apply_destructive_filter(&mut self, filter: DestructiveFilterKind) {
        if self.pending_filter_job.is_some() {
            self.status_message = "Another filter is already in progress".to_string();
            return;
        }
        if self.transform_session.is_some() {
            self.status_message =
                "Commit or cancel the active transform before applying a filter".to_string();
            return;
        }
        if self.document.active_edit_target() != LayerEditTarget::LayerPixels {
            self.status_message =
                "Destructive filters currently apply to layer pixels only".to_string();
            return;
        }

        let layer_index = self.document.active_layer_index();
        let layer_id = self.document.active_layer().id;
        let Some(before) = self.document.layer_state_snapshot(layer_index) else {
            self.status_message = format!("{} failed: active layer is unavailable", filter.label());
            return;
        };

        let requested_canvas_revision = self.canvas_revision;
        self.status_message = format!("Applying {}", filter.label());
        let pending_job_id = self.jobs.enqueue(JobPriority::UserVisible, move |job_id| {
            JobRequest::ApplyDestructiveFilter {
                job_id,
                layer_id,
                filter,
                before,
            }
        });
        self.pending_filter_job = Some(PendingFilterJob {
            job_id: pending_job_id,
            requested_canvas_revision,
        });
    }

    fn poll_background_tasks(&mut self) {
        self.poll_background_tasks_at(Instant::now());
    }

    fn select_tool(&mut self, tool: ShellToolKind) {
        self.active_tool = tool;
    }

    fn begin_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32) {
        self.begin_canvas_interaction_with_pressure(canvas_x, canvas_y, 1.0);
    }

    fn begin_canvas_interaction_with_pressure(
        &mut self,
        canvas_x: i32,
        canvas_y: i32,
        pressure: f32,
    ) {
        match self.active_tool {
            ShellToolKind::Move => {
                let layer_index = self.document.active_layer_index();
                let (start_offset_x, start_offset_y) =
                    self.document.layer_offset(layer_index).unwrap_or((0, 0));
                self.interaction = Some(CanvasInteraction::Move {
                    layer_id: self.document.active_layer().id,
                    start_canvas_x: canvas_x,
                    start_canvas_y: canvas_y,
                    start_offset_x,
                    start_offset_y,
                    initial_state: self
                        .document
                        .selection_shape()
                        .and_then(|_| self.document.layer_state_snapshot(layer_index)),
                    snapping_base_bounds: self.move_snapping_base_bounds(),
                });
            }
            ShellToolKind::RectangularMarquee => {
                self.interaction = Some(CanvasInteraction::Marquee {
                    before: self.document.selection_shape().cloned(),
                    before_inverted: self.document.selection_inverted(),
                    start_canvas_x: canvas_x,
                    start_canvas_y: canvas_y,
                });
            }
            ShellToolKind::Lasso => {
                self.interaction = Some(CanvasInteraction::Lasso {
                    before: self.document.selection_shape().cloned(),
                    before_inverted: self.document.selection_inverted(),
                    points: vec![(canvas_x, canvas_y)],
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
                        initial_state: None,
                        snapping_base_bounds: self.transform_snapping_base_bounds(),
                    });
                }
            }
            ShellToolKind::Brush | ShellToolKind::Eraser => {
                let mode = if self.active_tool == ShellToolKind::Brush {
                    BrushToolMode::Paint
                } else {
                    BrushToolMode::Erase
                };
                let aggregate = self.apply_active_layer_stroke_segment(
                    mode,
                    &[BrushSample::new(canvas_x as f32, canvas_y as f32, pressure)],
                );
                if aggregate.is_some() {
                    self.dirty_since_primary_save = true;
                    self.dirty_since_autosave = true;
                    self.last_change_at = Some(std::time::Instant::now());
                }
                self.interaction = Some(CanvasInteraction::Brush {
                    mode,
                    last_canvas_x: canvas_x,
                    last_canvas_y: canvas_y,
                    last_pressure: pressure,
                    aggregate,
                });
            }
            _ => {}
        }
    }

    fn update_canvas_interaction(&mut self, canvas_x: i32, canvas_y: i32) {
        self.update_canvas_interaction_with_pressure(canvas_x, canvas_y, 1.0);
    }

    fn update_canvas_interaction_with_pressure(
        &mut self,
        canvas_x: i32,
        canvas_y: i32,
        pressure: f32,
    ) {
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
                initial_state,
                snapping_base_bounds,
            } => {
                let raw_delta_x = canvas_x - start_canvas_x;
                let raw_delta_y = canvas_y - start_canvas_y;
                if self.active_tool == ShellToolKind::Transform {
                    let (translate_x, translate_y) = self.snapped_translation(
                        snapping_base_bounds,
                        start_offset_x + raw_delta_x,
                        start_offset_y + raw_delta_y,
                    );
                    if let Some(session) = &mut self.transform_session {
                        session.translate_x = translate_x;
                        session.translate_y = translate_y;
                    }
                } else if let Some(initial_state) = &initial_state {
                    let layer_index = self.document.active_layer_index();
                    let _ = self
                        .document
                        .apply_layer_state_snapshot(layer_id, initial_state.clone());
                    let (translate_x, translate_y) =
                        self.snapped_translation(snapping_base_bounds, raw_delta_x, raw_delta_y);
                    let _ = SimpleTransformTool::transform_layer(
                        &mut self.document,
                        layer_index,
                        1.0,
                        1.0,
                        0,
                        translate_x,
                        translate_y,
                    );
                    self.cached_canvas_raster =
                        Some(file_io::flatten_document_rgba(&self.document));
                    self.dirty_since_primary_save = true;
                    self.dirty_since_autosave = true;
                    self.last_change_at = Some(std::time::Instant::now());
                } else {
                    let layer_index = self.document.active_layer_index();
                    let old_bounds = self.document.layer_canvas_bounds(layer_index);
                    let (translate_x, translate_y) = self.snapped_translation(
                        snapping_base_bounds,
                        start_offset_x + raw_delta_x,
                        start_offset_y + raw_delta_y,
                    );

                    let _ = self
                        .document
                        .set_layer_offset(layer_index, translate_x, translate_y);

                    let new_bounds = self.document.layer_canvas_bounds(layer_index);

                    if self.cached_canvas_raster.is_none() {
                        self.cached_canvas_raster =
                            Some(file_io::flatten_document_rgba(&self.document));
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
                    initial_state,
                    snapping_base_bounds,
                })
            }
            CanvasInteraction::Marquee {
                before,
                before_inverted,
                start_canvas_x,
                start_canvas_y,
            } => {
                if let Some(rect) = RectangularMarqueeTool::preview_rect(
                    start_canvas_x,
                    start_canvas_y,
                    canvas_x,
                    canvas_y,
                ) {
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
            CanvasInteraction::Lasso {
                before,
                before_inverted,
                mut points,
            } => {
                if points.last().copied() != Some((canvas_x, canvas_y)) {
                    points.push((canvas_x, canvas_y));
                }
                self.dirty_since_primary_save = true;
                self.dirty_since_autosave = true;
                self.last_change_at = Some(std::time::Instant::now());
                Some(CanvasInteraction::Lasso {
                    before,
                    before_inverted,
                    points,
                })
            }
            CanvasInteraction::Brush {
                mode,
                last_canvas_x,
                last_canvas_y,
                last_pressure,
                mut aggregate,
            } => {
                if (last_canvas_x != canvas_x || last_canvas_y != canvas_y)
                    && let Some(segment) = self.apply_active_layer_stroke_segment(
                        mode,
                        &[
                            BrushSample::new(
                                last_canvas_x as f32,
                                last_canvas_y as f32,
                                last_pressure,
                            ),
                            BrushSample::new(canvas_x as f32, canvas_y as f32, pressure),
                        ],
                    )
                {
                    if let Some(existing) = &mut aggregate {
                        Self::merge_brush_records(existing, segment);
                    } else {
                        aggregate = Some(segment);
                    }
                    self.dirty_since_primary_save = true;
                    self.dirty_since_autosave = true;
                    self.last_change_at = Some(std::time::Instant::now());
                }

                Some(CanvasInteraction::Brush {
                    mode,
                    last_canvas_x: canvas_x,
                    last_canvas_y: canvas_y,
                    last_pressure: pressure,
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
                initial_state,
                ..
            }) => {
                if self.active_tool == ShellToolKind::Transform {
                    return;
                }
                if let Some(before_state) = initial_state {
                    let Some(after_state) = self
                        .document
                        .layer_index_by_id(layer_id)
                        .and_then(|index| self.document.layer_state_snapshot(index))
                    else {
                        return;
                    };
                    if before_state != after_state {
                        self.push_operation(
                            "Move Selection",
                            EditorOperation::TransformLayer(LayerTransformRecord {
                                layer_id,
                                before: before_state,
                                after: after_state,
                            }),
                        );
                    }
                } else {
                    let (current_x, current_y) = self
                        .document
                        .layer_offset(self.document.active_layer_index())
                        .unwrap_or((0, 0));
                    let delta_x = current_x - start_offset_x;
                    let delta_y = current_y - start_offset_y;
                    if delta_x != 0 || delta_y != 0 {
                        self.push_operation(
                            format!(
                                "Move Layer {} ({}, {})",
                                self.active_layer_name(),
                                delta_x,
                                delta_y
                            ),
                            EditorOperation::MoveLayer(MoveLayerRecord {
                                layer_id,
                                before_offset: (start_offset_x, start_offset_y),
                                after_offset: (current_x, current_y),
                            }),
                        );
                    }
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
                        EditorOperation::Selection(SelectionRecord {
                            before,
                            before_inverted,
                            after: Some(SelectionShape::Rectangular(selection)),
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
            Some(CanvasInteraction::Lasso {
                before,
                before_inverted,
                points,
            }) => {
                if let Some(record) = LassoTool::apply_selection(&mut self.document, &points) {
                    self.mark_document_dirty();
                    self.push_operation("Lasso Selection", EditorOperation::Selection(record));
                } else if before.is_some() && !before_inverted {
                    self.mark_document_dirty();
                }
            }
            Some(CanvasInteraction::Brush {
                mode,
                aggregate: Some(record),
                ..
            }) => {
                let label = match (record.target, mode) {
                    (LayerEditTarget::LayerPixels, BrushToolMode::Paint) => "Brush Stroke",
                    (LayerEditTarget::LayerPixels, BrushToolMode::Erase) => "Erase Stroke",
                    (LayerEditTarget::LayerMask, BrushToolMode::Paint) => "Mask Hide Stroke",
                    (LayerEditTarget::LayerMask, BrushToolMode::Erase) => "Mask Reveal Stroke",
                };
                self.push_operation(label, EditorOperation::BrushStroke(record));
            }
            Some(CanvasInteraction::Brush {
                aggregate: None, ..
            }) => {}
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AUTOSAVE_IDLE_INTERVAL, PhotoTuxController};
    use common::{CanvasRect, DestructiveFilterKind};
    use file_io::{
        PsdImportSidecar, export_png_to_path, flatten_document_rgba, load_document_from_path,
        recovery_path_for_project_path, save_document_to_path,
    };
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    #[cfg(unix)]
    use std::process::Command;
    use std::thread;
    use std::time::{Duration, Instant};
    use ui_shell::{ShellController, ShellGuide, ShellToolKind};

    fn set_pixel(
        document: &mut doc_model::Document,
        layer_index: usize,
        x: u32,
        y: u32,
        rgba: [u8; 4],
    ) {
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

    fn set_mask_alpha(
        document: &mut doc_model::Document,
        layer_index: usize,
        x: u32,
        y: u32,
        alpha: u8,
    ) {
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

    fn build_lasso_transform_fixture_document() -> doc_model::Document {
        let mut document = doc_model::Document::new(96, 96);
        document.rename_layer(0, "Subject");

        for y in 18..54 {
            for x in 14..46 {
                set_pixel(&mut document, 0, x, y, [210, 120, 80, 255]);
            }
        }

        for y in 30..66 {
            for x in 52..84 {
                set_pixel(&mut document, 0, x, y, [70, 150, 220, 255]);
            }
        }

        let freeform = doc_model::FreeformSelection::new(vec![
            doc_model::SelectionPoint::new(12, 16),
            doc_model::SelectionPoint::new(50, 18),
            doc_model::SelectionPoint::new(44, 58),
            doc_model::SelectionPoint::new(16, 60),
        ])
        .expect("lasso transform fixture selection should be valid");
        document.set_freeform_selection(freeform);

        document
    }

    fn build_guided_snapping_fixture_document() -> doc_model::Document {
        let mut document = doc_model::Document::new(128, 128);
        set_pixel(&mut document, 0, 0, 0, [255, 255, 255, 255]);
        document.add_guide(doc_model::Guide::vertical(32));
        document.add_guide(doc_model::Guide::horizontal(16));
        document
    }

    fn build_medium_paint_fixture_document() -> doc_model::Document {
        let mut document = doc_model::Document::new(1024, 768);
        document.rename_layer(0, "Background");

        for y in 0..96 {
            for x in 0..96 {
                set_pixel(&mut document, 0, x, y, [28, 32, 40, 255]);
            }
        }

        document.add_layer("Paint");
        document
    }

    fn build_supported_psd_import_document() -> doc_model::Document {
        let mut document = doc_model::Document::new(6, 6);
        document.rename_layer(0, "Background");
        for y in 0..3 {
            for x in 0..3 {
                set_pixel(&mut document, 0, x, y, [20, 40, 80, 255]);
            }
        }

        document.add_layer("Screen Accent");
        let top_index = document.active_layer_index();
        document.set_layer_blend_mode(top_index, doc_model::BlendMode::Screen);
        document.set_layer_opacity(top_index, 50);
        assert!(document.set_layer_offset(top_index, 2, 1));
        for y in 0..2 {
            for x in 0..2 {
                set_pixel(&mut document, top_index, x, y, [240, 180, 100, 255]);
            }
        }

        document
    }

    fn write_rgba_document_png(path: &Path, width: u32, height: u32, pixels: &[[u8; 4]]) {
        let mut document = doc_model::Document::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let pixel = pixels[(y * width + x) as usize];
                if pixel[3] > 0 {
                    set_pixel(&mut document, 0, x, y, pixel);
                }
            }
        }
        export_png_to_path(path, &document).expect("PNG asset should be exported");
    }

    fn supported_psd_manifest_json() -> String {
        r#"{
    "manifest_version": 1,
    "source_kind": "psd",
    "source_color_mode": "rgb",
    "source_depth_bits": 8,
    "canvas": {
        "width_px": 6,
        "height_px": 6
    },
    "composite": {
        "available": false,
        "asset_relpath": null
    },
    "diagnostics": [
        {
            "severity": "info",
            "code": "source_loaded",
            "message": "PSD manifest decoded successfully.",
            "source_index": null
        }
    ],
    "layers": [
        {
            "source_index": 0,
            "kind": "raster",
            "name": "Background",
            "visible": true,
            "opacity_0_255": 255,
            "blend_key": "norm",
            "offset_px": { "x": 0, "y": 0 },
            "bounds_px": { "left": 0, "top": 0, "width": 3, "height": 3 },
            "raster_asset_relpath": "layers/000-background.png",
            "unsupported_features": []
        },
        {
            "source_index": 1,
            "kind": "raster",
            "name": "Screen Accent",
            "visible": true,
            "opacity_0_255": 128,
            "blend_key": "scrn",
            "offset_px": { "x": 2, "y": 1 },
            "bounds_px": { "left": 2, "top": 1, "width": 2, "height": 2 },
            "raster_asset_relpath": "layers/001-screen.png",
            "unsupported_features": []
        }
    ]
}"#
        .to_string()
    }

    #[cfg(unix)]
    fn write_shell_script(path: &Path, contents: &str) {
        fs::write(path, contents).expect("shell script should be written");
        let mut permissions = fs::metadata(path)
            .expect("shell script metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("shell script permissions should be updated");
    }

    #[cfg(unix)]
    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repository root should resolve from the app_core crate")
    }

    #[cfg(unix)]
    fn repo_psd_fixture_path(file_name: &str) -> PathBuf {
        repo_root().join("tests/fixtures/psd").join(file_name)
    }

    #[cfg(unix)]
    fn repo_psd_sidecar_script_path() -> PathBuf {
        repo_root().join("tools/psd_import_sidecar/phototux_psd_sidecar.py")
    }

    #[cfg(unix)]
    fn repo_psd_sidecar_runtime_available() -> bool {
        if !repo_psd_sidecar_script_path().is_file() {
            return false;
        }

        match Command::new("python3")
            .args(["-c", "import psd_tools"])
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("phototux-{prefix}-{}-{nanos}", std::process::id()))
    }

    fn first_group_id(document: &doc_model::Document) -> common::GroupId {
        document
            .layer_hierarchy()
            .iter()
            .find_map(|node| match node {
                doc_model::LayerHierarchyNode::Group(group) => Some(group.id),
                doc_model::LayerHierarchyNode::Layer(_) => None,
            })
            .expect("expected a top-level group")
    }

    fn flattened_fallback_psd_manifest_json() -> String {
        r#"{
    "manifest_version": 1,
    "source_kind": "psd",
    "source_color_mode": "rgb",
    "source_depth_bits": 8,
    "canvas": {
        "width_px": 6,
        "height_px": 6
    },
    "composite": {
        "available": true,
        "asset_relpath": "composite.png"
    },
    "diagnostics": [
        {
            "severity": "info",
            "code": "source_loaded",
            "message": "PSD manifest decoded successfully.",
            "source_index": null
        }
    ],
    "layers": [
        {
            "source_index": 0,
            "kind": "text",
            "name": "Title",
            "visible": true,
            "opacity_0_255": 255,
            "blend_key": "norm",
            "offset_px": { "x": 1, "y": 1 },
            "bounds_px": { "left": 1, "top": 1, "width": 4, "height": 2 },
            "raster_asset_relpath": null,
            "unsupported_features": ["text_engine_data"]
        }
    ]
}"#
        .to_string()
    }

    fn wait_for_background_jobs(controller: &mut PhotoTuxController) {
        for _ in 0..50 {
            controller.poll_background_tasks_at(Instant::now());
            if controller.pending_primary_save_job.is_none()
                && controller.pending_autosave_job.is_none()
                && controller.pending_recovery_load_job.is_none()
                && controller.pending_document_load_job.is_none()
                && controller.pending_export_job.is_none()
                && controller.pending_filter_job.is_none()
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
        assert!(
            snapshot
                .history_entries
                .iter()
                .any(|entry| entry.contains("Add Layer"))
        );
        assert!(
            snapshot
                .history_entries
                .iter()
                .any(|entry| entry.contains("Duplicate Layer"))
        );
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
        let working_directory =
            std::env::temp_dir().join(format!("phototux-save-as-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary save-as directory should exist");

        let mut controller =
            PhotoTuxController::new_with_working_directory(working_directory.clone());
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
        let working_directory =
            std::env::temp_dir().join(format!("phototux-save-undo-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary save+undo directory should exist");

        let mut controller =
            PhotoTuxController::new_with_working_directory(working_directory.clone());
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
        fs::remove_dir(&working_directory)
            .expect("temporary save+undo directory should be removed");
    }

    #[test]
    fn autosave_writes_recovery_file_after_idle_period() {
        let working_directory =
            std::env::temp_dir().join(format!("phototux-autosave-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary autosave directory should exist");

        let mut controller =
            PhotoTuxController::new_with_working_directory(working_directory.clone());
        controller.add_layer();
        controller.last_change_at =
            Some(Instant::now() - AUTOSAVE_IDLE_INTERVAL - Duration::from_millis(1));

        controller.poll_background_tasks_at(Instant::now());
        wait_for_background_jobs(&mut controller);

        let recovery_path = recovery_path_for_project_path(&working_directory.join("untitled.ptx"));
        let recovered =
            load_document_from_path(&recovery_path).expect("autosave recovery file should load");
        assert_eq!(recovered.layers.len(), controller.document.layers.len());
        assert!(!controller.dirty_since_autosave);

        fs::remove_file(&recovery_path).expect("autosave recovery file should be removed");
        fs::remove_dir(&working_directory).expect("temporary autosave directory should be removed");
    }

    #[test]
    fn startup_recovery_offer_requires_explicit_load() {
        let working_directory =
            std::env::temp_dir().join(format!("phototux-recovery-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary recovery directory should exist");

        let mut recovered_document = doc_model::Document::new(320, 240);
        recovered_document.add_layer("Recovered Layer");
        let recovery_path = recovery_path_for_project_path(&working_directory.join("untitled.ptx"));
        save_document_to_path(&recovery_path, &recovered_document)
            .expect("recovery document should save");

        let mut controller =
            PhotoTuxController::new_with_working_directory(working_directory.clone());
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
        assert!(
            controller
                .snapshot()
                .history_entries
                .iter()
                .any(|entry| entry.contains("Recovered Autosave"))
        );

        fs::remove_file(&recovery_path).expect("recovery file should be removed");
        fs::remove_dir(&working_directory).expect("temporary recovery directory should be removed");
    }

    #[test]
    fn discard_recovery_offer_removes_recovery_file() {
        let working_directory =
            std::env::temp_dir().join(format!("phototux-recovery-discard-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary recovery directory should exist");

        let mut recovered_document = doc_model::Document::new(320, 240);
        recovered_document.add_layer("Recovered Layer");
        let recovery_path = recovery_path_for_project_path(&working_directory.join("untitled.ptx"));
        save_document_to_path(&recovery_path, &recovered_document)
            .expect("recovery document should save");

        let mut controller =
            PhotoTuxController::new_with_working_directory(working_directory.clone());
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
        let working_directory =
            std::env::temp_dir().join(format!("phototux-open-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary open directory should exist");

        let document = build_representative_controller_document();
        let project_path = working_directory.join("scene.ptx");
        save_document_to_path(&project_path, &document)
            .expect("project should save for reopen test");

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
        assert_eq!(
            controller.snapshot().history_entries,
            vec!["Open Document".to_string()]
        );

        fs::remove_file(&project_path).expect("project file should be removed");
        fs::remove_dir(&working_directory).expect("temporary open directory should be removed");
    }

    #[test]
    fn export_and_import_commands_roundtrip_through_background_jobs() {
        let working_directory =
            std::env::temp_dir().join(format!("phototux-io-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary io directory should exist");

        let mut export_controller =
            PhotoTuxController::new_with_working_directory(working_directory.clone());
        export_controller.document = build_representative_controller_document();
        let export_path = working_directory.join("scene.png");

        export_controller.export_document(export_path.clone());
        wait_for_background_jobs(&mut export_controller);

        assert!(export_path.exists());
        assert!(export_controller.status_message.contains("Exported"));

        let mut import_controller = PhotoTuxController::new();
        import_controller.import_image(export_path.clone());
        wait_for_background_jobs(&mut import_controller);

        assert_eq!(
            import_controller.document.canvas_size,
            export_controller.document.canvas_size
        );
        assert_eq!(import_controller.document.layer_count(), 1);
        assert!(import_controller.document_path.is_none());
        assert!(import_controller.dirty_since_primary_save);
        assert!(import_controller.dirty_since_autosave);
        assert!(import_controller.status_message.contains("Imported"));
        assert_eq!(
            import_controller.snapshot().history_entries,
            vec!["Import Image".to_string()]
        );

        fs::remove_file(&export_path).expect("exported png should be removed");
        fs::remove_dir(&working_directory).expect("temporary io directory should be removed");
    }

    #[test]
    fn psd_import_requires_configured_sidecar() {
        let working_directory =
            std::env::temp_dir().join(format!("phototux-psd-missing-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary PSD directory should exist");
        let source_path = working_directory.join("scene.psd");
        fs::write(&source_path, b"placeholder psd source").expect("PSD source should be written");

        let mut controller = PhotoTuxController::new_with_working_directory_and_psd_sidecar(
            working_directory.clone(),
            None,
        );
        controller.import_image(source_path.clone());

        assert!(controller.pending_document_load_job.is_none());
        assert!(
            controller
                .status_message
                .contains("PSD import sidecar is not configured")
        );

        fs::remove_file(&source_path).expect("PSD source should be removed");
        fs::remove_dir(&working_directory).expect("temporary PSD directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn psd_import_roundtrips_through_background_jobs() {
        let working_directory =
            std::env::temp_dir().join(format!("phototux-psd-import-{}", std::process::id()));
        fs::create_dir_all(&working_directory).expect("temporary PSD directory should exist");
        let fixture_dir = working_directory.join("fixture");
        fs::create_dir_all(fixture_dir.join("layers")).expect("fixture layers should exist");

        let manifest_path = fixture_dir.join("fixture-manifest.json");
        fs::write(&manifest_path, supported_psd_manifest_json())
            .expect("PSD manifest should be written");
        write_rgba_document_png(
            &fixture_dir.join("layers/000-background.png"),
            3,
            3,
            &[
                [20, 40, 80, 255],
                [20, 40, 80, 255],
                [20, 40, 80, 255],
                [20, 40, 80, 255],
                [20, 40, 80, 255],
                [20, 40, 80, 255],
                [20, 40, 80, 255],
                [20, 40, 80, 255],
                [20, 40, 80, 255],
            ],
        );
        write_rgba_document_png(
            &fixture_dir.join("layers/001-screen.png"),
            2,
            2,
            &[
                [240, 180, 100, 255],
                [240, 180, 100, 255],
                [240, 180, 100, 255],
                [240, 180, 100, 255],
            ],
        );

        let source_path = working_directory.join("scene.psd");
        fs::write(&source_path, b"placeholder psd source").expect("PSD source should be written");
        let script_path = working_directory.join("psd-sidecar.sh");
        write_shell_script(
            &script_path,
            &format!(
                "#!/bin/sh\nset -eu\nSOURCE=\"$1\"\nWORKSPACE=\"$2\"\nMANIFEST=\"$3\"\n[ -f \"$SOURCE\" ]\ncp \"{}\" \"$MANIFEST\"\nmkdir -p \"$WORKSPACE/layers\"\ncp \"{}\" \"$WORKSPACE/layers/000-background.png\"\ncp \"{}\" \"$WORKSPACE/layers/001-screen.png\"\n",
                manifest_path.display(),
                fixture_dir.join("layers/000-background.png").display(),
                fixture_dir.join("layers/001-screen.png").display(),
            ),
        );

        let mut controller = PhotoTuxController::new_with_working_directory_and_psd_sidecar(
            working_directory.clone(),
            Some(PsdImportSidecar::new(script_path.clone())),
        );
        controller.import_image(source_path.clone());
        wait_for_background_jobs(&mut controller);

        let expected = build_supported_psd_import_document();
        assert_eq!(
            flatten_document_rgba(&controller.document),
            flatten_document_rgba(&expected)
        );
        assert!(controller.document_path.is_none());
        assert!(controller.dirty_since_primary_save);
        assert!(controller.dirty_since_autosave);
        assert!(controller.status_message.contains("Imported"));
        assert_eq!(
            controller.snapshot().history_entries,
            vec!["Import PSD".to_string()]
        );
        assert!(controller.snapshot().latest_import_report.is_none());

        fs::remove_dir_all(&working_directory).expect("temporary PSD directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn psd_import_surfaces_flattened_fallback_report() {
        let working_directory =
            std::env::temp_dir().join(format!("phototux-psd-fallback-{}", std::process::id()));
        fs::create_dir_all(&working_directory)
            .expect("temporary PSD fallback directory should exist");
        let fixture_dir = working_directory.join("fixture");
        fs::create_dir_all(&fixture_dir).expect("fixture dir should exist");

        let manifest_path = fixture_dir.join("fixture-manifest.json");
        fs::write(&manifest_path, flattened_fallback_psd_manifest_json())
            .expect("PSD fallback manifest should be written");
        write_rgba_document_png(
            &fixture_dir.join("composite.png"),
            6,
            6,
            &[
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [30, 40, 60, 255],
                [30, 40, 60, 255],
                [30, 40, 60, 255],
                [30, 40, 60, 255],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [30, 40, 60, 255],
                [50, 80, 120, 255],
                [50, 80, 120, 255],
                [30, 40, 60, 255],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [30, 40, 60, 255],
                [50, 80, 120, 255],
                [50, 80, 120, 255],
                [30, 40, 60, 255],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [30, 40, 60, 255],
                [30, 40, 60, 255],
                [30, 40, 60, 255],
                [30, 40, 60, 255],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ],
        );

        let source_path = working_directory.join("fallback.psd");
        fs::write(&source_path, b"placeholder psd source").expect("PSD source should be written");
        let script_path = working_directory.join("psd-sidecar-fallback.sh");
        write_shell_script(
            &script_path,
            &format!(
                "#!/bin/sh\nset -eu\nSOURCE=\"$1\"\nWORKSPACE=\"$2\"\nMANIFEST=\"$3\"\n[ -f \"$SOURCE\" ]\ncp \"{}\" \"$MANIFEST\"\ncp \"{}\" \"$WORKSPACE/composite.png\"\n",
                manifest_path.display(),
                fixture_dir.join("composite.png").display(),
            ),
        );

        let mut controller = PhotoTuxController::new_with_working_directory_and_psd_sidecar(
            working_directory.clone(),
            Some(PsdImportSidecar::new(script_path.clone())),
        );
        controller.import_image(source_path.clone());
        wait_for_background_jobs(&mut controller);

        let snapshot = controller.snapshot();
        assert_eq!(controller.document.layer_count(), 1);
        assert!(
            snapshot
                .status_message
                .contains("flattened PSD fallback used")
        );
        let report = snapshot
            .latest_import_report
            .expect("flattened fallback should surface an import report");
        assert!(report.title.contains("Flattened Composite"));
        assert!(report.summary.contains("flattened composite"));
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("unsupported kind"))
        );

        fs::remove_dir_all(&working_directory)
            .expect("temporary PSD fallback directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn psd_repo_fixture_supported_layers_import_updates_controller_state() {
        if !repo_psd_sidecar_runtime_available() {
            eprintln!(
                "skipping app_core PSD repo fixture import test: python3 with psd_tools is unavailable"
            );
            return;
        }

        let working_directory = unique_temp_dir("psd-repo-import");
        fs::create_dir_all(&working_directory)
            .expect("temporary PSD repo import directory should exist");

        let mut controller = PhotoTuxController::new_with_working_directory_and_psd_sidecar(
            working_directory.clone(),
            Some(
                PsdImportSidecar::new("python3")
                    .with_arg(repo_psd_sidecar_script_path().as_os_str()),
            ),
        );
        controller.import_image(repo_psd_fixture_path("supported-simple-layers.psd"));
        wait_for_background_jobs(&mut controller);

        let expected = build_supported_psd_import_document();
        let snapshot = controller.snapshot();
        assert_eq!(
            flatten_document_rgba(&controller.document),
            flatten_document_rgba(&expected)
        );
        assert!(controller.document_path.is_none());
        assert!(controller.dirty_since_primary_save);
        assert!(controller.dirty_since_autosave);
        assert!(snapshot.status_message.contains("Imported"));
        assert_eq!(snapshot.history_entries, vec!["Import PSD".to_string()]);
        assert!(snapshot.latest_import_report.is_none());

        fs::remove_dir_all(&working_directory)
            .expect("temporary PSD repo import directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn psd_repo_fixture_cmyk_import_surfaces_controller_report() {
        if !repo_psd_sidecar_runtime_available() {
            eprintln!(
                "skipping app_core PSD repo fixture CMYK test: python3 with psd_tools is unavailable"
            );
            return;
        }

        let working_directory = unique_temp_dir("psd-repo-cmyk");
        fs::create_dir_all(&working_directory)
            .expect("temporary PSD repo CMYK directory should exist");

        let mut controller = PhotoTuxController::new_with_working_directory_and_psd_sidecar(
            working_directory.clone(),
            Some(
                PsdImportSidecar::new("python3")
                    .with_arg(repo_psd_sidecar_script_path().as_os_str()),
            ),
        );
        controller.import_image(repo_psd_fixture_path("unsupported-cmyk-fallback.psd"));
        wait_for_background_jobs(&mut controller);

        let snapshot = controller.snapshot();
        assert_eq!(controller.document.layer_count(), 1);
        assert!(
            snapshot
                .status_message
                .contains("flattened PSD fallback used")
        );
        assert_eq!(snapshot.history_entries, vec!["Import PSD".to_string()]);
        let report = snapshot
            .latest_import_report
            .expect("CMYK fallback should surface an import report");
        assert!(report.title.contains("Flattened Composite"));
        assert!(report.summary.contains("flattened composite"));
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "unsupported_color_mode")
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "flattened_fallback_used")
        );

        fs::remove_dir_all(&working_directory)
            .expect("temporary PSD repo CMYK directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn psd_repo_fixture_export_roundtrip_matches_imported_document() {
        if !repo_psd_sidecar_runtime_available() {
            eprintln!(
                "skipping app_core PSD repo export parity test: python3 with psd_tools is unavailable"
            );
            return;
        }

        let working_directory = unique_temp_dir("psd-repo-export");
        fs::create_dir_all(&working_directory)
            .expect("temporary PSD repo export directory should exist");
        let export_path = working_directory.join("imported-scene.png");

        let mut controller = PhotoTuxController::new_with_working_directory_and_psd_sidecar(
            working_directory.clone(),
            Some(
                PsdImportSidecar::new("python3")
                    .with_arg(repo_psd_sidecar_script_path().as_os_str()),
            ),
        );
        controller.import_image(repo_psd_fixture_path("supported-simple-layers.psd"));
        wait_for_background_jobs(&mut controller);

        let imported_pixels = flatten_document_rgba(&controller.document);
        controller.export_document(export_path.clone());
        wait_for_background_jobs(&mut controller);

        let mut import_controller = PhotoTuxController::new();
        import_controller.import_image(export_path.clone());
        wait_for_background_jobs(&mut import_controller);

        assert!(export_path.exists());
        assert!(controller.status_message.contains("Exported"));
        assert_eq!(
            flatten_document_rgba(&import_controller.document),
            imported_pixels
        );

        fs::remove_dir_all(&working_directory)
            .expect("temporary PSD repo export directory should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn psd_repo_fixture_canvas_raster_matches_flattened_document() {
        if !repo_psd_sidecar_runtime_available() {
            eprintln!(
                "skipping app_core PSD repo viewport parity test: python3 with psd_tools is unavailable"
            );
            return;
        }

        let working_directory = unique_temp_dir("psd-repo-viewport");
        fs::create_dir_all(&working_directory)
            .expect("temporary PSD repo viewport directory should exist");

        let mut controller = PhotoTuxController::new_with_working_directory_and_psd_sidecar(
            working_directory.clone(),
            Some(
                PsdImportSidecar::new("python3")
                    .with_arg(repo_psd_sidecar_script_path().as_os_str()),
            ),
        );
        controller.import_image(repo_psd_fixture_path("supported-simple-layers.psd"));
        wait_for_background_jobs(&mut controller);

        let viewport_pixels = controller.canvas_raster().pixels;
        let flattened_pixels = flatten_document_rgba(&controller.document);
        assert_eq!(viewport_pixels, flattened_pixels);

        fs::remove_dir_all(&working_directory)
            .expect("temporary PSD repo viewport directory should be removed");
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
        assert!(
            snapshot
                .history_entries
                .iter()
                .any(|entry| entry.contains("Move Layer"))
        );
    }

    #[test]
    fn marquee_interaction_sets_selection_rect() {
        let mut controller = PhotoTuxController::new();
        controller.select_tool(ShellToolKind::RectangularMarquee);
        controller.begin_canvas_interaction(10, 20);
        controller.update_canvas_interaction(50, 70);
        controller.end_canvas_interaction();

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.selection_rect,
            Some(CanvasRect::new(10, 20, 40, 50))
        );
    }

    #[test]
    fn lasso_interaction_sets_freeform_selection_path() {
        let mut controller = PhotoTuxController::new();
        controller.select_tool(ShellToolKind::Lasso);
        controller.begin_canvas_interaction(10, 20);
        controller.update_canvas_interaction(40, 20);
        controller.update_canvas_interaction(25, 50);
        controller.end_canvas_interaction();

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.selection_rect,
            Some(CanvasRect::new(10, 20, 31, 31))
        );
        assert_eq!(
            snapshot.selection_path,
            Some(vec![(10, 20), (40, 20), (25, 50)])
        );
        assert!(snapshot.selection_preview_path.is_none());
        assert!(controller.document.selection_contains_pixel(25, 30));
    }

    #[test]
    fn guide_commands_update_snapshot_and_roundtrip_history() {
        let mut controller = PhotoTuxController::new();

        controller.add_horizontal_guide();
        controller.add_vertical_guide();
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.guide_count, 2);
        assert!(snapshot.guides_visible);
        assert!(
            snapshot
                .guides
                .iter()
                .any(|guide| matches!(guide, ShellGuide::Horizontal { .. }))
        );
        assert!(
            snapshot
                .guides
                .iter()
                .any(|guide| matches!(guide, ShellGuide::Vertical { .. }))
        );

        controller.toggle_guides_visible();
        assert!(!controller.snapshot().guides_visible);

        controller.undo();
        assert!(controller.snapshot().guides_visible);

        controller.remove_last_guide();
        assert_eq!(controller.snapshot().guide_count, 1);
        controller.undo();
        assert_eq!(controller.snapshot().guide_count, 2);
        controller.redo();
        assert_eq!(controller.snapshot().guide_count, 1);
    }

    #[test]
    fn pressure_mapping_toggles_update_snapshot() {
        let mut controller = PhotoTuxController::new();

        assert!(!controller.snapshot().pressure_size_enabled);
        assert!(!controller.snapshot().pressure_opacity_enabled);

        controller.toggle_pressure_size_enabled();
        controller.toggle_pressure_opacity_enabled();

        let snapshot = controller.snapshot();
        assert!(snapshot.pressure_size_enabled);
        assert!(snapshot.pressure_opacity_enabled);
    }

    #[test]
    fn brush_parameter_controls_update_snapshot() {
        let mut controller = PhotoTuxController::new();

        let before = controller.snapshot();
        controller.increase_brush_radius();
        controller.decrease_brush_hardness();
        controller.increase_brush_spacing();
        controller.decrease_brush_flow();

        let after = controller.snapshot();
        assert!(after.brush_radius > before.brush_radius);
        assert!(after.brush_hardness_percent < before.brush_hardness_percent);
        assert!(after.brush_spacing > before.brush_spacing);
        assert!(after.brush_flow_percent < before.brush_flow_percent);
        assert_eq!(after.brush_preset_name, "Custom");
    }

    #[test]
    fn brush_presets_switch_and_custom_state_is_tracked() {
        let mut controller = PhotoTuxController::new();

        assert_eq!(controller.snapshot().brush_preset_name, "Balanced Round");

        controller.next_brush_preset();
        let soft = controller.snapshot();
        assert_eq!(soft.brush_preset_name, "Soft Shade");
        assert_eq!(soft.brush_radius, 24);
        assert_eq!(soft.brush_hardness_percent, 35);
        assert_eq!(soft.brush_spacing, 12);
        assert_eq!(soft.brush_flow_percent, 40);
        assert!(soft.pressure_size_enabled);
        assert!(soft.pressure_opacity_enabled);

        controller.previous_brush_preset();
        assert_eq!(controller.snapshot().brush_preset_name, "Balanced Round");

        controller.increase_brush_radius();
        assert_eq!(controller.snapshot().brush_preset_name, "Custom");

        controller.previous_brush_preset();
        let ink = controller.snapshot();
        assert_eq!(ink.brush_preset_name, "Ink Taper");
        assert_eq!(ink.brush_radius, 7);
        assert_eq!(ink.brush_hardness_percent, 100);
        assert_eq!(ink.brush_spacing, 3);
        assert_eq!(ink.brush_flow_percent, 28);
        assert!(ink.pressure_size_enabled);
        assert!(!ink.pressure_opacity_enabled);
    }

    #[test]
    fn pressure_enabled_repeated_strokes_remain_stable_on_medium_canvas() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_medium_paint_fixture_document();
        controller.reset_selected_structure_target_to_active_layer();
        controller.select_tool(ShellToolKind::Brush);
        controller.toggle_pressure_size_enabled();
        controller.toggle_pressure_opacity_enabled();
        controller.increase_brush_radius();
        controller.increase_brush_spacing();

        let baseline = controller.canvas_raster();
        let stroke_segments = [
            ((96, 96), (184, 140), (0.22, 0.94)),
            ((212, 118), (320, 152), (0.35, 1.0)),
            ((356, 210), (448, 240), (0.28, 0.86)),
            ((492, 196), (612, 248), (0.42, 0.98)),
            ((128, 332), (244, 364), (0.30, 0.88)),
            ((268, 352), (392, 396), (0.25, 0.92)),
            ((428, 318), (544, 372), (0.46, 1.0)),
            ((588, 344), (708, 388), (0.32, 0.9)),
            ((176, 508), (304, 544), (0.24, 0.84)),
            ((336, 520), (464, 560), (0.4, 0.96)),
            ((508, 474), (628, 526), (0.27, 0.9)),
            ((664, 512), (796, 564), (0.38, 1.0)),
        ];

        for ((start_x, start_y), (end_x, end_y), (start_pressure, end_pressure)) in stroke_segments
        {
            ui_shell::ShellController::begin_canvas_interaction_with_pressure(
                &mut controller,
                start_x,
                start_y,
                start_pressure,
            );
            ui_shell::ShellController::update_canvas_interaction_with_pressure(
                &mut controller,
                end_x,
                end_y,
                end_pressure,
            );
            controller.end_canvas_interaction();
        }

        let painted = controller.canvas_raster();
        assert_ne!(painted.pixels, baseline.pixels);
        assert_eq!(painted.pixels, flatten_document_rgba(&controller.document));
        assert!(!controller.document.active_layer().tiles.is_empty());
        assert!(controller.document.active_layer().tiles.len() < 64);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot
                .history_entries
                .iter()
                .filter(|entry| entry.contains("Brush Stroke"))
                .count(),
            12
        );
        assert!(snapshot.can_undo);

        for _ in 0..12 {
            controller.undo();
        }
        assert_eq!(controller.canvas_raster().pixels, baseline.pixels);

        for _ in 0..12 {
            controller.redo();
        }
        assert_eq!(controller.canvas_raster().pixels, painted.pixels);
    }

    #[test]
    fn destructive_filter_runs_through_background_job_and_history() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_representative_controller_document();
        let before = controller.canvas_raster();

        controller.apply_destructive_filter(DestructiveFilterKind::InvertColors);
        assert!(controller.snapshot().filter_job_active);
        wait_for_background_jobs(&mut controller);

        let after = controller.canvas_raster();
        assert_ne!(after.pixels, before.pixels);
        assert_eq!(after.pixels, flatten_document_rgba(&controller.document));
        assert!(
            controller
                .snapshot()
                .history_entries
                .iter()
                .any(|entry| entry.contains("Filter Invert Colors"))
        );

        controller.undo();
        assert_eq!(controller.canvas_raster().pixels, before.pixels);

        controller.redo();
        assert_eq!(controller.canvas_raster().pixels, after.pixels);
    }

    #[test]
    fn destructive_filter_result_is_discarded_if_document_changes_mid_job() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_representative_controller_document();
        let before = controller.canvas_raster();

        controller.apply_destructive_filter(DestructiveFilterKind::Desaturate);
        controller.add_layer();
        wait_for_background_jobs(&mut controller);

        assert_eq!(controller.canvas_raster().pixels, before.pixels);
        assert!(controller.status_message.contains("discarded"));
    }

    #[test]
    fn destructive_filter_rejects_mask_edit_target() {
        let mut controller = PhotoTuxController::new();
        controller.add_active_layer_mask();
        controller.edit_active_layer_mask();

        controller.apply_destructive_filter(DestructiveFilterKind::InvertColors);

        assert!(controller.pending_filter_job.is_none());
        assert_eq!(
            controller.status_message,
            "Destructive filters currently apply to layer pixels only"
        );
    }

    #[test]
    fn move_interaction_snaps_to_guides_when_enabled() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_guided_snapping_fixture_document();

        controller.select_tool(ShellToolKind::Move);
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(30, 15);
        controller.end_canvas_interaction();

        assert_eq!(
            controller.snapshot().active_layer_bounds,
            Some(CanvasRect::new(32, 16, 256, 256))
        );
    }

    #[test]
    fn transform_preview_snaps_to_guides_when_enabled() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_guided_snapping_fixture_document();

        controller.select_tool(ShellToolKind::Transform);
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(30, 15);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot.transform_preview_rect,
            Some(CanvasRect::new(32, 16, 256, 256))
        );
    }

    #[test]
    fn move_interaction_can_disable_or_bypass_snapping() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_guided_snapping_fixture_document();

        controller.toggle_snapping_enabled();
        assert!(!controller.snapshot().snapping_enabled);
        controller.select_tool(ShellToolKind::Move);
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(30, 15);
        controller.end_canvas_interaction();
        assert_eq!(
            controller.snapshot().active_layer_bounds,
            Some(CanvasRect::new(30, 15, 256, 256))
        );

        controller.undo();
        controller.toggle_snapping_enabled();
        controller.set_temporary_snap_bypass(true);
        controller.begin_canvas_interaction(0, 0);
        controller.update_canvas_interaction(30, 15);
        assert!(controller.snapshot().snapping_temporarily_bypassed);
        controller.end_canvas_interaction();
        controller.set_temporary_snap_bypass(false);

        assert_eq!(
            controller.snapshot().active_layer_bounds,
            Some(CanvasRect::new(30, 15, 256, 256))
        );
    }

    #[test]
    fn lasso_transform_fixture_commit_matches_flattened_document() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_lasso_transform_fixture_document();

        controller.select_tool(ShellToolKind::Transform);
        controller.begin_transform();
        controller.scale_transform_x_up();
        controller.rotate_transform_right();
        controller.begin_canvas_interaction(20, 24);
        controller.update_canvas_interaction(38, 30);
        controller.end_canvas_interaction();
        controller.commit_transform();

        let raster = controller.canvas_raster();
        let flattened = flatten_document_rgba(&controller.document);
        assert_eq!(raster.pixels, flattened);
        assert!(controller.document.selection_shape().is_some());
    }

    #[test]
    fn move_interaction_moves_only_selected_pixels() {
        let mut controller = PhotoTuxController::new();
        controller.document = doc_model::Document::new(128, 128);
        set_pixel(&mut controller.document, 0, 20, 20, [255, 255, 255, 255]);
        set_pixel(&mut controller.document, 0, 60, 20, [255, 255, 255, 255]);
        controller
            .document
            .set_selection(CanvasRect::new(16, 16, 16, 16));

        controller.select_tool(ShellToolKind::Move);
        controller.begin_canvas_interaction(20, 20);
        controller.update_canvas_interaction(30, 20);
        controller.end_canvas_interaction();

        let raster = controller.canvas_raster();
        assert_eq!(flattened_pixel(&raster, 20, 20)[3], 0);
        assert!(flattened_pixel(&raster, 30, 20)[3] > 0);
        assert!(flattened_pixel(&raster, 60, 20)[3] > 0);
        assert!(
            controller
                .snapshot()
                .history_entries
                .iter()
                .any(|entry| entry.contains("Move Selection"))
        );
    }

    #[test]
    fn transform_interaction_transforms_only_selected_pixels() {
        let mut controller = PhotoTuxController::new();
        controller.document = doc_model::Document::new(128, 128);
        set_pixel(&mut controller.document, 0, 20, 20, [255, 255, 255, 255]);
        set_pixel(&mut controller.document, 0, 60, 20, [255, 255, 255, 255]);
        controller
            .document
            .set_selection(CanvasRect::new(16, 16, 16, 16));

        controller.select_tool(ShellToolKind::Transform);
        controller.begin_transform();
        controller.begin_canvas_interaction(20, 20);
        controller.update_canvas_interaction(30, 20);
        controller.end_canvas_interaction();
        controller.commit_transform();

        let raster = controller.canvas_raster();
        assert_eq!(flattened_pixel(&raster, 20, 20)[3], 0);
        assert!(flattened_pixel(&raster, 30, 20)[3] > 0);
        assert!(flattened_pixel(&raster, 60, 20)[3] > 0);
        assert!(
            controller
                .snapshot()
                .history_entries
                .iter()
                .any(|entry| entry.contains("Transform Layer"))
        );
    }

    #[test]
    fn transform_controls_update_snapshot_for_non_uniform_scale_and_rotation() {
        let mut controller = PhotoTuxController::new();
        controller.document = doc_model::Document::new(128, 128);
        set_pixel(&mut controller.document, 0, 20, 20, [255, 255, 255, 255]);

        controller.begin_transform();
        controller.scale_transform_x_up();
        controller.scale_transform_y_down();
        controller.rotate_transform_right();

        let snapshot = controller.snapshot();
        assert!(snapshot.transform_active);
        assert_eq!(snapshot.transform_scale_x_percent, 110);
        assert_eq!(snapshot.transform_scale_y_percent, 90);
        assert_eq!(snapshot.transform_rotation_degrees, 90);
        assert_eq!(
            snapshot.transform_preview_rect,
            Some(CanvasRect::new(0, 0, 230, 282))
        );
    }

    #[test]
    fn transform_commit_applies_rotation_and_non_uniform_scale() {
        let mut controller = PhotoTuxController::new();
        controller.document = doc_model::Document::new(128, 128);
        set_pixel(&mut controller.document, 0, 0, 0, [255, 255, 255, 255]);
        let before = controller.canvas_raster();

        controller.begin_transform();
        controller.scale_transform_x_up();
        controller.scale_transform_y_down();
        controller.rotate_transform_right();
        controller.commit_transform();

        let raster = controller.canvas_raster();
        assert_ne!(before.pixels, raster.pixels);
        assert!(
            controller
                .snapshot()
                .history_entries
                .iter()
                .any(|entry| entry.contains("Transform Layer"))
        );
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
        assert!(
            controller
                .snapshot()
                .history_entries
                .iter()
                .any(|entry| entry.contains("Brush Stroke"))
        );
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
        assert!(
            with_mask
                .history_entries
                .iter()
                .any(|entry| entry.contains("Add Layer Mask"))
        );

        controller.toggle_active_layer_mask_enabled();
        assert!(!controller.snapshot().active_layer_mask_enabled);

        controller.edit_active_layer_pixels();
        assert_eq!(
            controller.snapshot().active_edit_target_name,
            "Layer Pixels"
        );

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
        assert!(
            controller
                .snapshot()
                .history_entries
                .iter()
                .any(|entry| entry.contains("Mask Hide Stroke"))
        );

        controller.undo();
        assert_eq!(
            flattened_pixel(&controller.canvas_raster(), sample_x, sample_y),
            before_pixel
        );

        controller.redo();
        assert_eq!(
            flattened_pixel(&controller.canvas_raster(), sample_x, sample_y),
            hidden_pixel
        );
    }

    #[test]
    fn group_commands_update_snapshot_and_undo_redo() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_representative_controller_document();
        controller.reset_selected_structure_target_to_active_layer();

        controller.create_group_from_active_layer();
        let group_id = first_group_id(&controller.document);
        let grouped_snapshot = controller.snapshot();
        assert!(grouped_snapshot.selected_structure_is_group);
        assert!(grouped_snapshot.can_ungroup_selected_group);
        assert!(
            grouped_snapshot
                .layers
                .iter()
                .any(|item| item.is_group && item.group_id == Some(group_id))
        );

        controller.undo();
        assert_eq!(controller.document.group_count(), 0);

        controller.redo();
        assert_eq!(controller.document.group_count(), 1);
        assert!(matches!(
            controller.selected_structure_target,
            doc_model::LayerHierarchyNodeRef::Group(_)
        ));
    }

    #[test]
    fn move_layer_into_and_out_of_group_roundtrips_through_history() {
        let mut controller = PhotoTuxController::new();
        controller.document = build_representative_controller_document();
        controller.reset_selected_structure_target_to_active_layer();

        let top_layer_id = controller.document.active_layer().id;
        controller.create_group_from_active_layer();
        let group_id = first_group_id(&controller.document);

        controller.select_layer(1);
        let moved_layer_id = controller.document.active_layer().id;
        controller.select_group(group_id);
        controller.move_active_layer_into_selected_group();
        assert_eq!(
            controller.document.group_for_layer(moved_layer_id),
            Some(group_id)
        );

        let nested_snapshot = controller.snapshot();
        let moved_row = nested_snapshot
            .layers
            .iter()
            .find(|item| item.index == Some(1))
            .expect("expected moved layer row in snapshot");
        assert_eq!(moved_row.depth, 1);

        controller.undo();
        assert_eq!(controller.document.group_for_layer(moved_layer_id), None);

        controller.redo();
        assert_eq!(
            controller.document.group_for_layer(moved_layer_id),
            Some(group_id)
        );

        controller.move_active_layer_out_of_group();
        assert_eq!(controller.document.group_for_layer(moved_layer_id), None);

        controller.undo();
        assert_eq!(
            controller.document.group_for_layer(moved_layer_id),
            Some(group_id)
        );
        assert_eq!(controller.document.active_layer().id, moved_layer_id);
        assert_eq!(
            top_layer_id,
            controller
                .document
                .layer(2)
                .expect("top layer should still exist")
                .id
        );
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
        assert_eq!(
            controller.snapshot().selection_rect,
            Some(CanvasRect::new(10, 10, 20, 30))
        );

        controller.undo();
        assert_eq!(controller.snapshot().selection_rect, None);
        controller.redo();
        assert_eq!(
            controller.snapshot().selection_rect,
            Some(CanvasRect::new(10, 10, 20, 30))
        );
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
        assert!(
            controller
                .snapshot()
                .history_entries
                .iter()
                .any(|entry| entry.contains("Transform Layer"))
        );
    }
}
