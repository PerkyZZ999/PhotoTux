//! Undo and redo scaffolding for PhotoTux.

use common::LayerId;
use doc_model::{LayeredRasterDocument, RasterLayer, RasterSurface};

const DEFAULT_HISTORY_BUDGET_BYTES: usize = 32 * 1024 * 1024;

/// A single pixel mutation captured for undo and redo.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelChange {
    /// Pixel x coordinate.
    pub x: u32,
    /// Pixel y coordinate.
    pub y: u32,
    /// Pixel value before the edit.
    pub before: [u8; 4],
    /// Pixel value after the edit.
    pub after: [u8; 4],
}

impl PixelChange {
    /// Create a pixel change record.
    #[must_use]
    pub const fn new(x: u32, y: u32, before: [u8; 4], after: [u8; 4]) -> Self {
        Self {
            x,
            y,
            before,
            after,
        }
    }
}

/// A grouped brush stroke history entry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StrokeHistoryEntry {
    changes: Vec<PixelChange>,
    target: RasterEditTarget,
}

/// Target surface for a raster edit history entry.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RasterEditTarget {
    /// Apply the edit to a standalone raster surface.
    #[default]
    StandaloneSurface,
    /// Apply the edit to a specific layer surface in a layered document.
    Layer(LayerId),
}

impl StrokeHistoryEntry {
    /// Create a grouped stroke history entry.
    #[must_use]
    pub fn new(changes: Vec<PixelChange>) -> Self {
        Self {
            changes,
            target: RasterEditTarget::StandaloneSurface,
        }
    }

    /// Create a grouped stroke history entry targeting a specific layer surface.
    #[must_use]
    pub fn for_layer(layer_id: LayerId, changes: Vec<PixelChange>) -> Self {
        Self {
            changes,
            target: RasterEditTarget::Layer(layer_id),
        }
    }

    /// Return the captured pixel changes.
    #[must_use]
    pub fn changes(&self) -> &[PixelChange] {
        &self.changes
    }

    /// Return whether the entry carries any changes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Return the target raster edit surface.
    #[must_use]
    pub const fn target(&self) -> RasterEditTarget {
        self.target
    }

    fn estimated_bytes(&self) -> usize {
        self.changes.len() * std::mem::size_of::<PixelChange>()
    }

    fn apply_undo(&self, surface: &mut RasterSurface) {
        for change in &self.changes {
            let _ = surface.write_pixel(change.x, change.y, change.before);
        }
    }

    fn apply_redo(&self, surface: &mut RasterSurface) {
        for change in &self.changes {
            let _ = surface.write_pixel(change.x, change.y, change.after);
        }
    }
}

/// Undoable structural document operations.
#[derive(Clone, Debug, PartialEq)]
pub enum StructuralHistoryEntry {
    /// Create a layer and its surface at a specific stack index.
    CreateLayer {
        /// Layer metadata to recreate.
        layer: RasterLayer,
        /// Raster surface to recreate.
        surface: RasterSurface,
        /// Stack index where the layer should be inserted.
        index: usize,
    },
    /// Delete a layer and its surface from a specific stack index.
    DeleteLayer {
        /// Layer metadata to restore on undo.
        layer: RasterLayer,
        /// Raster surface to restore on undo.
        surface: RasterSurface,
        /// Original stack index before deletion.
        index: usize,
    },
    /// Rename a layer.
    RenameLayer {
        /// Layer to rename.
        layer_id: LayerId,
        /// Previous display name.
        before_name: String,
        /// New display name.
        after_name: String,
    },
    /// Reorder a layer.
    ReorderLayer {
        /// Layer to reorder.
        layer_id: LayerId,
        /// Previous stack index.
        before_index: usize,
        /// New stack index.
        after_index: usize,
    },
    /// Change the active layer selection.
    SetActiveLayer {
        /// Previously active layer.
        before_layer_id: Option<LayerId>,
        /// Newly active layer.
        after_layer_id: Option<LayerId>,
    },
    /// Change layer visibility.
    SetLayerVisibility {
        /// Layer to update.
        layer_id: LayerId,
        /// Previous visibility state.
        before_visible: bool,
        /// New visibility state.
        after_visible: bool,
    },
    /// Change layer opacity.
    SetLayerOpacity {
        /// Layer to update.
        layer_id: LayerId,
        /// Previous opacity value.
        before_opacity: f32,
        /// New opacity value.
        after_opacity: f32,
    },
}

impl StructuralHistoryEntry {
    fn estimated_bytes(&self) -> usize {
        match self {
            Self::CreateLayer { layer, surface, .. } | Self::DeleteLayer { layer, surface, .. } => {
                std::mem::size_of::<RasterLayer>()
                    + layer.name.len()
                    + ((surface.width() as usize) * (surface.height() as usize) * 4)
            }
            Self::RenameLayer {
                before_name,
                after_name,
                ..
            } => std::mem::size_of::<Self>() + before_name.len() + after_name.len(),
            _ => std::mem::size_of::<Self>(),
        }
    }

    fn apply_undo(&self, document: &mut LayeredRasterDocument) -> bool {
        match self {
            Self::CreateLayer { layer, .. } => document.delete_layer(layer.id).is_some(),
            Self::DeleteLayer {
                layer,
                surface,
                index,
            } => document.insert_layer_with_surface(*index, layer.clone(), surface.clone()),
            Self::RenameLayer {
                layer_id,
                before_name,
                ..
            } => document.rename_layer(*layer_id, before_name.clone()),
            Self::ReorderLayer {
                layer_id,
                before_index,
                ..
            } => document.reorder_layer(*layer_id, *before_index),
            Self::SetActiveLayer {
                before_layer_id,
                after_layer_id: _,
            } => before_layer_id.is_none_or(|layer_id| document.set_active_layer(layer_id)),
            Self::SetLayerVisibility {
                layer_id,
                before_visible,
                ..
            } => document.set_layer_visibility(*layer_id, *before_visible),
            Self::SetLayerOpacity {
                layer_id,
                before_opacity,
                ..
            } => document.set_layer_opacity(*layer_id, *before_opacity),
        }
    }

    fn apply_redo(&self, document: &mut LayeredRasterDocument) -> bool {
        match self {
            Self::CreateLayer {
                layer,
                surface,
                index,
            } => document.insert_layer_with_surface(*index, layer.clone(), surface.clone()),
            Self::DeleteLayer { layer, .. } => document.delete_layer(layer.id).is_some(),
            Self::RenameLayer {
                layer_id,
                after_name,
                ..
            } => document.rename_layer(*layer_id, after_name.clone()),
            Self::ReorderLayer {
                layer_id,
                after_index,
                ..
            } => document.reorder_layer(*layer_id, *after_index),
            Self::SetActiveLayer {
                before_layer_id: _,
                after_layer_id,
            } => after_layer_id.is_none_or(|layer_id| document.set_active_layer(layer_id)),
            Self::SetLayerVisibility {
                layer_id,
                after_visible,
                ..
            } => document.set_layer_visibility(*layer_id, *after_visible),
            Self::SetLayerOpacity {
                layer_id,
                after_opacity,
                ..
            } => document.set_layer_opacity(*layer_id, *after_opacity),
        }
    }
}

/// A generic undo history entry.
#[derive(Clone, Debug, PartialEq)]
pub enum HistoryEntry {
    /// Raster pixel delta entry.
    RasterStroke(StrokeHistoryEntry),
    /// Structural document command entry.
    Structural(StructuralHistoryEntry),
}

impl HistoryEntry {
    fn estimated_bytes(&self) -> usize {
        match self {
            Self::RasterStroke(entry) => entry.estimated_bytes(),
            Self::Structural(entry) => entry.estimated_bytes(),
        }
    }
}

/// Mutable targets available while applying history entries.
pub struct HistoryContext<'a> {
    raster_surface: Option<&'a mut RasterSurface>,
    layered_document: Option<&'a mut LayeredRasterDocument>,
}

impl<'a> HistoryContext<'a> {
    /// Create a history context for a standalone raster surface.
    #[must_use]
    pub fn for_raster(surface: &'a mut RasterSurface) -> Self {
        Self {
            raster_surface: Some(surface),
            layered_document: None,
        }
    }

    /// Create a history context for a layered document.
    #[must_use]
    pub fn for_layered(document: &'a mut LayeredRasterDocument) -> Self {
        Self {
            raster_surface: None,
            layered_document: Some(document),
        }
    }

    fn raster_target(&mut self, target: RasterEditTarget) -> Option<&mut RasterSurface> {
        match target {
            RasterEditTarget::StandaloneSurface => self.raster_surface.as_deref_mut(),
            RasterEditTarget::Layer(layer_id) => {
                self.layered_document.as_deref_mut()?.surface_mut(layer_id)
            }
        }
    }

    fn layered_document(&mut self) -> Option<&mut LayeredRasterDocument> {
        self.layered_document.as_deref_mut()
    }
}

/// Undo and redo stack for grouped history entries.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistoryStack {
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    memory_budget_bytes: usize,
    retained_bytes: usize,
}

impl HistoryStack {
    /// Create an empty history stack.
    #[must_use]
    pub fn new() -> Self {
        Self::with_memory_budget(DEFAULT_HISTORY_BUDGET_BYTES)
    }

    /// Create a history stack with an explicit memory budget.
    #[must_use]
    pub fn with_memory_budget(memory_budget_bytes: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            memory_budget_bytes: memory_budget_bytes.max(1),
            retained_bytes: 0,
        }
    }

    /// Push a completed history entry and clear redo history.
    pub fn push_entry(&mut self, entry: HistoryEntry) {
        if matches!(&entry, HistoryEntry::RasterStroke(stroke) if stroke.is_empty()) {
            return;
        }

        self.retained_bytes = self.retained_bytes.saturating_sub(
            self.redo_stack
                .iter()
                .map(HistoryEntry::estimated_bytes)
                .sum::<usize>(),
        );
        self.redo_stack.clear();
        self.retained_bytes += entry.estimated_bytes();
        self.undo_stack.push(entry);
        self.enforce_memory_budget();
    }

    /// Push a completed brush stroke and clear redo history.
    pub fn push_brush_stroke(&mut self, entry: StrokeHistoryEntry) {
        self.push_entry(HistoryEntry::RasterStroke(entry));
    }

    /// Push a structural document edit and clear redo history.
    pub fn push_structural_edit(&mut self, entry: StructuralHistoryEntry) {
        self.push_entry(HistoryEntry::Structural(entry));
    }

    /// Return whether an undo operation is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Return whether a redo operation is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Return the number of retained undo entries.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    /// Return the number of retained redo entries.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }

    /// Return the current estimated memory retained by history.
    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    /// Undo the latest stroke.
    pub fn undo(&mut self, surface: &mut RasterSurface) -> bool {
        let mut context = HistoryContext::for_raster(surface);
        self.undo_with_context(&mut context)
    }

    /// Undo the latest edit against a layered document.
    pub fn undo_layered(&mut self, document: &mut LayeredRasterDocument) -> bool {
        let mut context = HistoryContext::for_layered(document);
        self.undo_with_context(&mut context)
    }

    /// Redo the latest undone stroke.
    pub fn redo(&mut self, surface: &mut RasterSurface) -> bool {
        let mut context = HistoryContext::for_raster(surface);
        self.redo_with_context(&mut context)
    }

    /// Redo the latest undone edit against a layered document.
    pub fn redo_layered(&mut self, document: &mut LayeredRasterDocument) -> bool {
        let mut context = HistoryContext::for_layered(document);
        self.redo_with_context(&mut context)
    }

    /// Undo the latest edit against an explicit history context.
    pub fn undo_with_context(&mut self, context: &mut HistoryContext<'_>) -> bool {
        let Some(entry) = self.undo_stack.pop() else {
            return false;
        };

        let applied = match &entry {
            HistoryEntry::RasterStroke(stroke) => context
                .raster_target(stroke.target())
                .map(|surface| {
                    stroke.apply_undo(surface);
                    true
                })
                .unwrap_or(false),
            HistoryEntry::Structural(structural) => context
                .layered_document()
                .map(|document| structural.apply_undo(document))
                .unwrap_or(false),
        };

        if !applied {
            self.undo_stack.push(entry);
            return false;
        }

        self.redo_stack.push(entry);
        true
    }

    /// Redo the latest edit against an explicit history context.
    pub fn redo_with_context(&mut self, context: &mut HistoryContext<'_>) -> bool {
        let Some(entry) = self.redo_stack.pop() else {
            return false;
        };

        let applied = match &entry {
            HistoryEntry::RasterStroke(stroke) => context
                .raster_target(stroke.target())
                .map(|surface| {
                    stroke.apply_redo(surface);
                    true
                })
                .unwrap_or(false),
            HistoryEntry::Structural(structural) => context
                .layered_document()
                .map(|document| structural.apply_redo(document))
                .unwrap_or(false),
        };

        if !applied {
            self.redo_stack.push(entry);
            return false;
        }

        self.undo_stack.push(entry);
        true
    }

    fn enforce_memory_budget(&mut self) {
        while self.retained_bytes > self.memory_budget_bytes && !self.undo_stack.is_empty() {
            let removed = self.undo_stack.remove(0);
            self.retained_bytes = self
                .retained_bytes
                .saturating_sub(removed.estimated_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HistoryContext, HistoryEntry, HistoryStack, PixelChange, StrokeHistoryEntry,
        StructuralHistoryEntry,
    };
    use common::{DocumentId, LayerId};
    use doc_model::{
        Canvas, Document, DocumentMetadata, LayeredRasterDocument, RasterLayer, RasterSurface,
    };

    fn test_layered_document() -> LayeredRasterDocument {
        let mut document = Document::new(
            DocumentId::new(1),
            Canvas::new(32, 32),
            DocumentMetadata {
                title: "History Test".to_string(),
            },
        );
        document.add_layer(RasterLayer::new(LayerId::new(1), "Base"));

        LayeredRasterDocument::from_document(document)
    }

    #[test]
    fn undo_and_redo_restore_surface_pixels() {
        let mut surface = RasterSurface::new(32, 32);
        let mut history = HistoryStack::new();

        let _ = surface.write_pixel(5, 5, [255, 0, 0, 255]);
        history.push_brush_stroke(StrokeHistoryEntry::new(vec![PixelChange::new(
            5,
            5,
            [0, 0, 0, 0],
            [255, 0, 0, 255],
        )]));

        assert!(history.undo(&mut surface));
        assert_eq!(surface.pixel(5, 5), [0, 0, 0, 0]);

        assert!(history.redo(&mut surface));
        assert_eq!(surface.pixel(5, 5), [255, 0, 0, 255]);
    }

    #[test]
    fn pushing_new_stroke_clears_redo_stack() {
        let mut surface = RasterSurface::new(16, 16);
        let mut history = HistoryStack::new();

        history.push_brush_stroke(StrokeHistoryEntry::new(vec![PixelChange::new(
            1,
            1,
            [0, 0, 0, 0],
            [1, 2, 3, 255],
        )]));
        assert!(history.undo(&mut surface));

        history.push_brush_stroke(StrokeHistoryEntry::new(vec![PixelChange::new(
            2,
            2,
            [0, 0, 0, 0],
            [4, 5, 6, 255],
        )]));

        assert!(!history.can_redo());
    }

    #[test]
    fn structural_entries_undo_and_redo_layer_metadata() {
        let mut document = test_layered_document();
        let mut history = HistoryStack::new();

        history.push_structural_edit(StructuralHistoryEntry::RenameLayer {
            layer_id: LayerId::new(1),
            before_name: "Base".to_string(),
            after_name: "Sketch".to_string(),
        });
        assert!(document.rename_layer(LayerId::new(1), "Sketch"));

        assert!(history.undo_layered(&mut document));
        assert_eq!(document.document().layers()[0].name, "Base");

        assert!(history.redo_layered(&mut document));
        assert_eq!(document.document().layers()[0].name, "Sketch");
    }

    #[test]
    fn structural_entries_restore_deleted_layer_and_surface() {
        let mut document = test_layered_document();
        assert!(document.create_layer(RasterLayer::new(LayerId::new(2), "Paint")));
        let _ = document
            .surface_mut(LayerId::new(2))
            .expect("paint layer surface should exist")
            .write_pixel(3, 4, [9, 8, 7, 255]);
        let deleted_index = document
            .layer_index(LayerId::new(2))
            .expect("layer index should exist");
        let deleted = document
            .delete_layer(LayerId::new(2))
            .expect("layer deletion should succeed");

        let mut history = HistoryStack::new();
        history.push_structural_edit(StructuralHistoryEntry::DeleteLayer {
            layer: deleted.0,
            surface: deleted.1,
            index: deleted_index,
        });

        assert!(history.undo_layered(&mut document));
        assert_eq!(document.document().layers().len(), 2);
        assert_eq!(
            document
                .surface(LayerId::new(2))
                .map(|surface| surface.pixel(3, 4)),
            Some([9, 8, 7, 255])
        );

        assert!(history.redo_layered(&mut document));
        assert!(document.surface(LayerId::new(2)).is_none());
    }

    #[test]
    fn history_budget_discards_oldest_entries() {
        let mut history = HistoryStack::with_memory_budget(std::mem::size_of::<PixelChange>() * 2);

        history.push_entry(HistoryEntry::RasterStroke(StrokeHistoryEntry::new(vec![
            PixelChange::new(1, 1, [0, 0, 0, 0], [1, 0, 0, 255]),
        ])));
        history.push_entry(HistoryEntry::RasterStroke(StrokeHistoryEntry::new(vec![
            PixelChange::new(2, 2, [0, 0, 0, 0], [2, 0, 0, 255]),
        ])));
        history.push_entry(HistoryEntry::RasterStroke(StrokeHistoryEntry::new(vec![
            PixelChange::new(3, 3, [0, 0, 0, 0], [3, 0, 0, 255]),
        ])));

        assert_eq!(history.undo_len(), 2);
    }

    #[test]
    fn mixed_structural_and_paint_history_entries_roundtrip() {
        let mut document = test_layered_document();
        let mut history = HistoryStack::new();

        history.push_structural_edit(StructuralHistoryEntry::CreateLayer {
            layer: RasterLayer::new(LayerId::new(2), "Paint"),
            surface: RasterSurface::new(32, 32),
            index: 1,
        });
        assert!(document.insert_layer_with_surface(
            1,
            RasterLayer::new(LayerId::new(2), "Paint"),
            RasterSurface::new(32, 32),
        ));

        history.push_entry(HistoryEntry::RasterStroke(StrokeHistoryEntry::for_layer(
            LayerId::new(2),
            vec![PixelChange::new(6, 7, [0, 0, 0, 0], [12, 34, 56, 255])],
        )));
        let _ = document
            .surface_mut(LayerId::new(2))
            .expect("paint layer should exist")
            .write_pixel(6, 7, [12, 34, 56, 255]);

        {
            let mut context = HistoryContext::for_layered(&mut document);
            assert!(history.undo_with_context(&mut context));
            assert!(history.undo_with_context(&mut context));
        }

        assert!(document.surface(LayerId::new(2)).is_none());

        {
            let mut context = HistoryContext::for_layered(&mut document);
            assert!(history.redo_with_context(&mut context));
            assert!(history.redo_with_context(&mut context));
        }

        assert_eq!(
            document
                .surface(LayerId::new(2))
                .map(|surface| surface.pixel(6, 7)),
            Some([12, 34, 56, 255])
        );
    }
}
