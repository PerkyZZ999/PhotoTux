use common::{CanvasRect, CanvasSize, DEFAULT_TILE_SIZE, DocumentId, GroupId, LayerId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileCoord {
    pub x: u32,
    pub y: u32,
}

impl TileCoord {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RasterTile {
    pub pixels: Vec<u8>,
}

impl RasterTile {
    pub fn new(tile_size: u32) -> Self {
        let pixel_count = tile_size as usize * tile_size as usize * 4;
        Self {
            pixels: vec![0; pixel_count],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaskTile {
    pub alpha: Vec<u8>,
}

impl MaskTile {
    pub fn new(tile_size: u32) -> Self {
        let pixel_count = tile_size as usize * tile_size as usize;
        Self {
            alpha: vec![255; pixel_count],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RasterMask {
    pub enabled: bool,
    pub tiles: HashMap<TileCoord, MaskTile>,
    pub dirty_tiles: HashSet<TileCoord>,
}

impl RasterMask {
    pub fn new() -> Self {
        Self {
            enabled: true,
            tiles: HashMap::new(),
            dirty_tiles: HashSet::new(),
        }
    }

    pub fn ensure_tile(&mut self, coord: TileCoord, tile_size: u32) -> &mut MaskTile {
        self.dirty_tiles.insert(coord);
        self.tiles
            .entry(coord)
            .or_insert_with(|| MaskTile::new(tile_size))
    }

    pub fn mark_tile_dirty(&mut self, coord: TileCoord) {
        self.dirty_tiles.insert(coord);
    }

    pub fn take_dirty_tiles(&mut self) -> Vec<TileCoord> {
        let mut dirty_tiles = self.dirty_tiles.drain().collect::<Vec<_>>();
        dirty_tiles.sort_by_key(|coord| (coord.y, coord.x));
        dirty_tiles
    }
}

impl Default for RasterMask {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterLayer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub opacity_percent: u8,
    pub blend_mode: BlendMode,
    pub mask: Option<RasterMask>,
    pub offset_x: i32,
    pub offset_y: i32,
    pub tiles: HashMap<TileCoord, RasterTile>,
    pub dirty_tiles: HashSet<TileCoord>,
}

impl RasterLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            opacity_percent: 100,
            blend_mode: BlendMode::Normal,
            mask: None,
            offset_x: 0,
            offset_y: 0,
            tiles: HashMap::new(),
            dirty_tiles: HashSet::new(),
        }
    }

    pub fn ensure_tile(&mut self, coord: TileCoord, tile_size: u32) -> &mut RasterTile {
        self.dirty_tiles.insert(coord);
        self.tiles
            .entry(coord)
            .or_insert_with(|| RasterTile::new(tile_size))
    }

    pub fn mark_tile_dirty(&mut self, coord: TileCoord) {
        self.dirty_tiles.insert(coord);
    }

    pub fn take_dirty_tiles(&mut self) -> Vec<TileCoord> {
        let mut dirty_tiles = self.dirty_tiles.drain().collect::<Vec<_>>();
        dirty_tiles.sort_by_key(|coord| (coord.y, coord.x));
        dirty_tiles
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileGridSize {
    pub columns: u32,
    pub rows: u32,
}

pub type RectSelection = CanvasRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionPoint {
    pub x: i32,
    pub y: i32,
}

impl SelectionPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeformSelection {
    pub points: Vec<SelectionPoint>,
}

impl FreeformSelection {
    pub fn new(points: Vec<SelectionPoint>) -> Option<Self> {
        if points.len() < 3 {
            return None;
        }

        Some(Self { points })
    }

    pub fn bounds(&self) -> CanvasRect {
        let mut min_x = self.points[0].x;
        let mut max_x = self.points[0].x;
        let mut min_y = self.points[0].y;
        let mut max_y = self.points[0].y;

        for point in &self.points[1..] {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }

        CanvasRect::new(
            min_x,
            min_y,
            (max_x - min_x + 1) as u32,
            (max_y - min_y + 1) as u32,
        )
    }

    pub fn contains_pixel(&self, pixel_x: i32, pixel_y: i32) -> bool {
        let mut inside = false;
        let mut previous = *self
            .points
            .last()
            .expect("freeform selections always have at least three points");

        for current in &self.points {
            let intersects_scanline = (current.y > pixel_y) != (previous.y > pixel_y);
            if intersects_scanline {
                let delta_y = previous.y - current.y;
                if delta_y != 0 {
                    let x_intersection = current.x as f32
                        + ((pixel_y - current.y) as f32 * (previous.x - current.x) as f32)
                            / delta_y as f32;
                    if pixel_x as f32 <= x_intersection {
                        inside = !inside;
                    }
                }
            }
            previous = *current;
        }

        inside
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionShape {
    Rectangular(RectSelection),
    Freeform(FreeformSelection),
}

impl SelectionShape {
    pub fn bounds(&self) -> CanvasRect {
        match self {
            Self::Rectangular(selection) => *selection,
            Self::Freeform(selection) => selection.bounds(),
        }
    }

    pub fn contains_pixel(&self, pixel_x: i32, pixel_y: i32) -> bool {
        match self {
            Self::Rectangular(selection) => {
                let right = selection.x + selection.width as i32;
                let bottom = selection.y + selection.height as i32;
                pixel_x >= selection.x
                    && pixel_x < right
                    && pixel_y >= selection.y
                    && pixel_y < bottom
            }
            Self::Freeform(selection) => selection.contains_pixel(pixel_x, pixel_y),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuideOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guide {
    pub orientation: GuideOrientation,
    pub position: i32,
}

impl Guide {
    pub const fn horizontal(position: i32) -> Self {
        Self {
            orientation: GuideOrientation::Horizontal,
            position,
        }
    }

    pub const fn vertical(position: i32) -> Self {
        Self {
            orientation: GuideOrientation::Vertical,
            position,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerEditTarget {
    LayerPixels,
    LayerMask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerStateSnapshot {
    pub offset_x: i32,
    pub offset_y: i32,
    pub tiles: HashMap<TileCoord, RasterTile>,
    pub mask: Option<RasterMask>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    Raster,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: String,
    pub font_size_px: u32,
    pub line_height_percent: u32,
    pub letter_spacing: i32,
    pub fill_rgba: [u8; 4],
    pub alignment: TextAlignment,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Bitmap Sans".to_string(),
            font_size_px: 48,
            line_height_percent: 120,
            letter_spacing: 0,
            fill_rgba: [255, 255, 255, 255],
            alignment: TextAlignment::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextTransform {
    pub origin_x: i32,
    pub origin_y: i32,
}

impl TextTransform {
    pub const fn new(origin_x: i32, origin_y: i32) -> Self {
        Self { origin_x, origin_y }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextLayer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub opacity_percent: u8,
    pub blend_mode: BlendMode,
    pub transform: TextTransform,
    pub content: String,
    pub style: TextStyle,
}

impl TextLayer {
    pub fn new(
        name: impl Into<String>,
        content: impl Into<String>,
        transform: TextTransform,
    ) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            visible: true,
            opacity_percent: 100,
            blend_mode: BlendMode::Normal,
            transform,
            content: content.into(),
            style: TextStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerHierarchyNodeRef {
    Layer(LayerId),
    Group(GroupId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerGroup {
    pub id: GroupId,
    pub name: String,
    pub visible: bool,
    pub opacity_percent: u8,
    pub children: Vec<LayerHierarchyNode>,
}

impl LayerGroup {
    pub fn new(name: impl Into<String>, children: Vec<LayerHierarchyNode>) -> Self {
        Self {
            id: GroupId::new(),
            name: name.into(),
            visible: true,
            opacity_percent: 100,
            children,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerHierarchyNode {
    Layer(LayerId),
    Group(LayerGroup),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub canvas_size: CanvasSize,
    pub layers: Vec<RasterLayer>,
    pub text_layers: Vec<TextLayer>,
    pub layer_hierarchy: Vec<LayerHierarchyNode>,
    pub active_layer_index: usize,
    pub active_edit_target: LayerEditTarget,
    pub tile_size: u32,
    pub selection: Option<SelectionShape>,
    pub selection_inverted: bool,
    pub guides: Vec<Guide>,
    pub guides_visible: bool,
}

impl Document {
    pub fn new(width: u32, height: u32) -> Self {
        let background = RasterLayer::new("Background");
        let background_id = background.id;
        Self {
            id: DocumentId::new(),
            canvas_size: CanvasSize::new(width, height),
            layers: vec![background],
            text_layers: Vec::new(),
            layer_hierarchy: vec![LayerHierarchyNode::Layer(background_id)],
            active_layer_index: 0,
            active_edit_target: LayerEditTarget::LayerPixels,
            tile_size: DEFAULT_TILE_SIZE,
            selection: None,
            selection_inverted: false,
            guides: Vec::new(),
            guides_visible: true,
        }
    }

    pub fn tile_grid_size(&self) -> TileGridSize {
        TileGridSize {
            columns: self.canvas_size.width.div_ceil(self.tile_size),
            rows: self.canvas_size.height.div_ceil(self.tile_size),
        }
    }

    pub fn tile_coord_for_pixel(&self, pixel_x: u32, pixel_y: u32) -> Option<TileCoord> {
        if pixel_x >= self.canvas_size.width || pixel_y >= self.canvas_size.height {
            return None;
        }

        Some(TileCoord::new(
            pixel_x / self.tile_size,
            pixel_y / self.tile_size,
        ))
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn total_layer_count(&self) -> usize {
        self.layers.len() + self.text_layers.len()
    }

    pub fn active_layer(&self) -> &RasterLayer {
        &self.layers[self.active_layer_index]
    }

    pub fn active_layer_index(&self) -> usize {
        self.active_layer_index
    }

    pub fn active_edit_target(&self) -> LayerEditTarget {
        self.active_edit_target
    }

    pub fn guides(&self) -> &[Guide] {
        &self.guides
    }

    pub fn guides_visible(&self) -> bool {
        self.guides_visible
    }

    pub fn add_guide(&mut self, guide: Guide) {
        self.guides.push(guide);
    }

    pub fn remove_last_guide(&mut self) -> Option<Guide> {
        self.guides.pop()
    }

    pub fn set_guides_state(&mut self, guides: Vec<Guide>, visible: bool) {
        self.guides = guides;
        self.guides_visible = visible;
    }

    pub fn toggle_guides_visible(&mut self) {
        self.guides_visible = !self.guides_visible;
    }

    pub fn layer_hierarchy(&self) -> &[LayerHierarchyNode] {
        &self.layer_hierarchy
    }

    pub fn group_count(&self) -> usize {
        Self::count_groups_in_nodes(&self.layer_hierarchy)
    }

    pub fn group(&self, group_id: GroupId) -> Option<&LayerGroup> {
        Self::find_group_in_nodes(&self.layer_hierarchy, group_id)
    }

    pub fn group_for_layer(&self, layer_id: LayerId) -> Option<GroupId> {
        Self::find_parent_group_for_ref(
            &self.layer_hierarchy,
            LayerHierarchyNodeRef::Layer(layer_id),
            None,
        )
    }

    pub fn set_layer_hierarchy(
        &mut self,
        layer_hierarchy: Vec<LayerHierarchyNode>,
    ) -> Result<(), &'static str> {
        let previous = std::mem::replace(&mut self.layer_hierarchy, layer_hierarchy);
        if let Err(error) = self.validate_layer_hierarchy() {
            self.layer_hierarchy = previous;
            return Err(error);
        }

        Ok(())
    }

    pub fn create_layer_group(
        &mut self,
        name: impl Into<String>,
        child_layer_ids: &[LayerId],
    ) -> Option<GroupId> {
        if child_layer_ids.is_empty() {
            return None;
        }

        let mut unique_ids = HashSet::new();
        for layer_id in child_layer_ids {
            if !unique_ids.insert(*layer_id) {
                return None;
            }
        }

        let mut insertion_index = None;
        let mut grouped_children = Vec::with_capacity(child_layer_ids.len());
        for expected_layer_id in child_layer_ids {
            let node_index = self.layer_hierarchy.iter().position(|node| {
                matches!(node, LayerHierarchyNode::Layer(layer_id) if layer_id == expected_layer_id)
            })?;
            if insertion_index.is_none() {
                insertion_index = Some(node_index);
            }
            let node = self.layer_hierarchy.remove(node_index);
            grouped_children.push(node);
        }

        let group = LayerGroup::new(name, grouped_children);
        let group_id = group.id;
        self.layer_hierarchy.insert(
            insertion_index.unwrap_or(self.layer_hierarchy.len()),
            LayerHierarchyNode::Group(group),
        );

        self.validate_layer_hierarchy().ok()?;
        Some(group_id)
    }

    pub fn wrap_hierarchy_node_in_group(
        &mut self,
        target: LayerHierarchyNodeRef,
        name: impl Into<String>,
    ) -> Option<GroupId> {
        let mut hierarchy = self.layer_hierarchy.clone();
        let group_id = Self::wrap_node_in_group_in_nodes(&mut hierarchy, target, name.into())?;
        self.set_layer_hierarchy(hierarchy).ok()?;
        Some(group_id)
    }

    pub fn ungroup(&mut self, group_id: GroupId) -> bool {
        let mut hierarchy = self.layer_hierarchy.clone();
        if !Self::ungroup_in_nodes(&mut hierarchy, group_id) {
            return false;
        }

        self.set_layer_hierarchy(hierarchy).is_ok()
    }

    pub fn move_node_into_group(
        &mut self,
        node_ref: LayerHierarchyNodeRef,
        group_id: GroupId,
    ) -> bool {
        if node_ref == LayerHierarchyNodeRef::Group(group_id) {
            return false;
        }

        let mut hierarchy = self.layer_hierarchy.clone();
        let Some(node) = Self::extract_node_from_nodes(&mut hierarchy, node_ref) else {
            return false;
        };
        if Self::node_contains_group_id(&node, group_id) {
            return false;
        }
        if Self::insert_node_into_group(&mut hierarchy, group_id, node).is_err() {
            return false;
        }

        self.set_layer_hierarchy(hierarchy).is_ok()
    }

    pub fn move_node_out_of_group(&mut self, node_ref: LayerHierarchyNodeRef) -> bool {
        let mut hierarchy = self.layer_hierarchy.clone();
        let Some((node, Some(parent_group_id))) =
            Self::extract_node_with_parent_group(&mut hierarchy, node_ref, None)
        else {
            return false;
        };
        if Self::insert_node_after_group(&mut hierarchy, parent_group_id, node).is_err() {
            return false;
        }

        self.set_layer_hierarchy(hierarchy).is_ok()
    }

    pub fn set_group_visibility(&mut self, group_id: GroupId, visible: bool) -> bool {
        let Some(group) = Self::find_group_mut_in_nodes(&mut self.layer_hierarchy, group_id) else {
            return false;
        };
        group.visible = visible;
        true
    }

    pub fn validate_layer_hierarchy(&self) -> Result<(), &'static str> {
        let known_layer_ids = self
            .layers
            .iter()
            .map(|layer| layer.id)
            .chain(self.text_layers.iter().map(|layer| layer.id))
            .collect::<HashSet<_>>();
        let mut referenced_layer_ids = HashSet::new();
        let mut referenced_group_ids = HashSet::new();
        Self::validate_hierarchy_nodes(
            &self.layer_hierarchy,
            &known_layer_ids,
            &mut referenced_layer_ids,
            &mut referenced_group_ids,
        )?;

        if referenced_layer_ids != known_layer_ids {
            return Err("layer hierarchy does not reference every document layer exactly once");
        }

        Ok(())
    }

    pub fn layer(&self, index: usize) -> Option<&RasterLayer> {
        self.layers.get(index)
    }

    pub fn text_layers(&self) -> &[TextLayer] {
        &self.text_layers
    }

    pub fn text_layer_count(&self) -> usize {
        self.text_layers.len()
    }

    pub fn text_layer(&self, index: usize) -> Option<&TextLayer> {
        self.text_layers.get(index)
    }

    pub fn text_layer_mut(&mut self, index: usize) -> Option<&mut TextLayer> {
        self.text_layers.get_mut(index)
    }

    pub fn layer_by_id(&self, layer_id: LayerId) -> Option<&RasterLayer> {
        let index = self.layer_index_by_id(layer_id)?;
        self.layers.get(index)
    }

    pub fn text_layer_by_id(&self, layer_id: LayerId) -> Option<&TextLayer> {
        let index = self.text_layer_index_by_id(layer_id)?;
        self.text_layers.get(index)
    }

    pub fn text_layer_mut_by_id(&mut self, layer_id: LayerId) -> Option<&mut TextLayer> {
        let index = self.text_layer_index_by_id(layer_id)?;
        self.text_layers.get_mut(index)
    }

    pub fn layer_kind_by_id(&self, layer_id: LayerId) -> Option<LayerKind> {
        if self.layer_index_by_id(layer_id).is_some() {
            return Some(LayerKind::Raster);
        }
        if self.text_layer_index_by_id(layer_id).is_some() {
            return Some(LayerKind::Text);
        }
        None
    }

    pub fn layer_mut(&mut self, index: usize) -> Option<&mut RasterLayer> {
        self.layers.get_mut(index)
    }

    pub fn layer_index_by_id(&self, layer_id: LayerId) -> Option<usize> {
        self.layers.iter().position(|layer| layer.id == layer_id)
    }

    pub fn text_layer_index_by_id(&self, layer_id: LayerId) -> Option<usize> {
        self.text_layers.iter().position(|layer| layer.id == layer_id)
    }

    pub fn has_visible_text_layers(&self) -> bool {
        self.text_layers.iter().any(|layer| layer.visible)
    }

    pub fn insert_text_layer(&mut self, layer: TextLayer, after_layer_id: Option<LayerId>) -> bool {
        if self.layer_kind_by_id(layer.id).is_some() {
            return false;
        }

        let backup_hierarchy = self.layer_hierarchy.clone();
        self.text_layers.push(layer.clone());
        if self.group_count() == 0 {
            if let Some(after_layer_id) = after_layer_id {
                self.insert_flat_layer_after(after_layer_id, layer.id);
            } else {
                self.layer_hierarchy.push(LayerHierarchyNode::Layer(layer.id));
            }
        } else {
            self.layer_hierarchy.push(LayerHierarchyNode::Layer(layer.id));
        }

        if self.validate_layer_hierarchy().is_err() {
            self.text_layers.retain(|candidate| candidate.id != layer.id);
            self.layer_hierarchy = backup_hierarchy;
            return false;
        }

        true
    }

    pub fn upsert_text_layer_storage(&mut self, layer: TextLayer) {
        if let Some(existing) = self.text_layer_mut_by_id(layer.id) {
            *existing = layer;
        } else {
            self.text_layers.push(layer);
        }
    }

    pub fn remove_text_layer_storage(&mut self, layer_id: LayerId) -> bool {
        let Some(index) = self.text_layer_index_by_id(layer_id) else {
            return false;
        };
        self.text_layers.remove(index);
        true
    }

    pub fn remove_text_layer(&mut self, layer_id: LayerId) -> bool {
        let Some(text_index) = self.text_layer_index_by_id(layer_id) else {
            return false;
        };

        let mut hierarchy = self.layer_hierarchy.clone();
        if Self::extract_node_from_nodes(&mut hierarchy, LayerHierarchyNodeRef::Layer(layer_id))
            .is_none()
        {
            return false;
        }

        self.text_layers.remove(text_index);
        self.set_layer_hierarchy(hierarchy).is_ok()
    }

    pub fn rename_text_layer(&mut self, layer_id: LayerId, name: impl Into<String>) -> bool {
        let Some(layer) = self.text_layer_mut_by_id(layer_id) else {
            return false;
        };
        layer.name = name.into();
        true
    }

    pub fn set_text_layer_visibility(&mut self, layer_id: LayerId, visible: bool) -> bool {
        let Some(layer) = self.text_layer_mut_by_id(layer_id) else {
            return false;
        };
        layer.visible = visible;
        true
    }

    pub fn set_text_layer_opacity(&mut self, layer_id: LayerId, opacity_percent: u8) -> bool {
        let Some(layer) = self.text_layer_mut_by_id(layer_id) else {
            return false;
        };
        layer.opacity_percent = opacity_percent.min(100);
        true
    }

    pub fn set_text_layer_blend_mode(&mut self, layer_id: LayerId, blend_mode: BlendMode) -> bool {
        let Some(layer) = self.text_layer_mut_by_id(layer_id) else {
            return false;
        };
        layer.blend_mode = blend_mode;
        true
    }

    pub fn set_text_layer_content(&mut self, layer_id: LayerId, content: impl Into<String>) -> bool {
        let Some(layer) = self.text_layer_mut_by_id(layer_id) else {
            return false;
        };
        layer.content = content.into();
        true
    }

    pub fn set_text_layer_style(&mut self, layer_id: LayerId, style: TextStyle) -> bool {
        let Some(layer) = self.text_layer_mut_by_id(layer_id) else {
            return false;
        };
        layer.style = style;
        true
    }

    pub fn set_text_layer_transform(&mut self, layer_id: LayerId, transform: TextTransform) -> bool {
        let Some(layer) = self.text_layer_mut_by_id(layer_id) else {
            return false;
        };
        layer.transform = transform;
        true
    }

    pub fn translate_text_layer(&mut self, layer_id: LayerId, delta_x: i32, delta_y: i32) -> bool {
        let Some(layer) = self.text_layer_mut_by_id(layer_id) else {
            return false;
        };
        layer.transform.origin_x += delta_x;
        layer.transform.origin_y += delta_y;
        true
    }

    pub fn tile_origin(&self, coord: TileCoord) -> (u32, u32) {
        (coord.x * self.tile_size, coord.y * self.tile_size)
    }

    pub fn layer_offset(&self, index: usize) -> Option<(i32, i32)> {
        let layer = self.layers.get(index)?;
        Some((layer.offset_x, layer.offset_y))
    }

    pub fn layer_canvas_bounds(&self, index: usize) -> Option<CanvasRect> {
        let layer = self.layers.get(index)?;
        let mut tile_iter = layer.tiles.keys();
        let first = *tile_iter.next()?;
        let mut min_x = first.x;
        let mut max_x = first.x;
        let mut min_y = first.y;
        let mut max_y = first.y;

        for coord in tile_iter {
            min_x = min_x.min(coord.x);
            max_x = max_x.max(coord.x);
            min_y = min_y.min(coord.y);
            max_y = max_y.max(coord.y);
        }

        Some(CanvasRect::new(
            min_x as i32 * self.tile_size as i32 + layer.offset_x,
            min_y as i32 * self.tile_size as i32 + layer.offset_y,
            (max_x - min_x + 1) * self.tile_size,
            (max_y - min_y + 1) * self.tile_size,
        ))
    }

    fn count_groups_in_nodes(nodes: &[LayerHierarchyNode]) -> usize {
        nodes
            .iter()
            .map(|node| match node {
                LayerHierarchyNode::Layer(_) => 0,
                LayerHierarchyNode::Group(group) => {
                    1 + Self::count_groups_in_nodes(&group.children)
                }
            })
            .sum()
    }

    fn find_group_in_nodes(nodes: &[LayerHierarchyNode], group_id: GroupId) -> Option<&LayerGroup> {
        for node in nodes {
            if let LayerHierarchyNode::Group(group) = node {
                if group.id == group_id {
                    return Some(group);
                }
                if let Some(found) = Self::find_group_in_nodes(&group.children, group_id) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn find_group_mut_in_nodes(
        nodes: &mut [LayerHierarchyNode],
        group_id: GroupId,
    ) -> Option<&mut LayerGroup> {
        for node in nodes {
            if let LayerHierarchyNode::Group(group) = node {
                if group.id == group_id {
                    return Some(group);
                }
                if let Some(found) = Self::find_group_mut_in_nodes(&mut group.children, group_id) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn find_parent_group_for_ref(
        nodes: &[LayerHierarchyNode],
        target: LayerHierarchyNodeRef,
        parent_group_id: Option<GroupId>,
    ) -> Option<GroupId> {
        for node in nodes {
            if Self::node_matches_ref(node, target) {
                return parent_group_id;
            }
            if let LayerHierarchyNode::Group(group) = node
                && let Some(found) =
                    Self::find_parent_group_for_ref(&group.children, target, Some(group.id))
            {
                return Some(found);
            }
        }

        None
    }

    fn node_matches_ref(node: &LayerHierarchyNode, target: LayerHierarchyNodeRef) -> bool {
        match (node, target) {
            (LayerHierarchyNode::Layer(layer_id), LayerHierarchyNodeRef::Layer(target_id)) => {
                *layer_id == target_id
            }
            (LayerHierarchyNode::Group(group), LayerHierarchyNodeRef::Group(target_id)) => {
                group.id == target_id
            }
            _ => false,
        }
    }

    fn wrap_node_in_group_in_nodes(
        nodes: &mut Vec<LayerHierarchyNode>,
        target: LayerHierarchyNodeRef,
        name: String,
    ) -> Option<GroupId> {
        for index in 0..nodes.len() {
            if Self::node_matches_ref(&nodes[index], target) {
                let child = nodes.remove(index);
                let group = LayerGroup::new(name, vec![child]);
                let group_id = group.id;
                nodes.insert(index, LayerHierarchyNode::Group(group));
                return Some(group_id);
            }
            if let LayerHierarchyNode::Group(group) = &mut nodes[index]
                && let Some(group_id) =
                    Self::wrap_node_in_group_in_nodes(&mut group.children, target, name.clone())
            {
                return Some(group_id);
            }
        }

        None
    }

    fn ungroup_in_nodes(nodes: &mut Vec<LayerHierarchyNode>, group_id: GroupId) -> bool {
        for index in 0..nodes.len() {
            match &mut nodes[index] {
                LayerHierarchyNode::Group(group) if group.id == group_id => {
                    let LayerHierarchyNode::Group(group) = nodes.remove(index) else {
                        return false;
                    };
                    nodes.splice(index..index, group.children);
                    return true;
                }
                LayerHierarchyNode::Group(group) => {
                    if Self::ungroup_in_nodes(&mut group.children, group_id) {
                        return true;
                    }
                }
                LayerHierarchyNode::Layer(_) => {}
            }
        }

        false
    }

    fn extract_node_from_nodes(
        nodes: &mut Vec<LayerHierarchyNode>,
        target: LayerHierarchyNodeRef,
    ) -> Option<LayerHierarchyNode> {
        for index in 0..nodes.len() {
            if Self::node_matches_ref(&nodes[index], target) {
                return Some(nodes.remove(index));
            }
            if let LayerHierarchyNode::Group(group) = &mut nodes[index]
                && let Some(node) = Self::extract_node_from_nodes(&mut group.children, target)
            {
                return Some(node);
            }
        }

        None
    }

    fn extract_node_with_parent_group(
        nodes: &mut Vec<LayerHierarchyNode>,
        target: LayerHierarchyNodeRef,
        parent_group_id: Option<GroupId>,
    ) -> Option<(LayerHierarchyNode, Option<GroupId>)> {
        for index in 0..nodes.len() {
            if Self::node_matches_ref(&nodes[index], target) {
                return Some((nodes.remove(index), parent_group_id));
            }
            if let LayerHierarchyNode::Group(group) = &mut nodes[index]
                && let Some(result) = Self::extract_node_with_parent_group(
                    &mut group.children,
                    target,
                    Some(group.id),
                )
            {
                return Some(result);
            }
        }

        None
    }

    fn insert_node_into_group(
        nodes: &mut Vec<LayerHierarchyNode>,
        group_id: GroupId,
        node: LayerHierarchyNode,
    ) -> Result<(), LayerHierarchyNode> {
        let mut pending_node = Some(node);
        for entry in nodes {
            if let LayerHierarchyNode::Group(group) = entry {
                if group.id == group_id {
                    group
                        .children
                        .push(pending_node.take().expect("node should still be present"));
                    return Ok(());
                }
                if let Err(node) = Self::insert_node_into_group(
                    &mut group.children,
                    group_id,
                    pending_node.take().expect("node should still be present"),
                ) {
                    pending_node = Some(node);
                } else {
                    return Ok(());
                }
            }
        }

        Err(pending_node.expect("node should still be present"))
    }

    fn insert_node_after_group(
        nodes: &mut Vec<LayerHierarchyNode>,
        group_id: GroupId,
        node: LayerHierarchyNode,
    ) -> Result<(), LayerHierarchyNode> {
        let mut pending_node = Some(node);
        for index in 0..nodes.len() {
            if let LayerHierarchyNode::Group(group) = &mut nodes[index] {
                if group.id == group_id {
                    nodes.insert(
                        index + 1,
                        pending_node.take().expect("node should still be present"),
                    );
                    return Ok(());
                }
                if let Err(node) = Self::insert_node_after_group(
                    &mut group.children,
                    group_id,
                    pending_node.take().expect("node should still be present"),
                ) {
                    pending_node = Some(node);
                } else {
                    return Ok(());
                }
            }
        }

        Err(pending_node.expect("node should still be present"))
    }

    fn node_contains_group_id(node: &LayerHierarchyNode, group_id: GroupId) -> bool {
        match node {
            LayerHierarchyNode::Layer(_) => false,
            LayerHierarchyNode::Group(group) => {
                group.id == group_id
                    || group
                        .children
                        .iter()
                        .any(|child| Self::node_contains_group_id(child, group_id))
            }
        }
    }

    fn validate_hierarchy_nodes(
        nodes: &[LayerHierarchyNode],
        known_layer_ids: &HashSet<LayerId>,
        referenced_layer_ids: &mut HashSet<LayerId>,
        referenced_group_ids: &mut HashSet<GroupId>,
    ) -> Result<(), &'static str> {
        for node in nodes {
            match node {
                LayerHierarchyNode::Layer(layer_id) => {
                    if !known_layer_ids.contains(layer_id) {
                        return Err("layer hierarchy references a missing layer");
                    }
                    if !referenced_layer_ids.insert(*layer_id) {
                        return Err("layer hierarchy references the same layer more than once");
                    }
                }
                LayerHierarchyNode::Group(group) => {
                    if !referenced_group_ids.insert(group.id) {
                        return Err("layer hierarchy references the same group more than once");
                    }
                    Self::validate_hierarchy_nodes(
                        &group.children,
                        known_layer_ids,
                        referenced_layer_ids,
                        referenced_group_ids,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn rebuild_flat_layer_hierarchy(&mut self) {
        if self.layer_hierarchy.iter().any(|node| matches!(node, LayerHierarchyNode::Group(_))) {
            return;
        }

        let mut raster_ids = self.layers.iter().map(|layer| layer.id);
        let text_ids = self
            .text_layers
            .iter()
            .map(|layer| layer.id)
            .collect::<HashSet<_>>();
        let mut rebuilt = Vec::with_capacity(self.total_layer_count());

        for node in &self.layer_hierarchy {
            match node {
                LayerHierarchyNode::Layer(layer_id) if text_ids.contains(layer_id) => {
                    rebuilt.push(LayerHierarchyNode::Layer(*layer_id));
                }
                LayerHierarchyNode::Layer(_) => {
                    if let Some(layer_id) = raster_ids.next() {
                        rebuilt.push(LayerHierarchyNode::Layer(layer_id));
                    }
                }
                LayerHierarchyNode::Group(_) => {}
            }
        }

        for layer_id in raster_ids {
            rebuilt.push(LayerHierarchyNode::Layer(layer_id));
        }

        for layer in &self.text_layers {
            if !rebuilt.iter().any(|node| matches!(node, LayerHierarchyNode::Layer(layer_id) if *layer_id == layer.id)) {
                rebuilt.push(LayerHierarchyNode::Layer(layer.id));
            }
        }

        self.layer_hierarchy = rebuilt;
    }

    fn insert_flat_layer_after(&mut self, after_layer_id: LayerId, inserted_layer_id: LayerId) {
        if self.group_count() != 0 {
            self.layer_hierarchy
                .push(LayerHierarchyNode::Layer(inserted_layer_id));
            return;
        }

        let insertion_index = self
            .layer_hierarchy
            .iter()
            .position(|node| matches!(node, LayerHierarchyNode::Layer(layer_id) if *layer_id == after_layer_id))
            .map(|index| index + 1)
            .unwrap_or(self.layer_hierarchy.len());
        self.layer_hierarchy
            .insert(insertion_index, LayerHierarchyNode::Layer(inserted_layer_id));
    }

    pub fn selection(&self) -> Option<RectSelection> {
        self.selection.as_ref().map(SelectionShape::bounds)
    }

    pub fn selection_shape(&self) -> Option<&SelectionShape> {
        self.selection.as_ref()
    }

    pub fn selection_inverted(&self) -> bool {
        self.selection_inverted
    }

    pub fn set_selection_shape_state(&mut self, selection: Option<SelectionShape>, inverted: bool) {
        self.selection = selection;
        self.selection_inverted = self.selection.is_some() && inverted;
    }

    pub fn set_selection_state(&mut self, selection: Option<RectSelection>, inverted: bool) {
        self.set_selection_shape_state(selection.map(SelectionShape::Rectangular), inverted);
    }

    pub fn set_selection(&mut self, selection: RectSelection) {
        self.set_selection_state(Some(selection), false);
    }

    pub fn set_freeform_selection_state(
        &mut self,
        selection: Option<FreeformSelection>,
        inverted: bool,
    ) {
        self.set_selection_shape_state(selection.map(SelectionShape::Freeform), inverted);
    }

    pub fn set_freeform_selection(&mut self, selection: FreeformSelection) {
        self.set_freeform_selection_state(Some(selection), false);
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.selection_inverted = false;
    }

    pub fn invert_selection(&mut self) -> bool {
        if self.selection.is_none() {
            return false;
        }

        self.selection_inverted = !self.selection_inverted;
        true
    }

    pub fn selection_contains_pixel(&self, pixel_x: i32, pixel_y: i32) -> bool {
        let Some(selection) = self.selection.as_ref() else {
            return false;
        };

        selection.contains_pixel(pixel_x, pixel_y)
    }

    pub fn allows_pixel_edit(&self, pixel_x: i32, pixel_y: i32) -> bool {
        if self.selection.is_none() {
            return true;
        }

        self.selection_contains_pixel(pixel_x, pixel_y) != self.selection_inverted
    }

    pub fn tile_coords_in_radius(
        &self,
        center_x: f32,
        center_y: f32,
        radius: f32,
    ) -> Vec<TileCoord> {
        if radius <= 0.0 {
            return Vec::new();
        }

        let min_x = center_x - radius;
        let min_y = center_y - radius;
        let max_x = center_x + radius;
        let max_y = center_y + radius;

        if max_x < 0.0
            || max_y < 0.0
            || min_x >= self.canvas_size.width as f32
            || min_y >= self.canvas_size.height as f32
        {
            return Vec::new();
        }

        let start_tile_x = (min_x.max(0.0) as u32) / self.tile_size;
        let start_tile_y = (min_y.max(0.0) as u32) / self.tile_size;
        let end_tile_x = (max_x.floor().max(0.0) as u32)
            .min(self.canvas_size.width.saturating_sub(1))
            / self.tile_size;
        let end_tile_y = (max_y.floor().max(0.0) as u32)
            .min(self.canvas_size.height.saturating_sub(1))
            / self.tile_size;

        let mut coords = Vec::new();
        for tile_y in start_tile_y..=end_tile_y {
            for tile_x in start_tile_x..=end_tile_x {
                coords.push(TileCoord::new(tile_x, tile_y));
            }
        }

        coords
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> LayerId {
        let active_layer_id = self.active_layer().id;
        let layer = RasterLayer::new(name);
        let layer_id = layer.id;
        self.layers.insert(self.active_layer_index + 1, layer);
        if self.group_count() == 0 {
            self.insert_flat_layer_after(active_layer_id, layer_id);
        }
        self.active_layer_index += 1;
        layer_id
    }

    pub fn add_text_layer(
        &mut self,
        name: impl Into<String>,
        content: impl Into<String>,
        transform: TextTransform,
    ) -> LayerId {
        let active_layer_id = self.active_layer().id;
        let layer = TextLayer::new(name, content, transform);
        let layer_id = layer.id;
        self.text_layers.push(layer);
        if self.group_count() == 0 {
            self.insert_flat_layer_after(active_layer_id, layer_id);
        } else {
            self.layer_hierarchy.push(LayerHierarchyNode::Layer(layer_id));
        }
        layer_id
    }

    pub fn set_active_layer(&mut self, index: usize) -> bool {
        if index >= self.layers.len() {
            return false;
        }

        self.active_layer_index = index;
        if self.active_edit_target == LayerEditTarget::LayerMask
            && self.layers[index].mask.is_none()
        {
            self.active_edit_target = LayerEditTarget::LayerPixels;
        }
        true
    }

    pub fn set_active_edit_target(&mut self, target: LayerEditTarget) -> bool {
        if target == LayerEditTarget::LayerMask && self.active_layer().mask.is_none() {
            return false;
        }

        self.active_edit_target = target;
        true
    }

    pub fn layer_mask(&self, index: usize) -> Option<&RasterMask> {
        self.layers.get(index)?.mask.as_ref()
    }

    pub fn layer_mask_mut(&mut self, index: usize) -> Option<&mut RasterMask> {
        self.layers.get_mut(index)?.mask.as_mut()
    }

    pub fn add_layer_mask(&mut self, index: usize) -> bool {
        let Some(layer) = self.layers.get_mut(index) else {
            return false;
        };
        if layer.mask.is_some() {
            return false;
        }

        layer.mask = Some(RasterMask::new());
        true
    }

    pub fn remove_layer_mask(&mut self, index: usize) -> bool {
        let Some(layer) = self.layers.get_mut(index) else {
            return false;
        };
        if layer.mask.is_none() {
            return false;
        }

        layer.mask = None;
        if self.active_layer_index == index {
            self.active_edit_target = LayerEditTarget::LayerPixels;
        }
        true
    }

    pub fn set_layer_mask_enabled(&mut self, index: usize, enabled: bool) -> bool {
        let Some(mask) = self.layer_mask_mut(index) else {
            return false;
        };

        mask.enabled = enabled;
        true
    }

    pub fn duplicate_layer(&mut self, index: usize) -> Option<LayerId> {
        let source = self.layers.get(index)?.clone();
        let source_id = source.id;
        let duplicate_id = LayerId::new();
        let duplicate_name = format!("{} copy", source.name);

        let duplicate = RasterLayer {
            id: duplicate_id,
            name: duplicate_name,
            visible: source.visible,
            opacity_percent: source.opacity_percent,
            blend_mode: source.blend_mode,
            mask: source.mask.map(|mask| RasterMask {
                enabled: mask.enabled,
                tiles: mask.tiles,
                dirty_tiles: HashSet::new(),
            }),
            offset_x: source.offset_x,
            offset_y: source.offset_y,
            tiles: source.tiles,
            dirty_tiles: HashSet::new(),
        };

        self.layers.insert(index + 1, duplicate);
        if self.group_count() == 0 {
            self.insert_flat_layer_after(source_id, duplicate_id);
        }
        self.active_layer_index = index + 1;
        Some(duplicate_id)
    }

    pub fn layer_state_snapshot(&self, index: usize) -> Option<LayerStateSnapshot> {
        let layer = self.layers.get(index)?;
        Some(LayerStateSnapshot {
            offset_x: layer.offset_x,
            offset_y: layer.offset_y,
            tiles: layer.tiles.clone(),
            mask: layer.mask.clone(),
        })
    }

    pub fn apply_layer_state_snapshot(
        &mut self,
        layer_id: LayerId,
        snapshot: LayerStateSnapshot,
    ) -> bool {
        let Some(layer_index) = self.layer_index_by_id(layer_id) else {
            return false;
        };

        let Some(layer) = self.layers.get_mut(layer_index) else {
            return false;
        };

        layer.offset_x = snapshot.offset_x;
        layer.offset_y = snapshot.offset_y;
        layer.tiles = snapshot.tiles;
        layer.dirty_tiles = layer.tiles.keys().copied().collect();
        layer.mask = snapshot.mask;
        if let Some(mask) = layer.mask.as_mut() {
            mask.dirty_tiles = mask.tiles.keys().copied().collect();
        }
        true
    }

    pub fn ensure_tile_for_pixel(
        &mut self,
        layer_index: usize,
        pixel_x: u32,
        pixel_y: u32,
    ) -> Option<&mut RasterTile> {
        let coord = self.tile_coord_for_pixel(pixel_x, pixel_y)?;
        let tile_size = self.tile_size;
        let layer = self.layers.get_mut(layer_index)?;
        Some(layer.ensure_tile(coord, tile_size))
    }

    pub fn ensure_mask_tile_for_pixel(
        &mut self,
        layer_index: usize,
        pixel_x: u32,
        pixel_y: u32,
    ) -> Option<&mut MaskTile> {
        let coord = self.tile_coord_for_pixel(pixel_x, pixel_y)?;
        let tile_size = self.tile_size;
        let mask = self.layer_mask_mut(layer_index)?;
        Some(mask.ensure_tile(coord, tile_size))
    }

    pub fn dirty_tiles(&self, layer_index: usize) -> Option<&HashSet<TileCoord>> {
        Some(&self.layers.get(layer_index)?.dirty_tiles)
    }

    pub fn tile_snapshot(&self, layer_index: usize, coord: TileCoord) -> Option<RasterTile> {
        self.layers.get(layer_index)?.tiles.get(&coord).cloned()
    }

    pub fn mask_tile_snapshot(&self, layer_index: usize, coord: TileCoord) -> Option<MaskTile> {
        self.layers
            .get(layer_index)?
            .mask
            .as_ref()?
            .tiles
            .get(&coord)
            .cloned()
    }

    pub fn apply_tile_snapshot(
        &mut self,
        layer_id: LayerId,
        coord: TileCoord,
        tile: Option<RasterTile>,
    ) -> bool {
        let Some(layer_index) = self.layer_index_by_id(layer_id) else {
            return false;
        };

        let Some(layer) = self.layers.get_mut(layer_index) else {
            return false;
        };

        match tile {
            Some(tile) => {
                layer.tiles.insert(coord, tile);
                layer.mark_tile_dirty(coord);
            }
            None => {
                layer.tiles.remove(&coord);
                layer.mark_tile_dirty(coord);
            }
        }

        true
    }

    pub fn apply_mask_tile_snapshot(
        &mut self,
        layer_id: LayerId,
        coord: TileCoord,
        tile: Option<MaskTile>,
    ) -> bool {
        let Some(layer_index) = self.layer_index_by_id(layer_id) else {
            return false;
        };

        let Some(layer) = self.layers.get_mut(layer_index) else {
            return false;
        };

        let Some(mask) = layer.mask.as_mut() else {
            return false;
        };

        match tile {
            Some(tile) => {
                mask.tiles.insert(coord, tile);
                mask.mark_tile_dirty(coord);
            }
            None => {
                mask.tiles.remove(&coord);
                mask.mark_tile_dirty(coord);
            }
        }

        true
    }

    pub fn rename_layer(&mut self, index: usize, name: impl Into<String>) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.name = name.into();
        }
    }

    pub fn set_layer_visibility(&mut self, index: usize, visible: bool) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.visible = visible;
        }
    }

    pub fn set_layer_opacity(&mut self, index: usize, opacity_percent: u8) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.opacity_percent = opacity_percent.min(100);
        }
    }

    pub fn set_layer_blend_mode(&mut self, index: usize, blend_mode: BlendMode) {
        if let Some(layer) = self.layers.get_mut(index) {
            layer.blend_mode = blend_mode;
        }
    }

    pub fn set_layer_offset(&mut self, index: usize, offset_x: i32, offset_y: i32) -> bool {
        let Some(layer) = self.layers.get_mut(index) else {
            return false;
        };

        layer.offset_x = offset_x;
        layer.offset_y = offset_y;
        true
    }

    pub fn translate_layer(&mut self, index: usize, delta_x: i32, delta_y: i32) -> bool {
        let Some(layer) = self.layers.get_mut(index) else {
            return false;
        };

        layer.offset_x += delta_x;
        layer.offset_y += delta_y;
        true
    }

    pub fn move_layer(&mut self, from_index: usize, to_index: usize) -> bool {
        if from_index >= self.layers.len()
            || to_index >= self.layers.len()
            || from_index == to_index
        {
            return false;
        }

        if self.group_count() != 0 {
            return false;
        }

        let layer = self.layers.remove(from_index);
        self.layers.insert(to_index, layer);
        self.rebuild_flat_layer_hierarchy();
        self.active_layer_index = to_index;
        true
    }

    pub fn delete_layer(&mut self, index: usize) -> bool {
        if self.layers.len() <= 1 || index >= self.layers.len() {
            return false;
        }

        let deleted_layer_id = self.layers[index].id;
        self.layers.remove(index);
        if self.group_count() == 0 {
            self.layer_hierarchy.retain(
                |node| !matches!(node, LayerHierarchyNode::Layer(layer_id) if *layer_id == deleted_layer_id),
            );
            self.rebuild_flat_layer_hierarchy();
        }
        if self.active_layer_index >= self.layers.len() {
            self.active_layer_index = self.layers.len() - 1;
        }
        if self.active_edit_target == LayerEditTarget::LayerMask
            && self.layers[self.active_layer_index].mask.is_none()
        {
            self.active_edit_target = LayerEditTarget::LayerPixels;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlendMode, Document, FreeformSelection, Guide, LayerEditTarget, LayerHierarchyNode,
        LayerHierarchyNodeRef, LayerKind, RasterTile, SelectionPoint, SelectionShape,
        TextAlignment, TextStyle, TextTransform, TileCoord,
    };
    use common::CanvasRect;

    #[test]
    fn new_document_starts_with_background_layer() {
        let document = Document::new(1920, 1080);

        assert_eq!(document.canvas_size.width, 1920);
        assert_eq!(document.canvas_size.height, 1080);
        assert_eq!(document.layer_count(), 1);
        assert_eq!(document.active_layer().name, "Background");
        assert_eq!(document.active_layer().blend_mode, BlendMode::Normal);
        assert_eq!(document.active_edit_target(), LayerEditTarget::LayerPixels);
        assert!(document.layer_mask(0).is_none());
        assert_eq!(document.tile_grid_size().columns, 8);
        assert_eq!(document.tile_grid_size().rows, 5);
        assert!(document.validate_layer_hierarchy().is_ok());
        assert_eq!(
            document.layer_hierarchy(),
            &[LayerHierarchyNode::Layer(document.active_layer().id)]
        );
    }

    #[test]
    fn layer_masks_can_be_added_targeted_and_removed() {
        let mut document = Document::new(640, 480);

        assert!(document.add_layer_mask(0));
        assert!(document.layer_mask(0).is_some());
        assert!(document.layer_mask(0).expect("mask exists").enabled);
        assert!(document.set_active_edit_target(LayerEditTarget::LayerMask));
        assert_eq!(document.active_edit_target(), LayerEditTarget::LayerMask);

        assert!(document.set_layer_mask_enabled(0, false));
        assert!(!document.layer_mask(0).expect("mask exists").enabled);

        assert!(document.remove_layer_mask(0));
        assert!(document.layer_mask(0).is_none());
        assert_eq!(document.active_edit_target(), LayerEditTarget::LayerPixels);
    }

    #[test]
    fn layer_mask_target_requires_mask_presence() {
        let mut document = Document::new(640, 480);

        assert!(!document.set_active_edit_target(LayerEditTarget::LayerMask));
        assert_eq!(document.active_edit_target(), LayerEditTarget::LayerPixels);
    }

    #[test]
    fn add_layer_inserts_after_active_layer() {
        let mut document = Document::new(640, 480);

        document.add_layer("Sketch");

        assert_eq!(document.layer_count(), 2);
        assert_eq!(document.active_layer_index, 1);
        assert_eq!(document.active_layer().name, "Sketch");
        assert!(document.validate_layer_hierarchy().is_ok());
    }

    #[test]
    fn text_layers_are_document_owned_and_hierarchy_addressable() {
        let mut document = Document::new(640, 480);

        let text_layer_id = document.add_text_layer(
            "Title",
            "PhotoTux",
            TextTransform::new(120, 80),
        );
        let text_index = document
            .text_layer_index_by_id(text_layer_id)
            .expect("text layer should be tracked");
        let text_layer = document
            .text_layer(text_index)
            .expect("text layer should be accessible");

        assert_eq!(document.text_layer_count(), 1);
        assert_eq!(document.total_layer_count(), 2);
        assert_eq!(document.layer_kind_by_id(text_layer_id), Some(LayerKind::Text));
        assert_eq!(text_layer.name, "Title");
        assert_eq!(text_layer.content, "PhotoTux");
        assert_eq!(text_layer.transform, TextTransform::new(120, 80));
        assert_eq!(text_layer.style.alignment, TextAlignment::Left);
        assert!(document.has_visible_text_layers());
        assert!(document.validate_layer_hierarchy().is_ok());
        assert!(document
            .layer_hierarchy()
            .iter()
            .any(|node| matches!(node, LayerHierarchyNode::Layer(layer_id) if *layer_id == text_layer_id)));
    }

    #[test]
    fn text_layers_can_be_grouped_like_other_layer_nodes() {
        let mut document = Document::new(640, 480);
        let text_layer_id = document.add_text_layer(
            "Caption",
            "Editable",
            TextTransform::new(12, 18),
        );

        let group_id = document
            .wrap_hierarchy_node_in_group(LayerHierarchyNodeRef::Layer(text_layer_id), "Type")
            .expect("text layer should be wrappable");

        assert_eq!(document.group_for_layer(text_layer_id), Some(group_id));
        assert!(document.validate_layer_hierarchy().is_ok());
    }

    #[test]
    fn text_style_updates_can_be_applied_headlessly() {
        let mut document = Document::new(640, 480);
        let text_layer_id = document.add_text_layer(
            "Label",
            "Draft",
            TextTransform::new(20, 32),
        );
        let text_layer = document
            .text_layer_mut(
                document
                    .text_layer_index_by_id(text_layer_id)
                    .expect("text layer should exist"),
            )
            .expect("text layer should be mutable");
        text_layer.style = TextStyle {
            font_family: "Noto Sans".to_string(),
            font_size_px: 64,
            line_height_percent: 140,
            letter_spacing: 12,
            fill_rgba: [200, 210, 220, 255],
            alignment: TextAlignment::Center,
        };

        assert_eq!(text_layer.style.font_family, "Noto Sans");
        assert_eq!(text_layer.style.font_size_px, 64);
        assert_eq!(text_layer.style.line_height_percent, 140);
        assert_eq!(text_layer.style.letter_spacing, 12);
        assert_eq!(text_layer.style.fill_rgba, [200, 210, 220, 255]);
        assert_eq!(text_layer.style.alignment, TextAlignment::Center);
    }

    #[test]
    fn create_layer_group_wraps_top_level_layers_in_order() {
        let mut document = Document::new(640, 480);
        document.add_layer("Sketch");
        document.add_layer("Highlights");

        let background_id = document.layers[0].id;
        let sketch_id = document.layers[1].id;
        let highlights_id = document.layers[2].id;

        let group_id = document
            .create_layer_group("Paint Stack", &[sketch_id, highlights_id])
            .expect("group creation should succeed");

        assert_eq!(document.group_count(), 1);
        assert!(document.validate_layer_hierarchy().is_ok());

        let [
            LayerHierarchyNode::Layer(layer_id),
            LayerHierarchyNode::Group(group),
        ] = document.layer_hierarchy()
        else {
            panic!("expected one top-level layer followed by one top-level group");
        };

        assert_eq!(*layer_id, background_id);
        assert_eq!(group.id, group_id);
        assert_eq!(group.name, "Paint Stack");
        assert_eq!(group.children.len(), 2);
        assert_eq!(group.children[0], LayerHierarchyNode::Layer(sketch_id));
        assert_eq!(group.children[1], LayerHierarchyNode::Layer(highlights_id));
    }

    #[test]
    fn create_layer_group_rejects_duplicate_or_missing_layer_ids() {
        let mut document = Document::new(640, 480);
        document.add_layer("Sketch");

        let background_id = document.layers[0].id;
        let sketch_id = document.layers[1].id;

        assert!(
            document
                .create_layer_group("Invalid", &[background_id, background_id])
                .is_none()
        );
        assert!(
            document
                .create_layer_group("Valid", &[background_id, sketch_id])
                .is_some()
        );
        assert!(
            document
                .create_layer_group("Nested Duplicate", &[background_id])
                .is_none()
        );
    }

    #[test]
    fn wrap_node_in_group_and_ungroup_roundtrip_hierarchy() {
        let mut document = Document::new(640, 480);
        document.add_layer("Sketch");
        let sketch_id = document.layers[1].id;

        let group_id = document
            .wrap_hierarchy_node_in_group(LayerHierarchyNodeRef::Layer(sketch_id), "Sketch Group")
            .expect("wrapping a top-level layer should succeed");
        assert_eq!(document.group_count(), 1);
        assert!(document.group(group_id).is_some());

        assert!(document.ungroup(group_id));
        assert_eq!(document.group_count(), 0);
        assert!(document.validate_layer_hierarchy().is_ok());
    }

    #[test]
    fn move_layer_into_group_and_back_out_preserves_hierarchy() {
        let mut document = Document::new(640, 480);
        document.add_layer("Sketch");
        document.add_layer("Highlights");

        let sketch_id = document.layers[1].id;
        let highlights_id = document.layers[2].id;
        let group_id = document
            .wrap_hierarchy_node_in_group(
                LayerHierarchyNodeRef::Layer(highlights_id),
                "Highlights Group",
            )
            .expect("wrapping highlights should create a group");

        assert!(document.move_node_into_group(LayerHierarchyNodeRef::Layer(sketch_id), group_id));
        assert_eq!(document.group_for_layer(sketch_id), Some(group_id));
        assert!(document.validate_layer_hierarchy().is_ok());

        assert!(document.move_node_out_of_group(LayerHierarchyNodeRef::Layer(sketch_id)));
        assert_eq!(document.group_for_layer(sketch_id), None);
        assert!(document.validate_layer_hierarchy().is_ok());
    }

    #[test]
    fn set_group_visibility_updates_stored_group_state() {
        let mut document = Document::new(640, 480);
        let background_id = document.layers[0].id;
        let group_id = document
            .wrap_hierarchy_node_in_group(
                LayerHierarchyNodeRef::Layer(background_id),
                "Background Group",
            )
            .expect("wrapping background should succeed");

        assert!(document.set_group_visibility(group_id, false));
        assert_eq!(
            document.group(group_id).map(|group| group.visible),
            Some(false)
        );
    }

    #[test]
    fn duplicate_layer_clones_tiles_and_activates_copy() {
        let mut document = Document::new(512, 512);
        document.rename_layer(0, "Paint");
        let tile = document
            .ensure_tile_for_pixel(0, 25, 25)
            .expect("tile should be created");
        tile.pixels[0] = 120;
        tile.pixels[3] = 255;
        assert!(document.add_layer_mask(0));
        let tile_size = document.tile_size;
        let mask = document.layer_mask_mut(0).expect("mask exists");
        let mask_tile = mask.ensure_tile(TileCoord::new(0, 0), tile_size);
        mask_tile.alpha[0] = 64;

        let duplicate_id = document
            .duplicate_layer(0)
            .expect("layer duplication should succeed");

        assert_eq!(document.layer_count(), 2);
        assert_eq!(document.active_layer_index(), 1);
        assert_eq!(document.layers[1].name, "Paint copy");
        assert_ne!(document.layers[0].id, duplicate_id);
        assert_eq!(document.layers[1].id, duplicate_id);
        assert_eq!(document.layers[1].tiles, document.layers[0].tiles);
        assert_eq!(
            document.layers[1].mask.as_ref().map(|mask| mask.enabled),
            document.layers[0].mask.as_ref().map(|mask| mask.enabled)
        );
        assert_eq!(
            document.layers[1].mask.as_ref().map(|mask| &mask.tiles),
            document.layers[0].mask.as_ref().map(|mask| &mask.tiles)
        );
    }

    #[test]
    fn set_active_layer_rejects_invalid_indices() {
        let mut document = Document::new(320, 240);
        document.add_layer("Top");

        assert!(document.set_active_layer(0));
        assert_eq!(document.active_layer_index(), 0);
        assert!(!document.set_active_layer(99));
        assert_eq!(document.active_layer_index(), 0);
    }

    #[test]
    fn move_layer_reorders_layers() {
        let mut document = Document::new(640, 480);
        document.add_layer("Sketch");
        document.add_layer("Highlights");

        let moved = document.move_layer(2, 0);

        assert!(moved);
        assert_eq!(document.layers[0].name, "Highlights");
        assert_eq!(document.active_layer_index, 0);
        assert!(document.validate_layer_hierarchy().is_ok());
    }

    #[test]
    fn delete_layer_keeps_at_least_one_layer() {
        let mut document = Document::new(640, 480);

        assert!(!document.delete_layer(0));

        document.add_layer("Sketch");
        assert!(document.delete_layer(1));
        assert_eq!(document.layer_count(), 1);
    }

    #[test]
    fn layer_property_updates_are_clamped_and_applied() {
        let mut document = Document::new(640, 480);

        document.rename_layer(0, "Base");
        document.set_layer_visibility(0, false);
        document.set_layer_opacity(0, 255);
        document.set_layer_blend_mode(0, BlendMode::Screen);

        assert_eq!(document.layers[0].name, "Base");
        assert!(!document.layers[0].visible);
        assert_eq!(document.layers[0].opacity_percent, 100);
        assert_eq!(document.layers[0].blend_mode, BlendMode::Screen);
    }

    #[test]
    fn layer_offset_updates_and_translates() {
        let mut document = Document::new(640, 480);

        assert!(document.set_layer_offset(0, 12, -8));
        assert_eq!(document.layer_offset(0), Some((12, -8)));

        assert!(document.translate_layer(0, 5, 10));
        assert_eq!(document.layer_offset(0), Some((17, 2)));
    }

    #[test]
    fn layer_canvas_bounds_include_tile_region_and_offset() {
        let mut document = Document::new(1024, 1024);
        let _ = document.ensure_tile_for_pixel(0, 300, 20);
        let _ = document.ensure_tile_for_pixel(0, 700, 500);
        assert!(document.set_layer_offset(0, 5, -3));

        let bounds = document
            .layer_canvas_bounds(0)
            .expect("bounds should exist");

        assert_eq!(bounds, CanvasRect::new(261, -3, 512, 512));
    }

    #[test]
    fn selection_can_be_set_and_cleared() {
        let mut document = Document::new(320, 240);
        let selection = CanvasRect::new(10, 20, 30, 40);

        document.set_selection(selection);
        assert_eq!(document.selection(), Some(selection));
        assert!(!document.selection_inverted());

        document.clear_selection();
        assert_eq!(document.selection(), None);
        assert!(!document.selection_inverted());
    }

    #[test]
    fn selection_can_be_inverted() {
        let mut document = Document::new(320, 240);
        document.set_selection(CanvasRect::new(5, 6, 7, 8));

        assert!(document.invert_selection());
        assert!(document.selection_inverted());

        assert!(document.invert_selection());
        assert!(!document.selection_inverted());
    }

    #[test]
    fn guides_can_be_added_removed_and_toggled() {
        let mut document = Document::new(320, 240);

        document.add_guide(Guide::horizontal(40));
        document.add_guide(Guide::vertical(60));

        assert_eq!(
            document.guides(),
            &[Guide::horizontal(40), Guide::vertical(60)]
        );
        assert!(document.guides_visible());

        document.toggle_guides_visible();
        assert!(!document.guides_visible());

        assert_eq!(document.remove_last_guide(), Some(Guide::vertical(60)));
        assert_eq!(document.guides(), &[Guide::horizontal(40)]);
    }

    #[test]
    fn freeform_selection_reports_bounds_and_shape() {
        let mut document = Document::new(320, 240);
        let freeform = FreeformSelection::new(vec![
            SelectionPoint::new(12, 14),
            SelectionPoint::new(30, 10),
            SelectionPoint::new(24, 36),
        ])
        .expect("three points should define a freeform selection");

        document.set_freeform_selection(freeform.clone());

        assert_eq!(document.selection(), Some(CanvasRect::new(12, 10, 19, 27)));
        assert_eq!(
            document.selection_shape(),
            Some(&SelectionShape::Freeform(freeform))
        );
    }

    #[test]
    fn freeform_selection_contains_points_using_polygon_fill_rule() {
        let mut document = Document::new(320, 240);
        let freeform = FreeformSelection::new(vec![
            SelectionPoint::new(10, 10),
            SelectionPoint::new(40, 10),
            SelectionPoint::new(25, 40),
        ])
        .expect("triangle selection should be valid");

        document.set_freeform_selection(freeform);

        assert!(document.selection_contains_pixel(25, 20));
        assert!(!document.selection_contains_pixel(5, 5));
        assert!(!document.selection_contains_pixel(25, 45));
    }

    #[test]
    fn freeform_selection_respects_inverted_edit_queries() {
        let mut document = Document::new(320, 240);
        let freeform = FreeformSelection::new(vec![
            SelectionPoint::new(10, 10),
            SelectionPoint::new(30, 10),
            SelectionPoint::new(30, 30),
            SelectionPoint::new(10, 30),
        ])
        .expect("quad selection should be valid");

        document.set_freeform_selection(freeform);
        assert!(document.allows_pixel_edit(20, 20));
        assert!(!document.allows_pixel_edit(5, 5));

        assert!(document.invert_selection());
        assert!(!document.allows_pixel_edit(20, 20));
        assert!(document.allows_pixel_edit(5, 5));
    }

    #[test]
    fn selection_pixel_tests_use_exclusive_bottom_right_edge() {
        let mut document = Document::new(320, 240);
        document.set_selection(CanvasRect::new(10, 20, 30, 40));

        assert!(document.selection_contains_pixel(10, 20));
        assert!(document.selection_contains_pixel(39, 59));
        assert!(!document.selection_contains_pixel(40, 59));
        assert!(!document.selection_contains_pixel(39, 60));
    }

    #[test]
    fn allows_pixel_edit_respects_normal_and_inverted_selection() {
        let mut document = Document::new(320, 240);

        assert!(document.allows_pixel_edit(2, 3));

        document.set_selection(CanvasRect::new(10, 20, 30, 40));
        assert!(document.allows_pixel_edit(20, 30));
        assert!(!document.allows_pixel_edit(2, 3));

        assert!(document.invert_selection());
        assert!(!document.allows_pixel_edit(20, 30));
        assert!(document.allows_pixel_edit(2, 3));
    }

    #[test]
    fn tile_coord_for_pixel_maps_pixels_to_tile_grid() {
        let document = Document::new(1024, 1024);

        assert_eq!(
            document.tile_coord_for_pixel(0, 0),
            Some(TileCoord::new(0, 0))
        );
        assert_eq!(
            document.tile_coord_for_pixel(255, 255),
            Some(TileCoord::new(0, 0))
        );
        assert_eq!(
            document.tile_coord_for_pixel(256, 256),
            Some(TileCoord::new(1, 1))
        );
        assert_eq!(document.tile_coord_for_pixel(1024, 0), None);
    }

    #[test]
    fn ensure_tile_for_pixel_creates_and_marks_dirty_tile() {
        let mut document = Document::new(512, 512);

        let tile = document
            .ensure_tile_for_pixel(0, 300, 20)
            .expect("tile should exist for a valid layer and pixel");

        assert_eq!(tile.pixels.len(), 256 * 256 * 4);
        assert!(
            document
                .dirty_tiles(0)
                .expect("layer should exist")
                .contains(&TileCoord::new(1, 0))
        );
    }

    #[test]
    fn take_dirty_tiles_returns_sorted_coordinates() {
        let mut document = Document::new(512, 512);
        let layer = &mut document.layers[0];
        layer.mark_tile_dirty(TileCoord::new(1, 1));
        layer.mark_tile_dirty(TileCoord::new(0, 0));
        layer.mark_tile_dirty(TileCoord::new(1, 0));

        let dirty_tiles = layer.take_dirty_tiles();

        assert_eq!(
            dirty_tiles,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(1, 1)
            ]
        );
        assert!(layer.dirty_tiles.is_empty());
    }

    #[test]
    fn tile_coords_in_radius_returns_touched_tiles() {
        let document = Document::new(600, 600);

        let coords = document.tile_coords_in_radius(260.0, 260.0, 40.0);

        assert_eq!(
            coords,
            vec![
                TileCoord::new(0, 0),
                TileCoord::new(1, 0),
                TileCoord::new(0, 1),
                TileCoord::new(1, 1),
            ]
        );
    }

    #[test]
    fn apply_tile_snapshot_restores_tile_presence() {
        let mut document = Document::new(512, 512);
        let layer_id = document.layers[0].id;
        let coord = TileCoord::new(1, 1);
        let tile = RasterTile::new(document.tile_size);

        assert!(document.apply_tile_snapshot(layer_id, coord, Some(tile.clone())));
        assert!(document.tile_snapshot(0, coord).is_some());
        assert!(document.apply_tile_snapshot(layer_id, coord, None));
        assert!(document.tile_snapshot(0, coord).is_none());
    }

    #[test]
    fn layer_state_snapshot_roundtrips_tiles_and_offsets() {
        let mut document = Document::new(512, 512);
        assert!(document.set_layer_offset(0, 12, -9));
        let tile = document
            .ensure_tile_for_pixel(0, 20, 20)
            .expect("tile should be created");
        tile.pixels[0] = 180;
        tile.pixels[3] = 255;

        let snapshot = document
            .layer_state_snapshot(0)
            .expect("snapshot should exist");
        let layer_id = document.layer(0).expect("layer exists").id;
        document.set_layer_offset(0, 0, 0);
        document.layer_mut(0).expect("layer exists").tiles.clear();

        assert!(document.apply_layer_state_snapshot(layer_id, snapshot.clone()));
        assert_eq!(document.layer_offset(0), Some((12, -9)));
        assert_eq!(document.layer_state_snapshot(0), Some(snapshot));
    }
}
