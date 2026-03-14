# PhotoTux UI Layout Specification
Version: 1.0
Target: Linux desktop
Stack: Rust + GTK4 + wgpu

---

## 1. Layout Goals

The interface must feel like a professional image editor:
- dense but readable
- keyboard-friendly
- panel-based
- canvas-first
- dark themed
- efficient on 1440p and 4K displays
- familiar to users of established raster-editor workspaces without cloning any vendor-specific visual identity

The layout should support:
- one or more open documents
- tool-specific options
- complex layer management
- contextual editing controls
- later workspace presets without destabilizing the fixed MVP shell

Scope note: only the fixed-layout shell is part of the MVP direction. Docking, floating panels, and persistent workspace systems remain later-phase features unless the core roadmap changes.

---

## 2. Global Window Structure

The main window is composed of these vertical regions:

1. Native titlebar or GTK header bar
2. Menu bar
3. Tool options bar
4. Main workspace body
5. Status bar

### Main Workspace Body
The body is composed of three horizontal columns:

1. Left tool rail
2. Central document region
3. Right dock region

### Primary Wireframe

```text
┌────────────────────────────────────────────────────────────────────────────────────┐
│ Titlebar / Window Controls / App Name / Workspace Preset / Search                │
├────────────────────────────────────────────────────────────────────────────────────┤
│ Menu Bar: File Edit Image Layer Select Filter View Window Help                   │
├────────────────────────────────────────────────────────────────────────────────────┤
│ Tool Options Bar                                                                 │
├──────────┬──────────────────────────────────────────────────────┬──────────────────┤
│ Tools    │ Document Tabs                                        │ Panel Dock Icons │
│ Bar      ├──────────────────────────────────────────────────────┼──────────────────┤
│          │ Rulers / Document Window / Canvas / Pasteboard       │ Color / Swatches │
│          │                                                      ├──────────────────┤
│          │                                                      │ Props / Adjust   │
│          │                                                      ├──────────────────┤
│          │                                                      │ Layers / Paths   │
├──────────┴──────────────────────────────────────────────────────┴──────────────────┤
│ Status Bar                                                                        │
└────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Default Window Metrics

### Default App Size
- Minimum: 1280 × 800
- Recommended: 1600 × 900
- Ideal working size: 1720 × 980+
- Must scale well to 2560 × 1440 and 4K

### Regional Size Targets
- Titlebar height: 28px
- Menu bar height: 24px
- Tool options bar height: 36px
- Status bar height: 20px
- Left tool rail width: 44px
- Right dock width: 312px
- Document tab strip height: 28px

### Adaptive Rules
- If window width is under 1400px, right dock compresses to 300px
- If window width is under 1220px, lower-priority right panels should compress or collapse; tab conversion is a later enhancement
- If window width is under 1080px, optional later panels such as Swatches should hide by default if enabled
- The canvas must always remain the visual center

---

## 4. Region-by-Region Specification

## 4.1 Titlebar

### Purpose
Provide app identity, workspace preset access, quick search, and window controls while leaving document-specific chrome to the document region.

### Window Framing
- The early shell should prefer native GTK window behavior or a GTK header bar with native-feeling controls
- Custom window chrome is later scope and should not delay editing-core milestones
- Drag, maximize, and window-state behavior should feel native on Linux before any deeper shell customization

### Layout
- Left: app icon + app name
- Center: spacer or optional global status
- Right: workspace preset selector, search button, window controls

If workspace presets are added later, their switcher can appear in this region.

### Structure
```text
[AppIcon] PhotoTux                               [Essentials v] [⌕] [_][□][×]
```

### Behavior
- Double-click titlebar toggles maximize
- Search opens command palette
- Workspace preset switching may appear here as a compact selector even before full workspace persistence exists

---

## 4.2 Menu Bar

### Menus
- File
- Edit
- Image
- Layer
- Select
- Filter
- View
- Window
- Help

### Rules
- Full keyboard navigation
- Mnemonics enabled on Linux
- Every command accessible somewhere in the menu system
- Menus should remain text-based, not icon-heavy

### Illustrative Menu Model
```text
<MenuBar>
  <Menu title="File"/>
  <Menu title="Edit"/>
  <Menu title="Image"/>
  <Menu title="Layer"/>
  <Menu title="Select"/>
  <Menu title="Filter"/>
  <Menu title="View"/>
  <Menu title="Window"/>
  <Menu title="Help"/>
</MenuBar>
```

---

## 4.3 Tool Options Bar

### Purpose
Display context-sensitive settings for the active tool.

### Layout Zones
- Left zone: active tool label + tool icon + preset selector
- Center zone: dense, context-sensitive controls matching the current tool family
- Right zone: compact mode controls, optional help/context chip, reset

### Dimensions
- Height: 36px
- Horizontal padding: 6px
- Inter-control gap: 4-6px

### Example Control Sets

#### Move Tool
- Auto-select
- Show transform controls
- Snap
- Align later
- Distribute later

#### Brush Tool
- Preset
- Size
- Hardness
- Opacity
- Flow
- Smoothing
- Blend mode

#### Selection Tools
- Feather
- Anti-alias
- Add later
- Subtract later
- Intersect later
- Fixed ratio later

### Illustrative Structure
```text
<ToolOptionsBar>
  <ToolLabel text="Brush Tool"/>
  <CompactPresetChip id="brush-presets"/>
  <CompactToggle id="mode"/>
  <CompactToggle id="transform-controls"/>
  <Stepper id="size"/>
  <Stepper id="hardness"/>
  <Stepper id="opacity"/>
  <Stepper id="flow"/>
  <CompactInfoChip id="context-mode"/>
  <IconButton id="reset-tool"/>
</ToolOptionsBar>
```

---

## 4.4 Left Tool Rail

### Purpose
Provide primary tool switching with minimal travel distance.

### Width
- Fixed: 44px in the compact single-column shell

### Internal Layout
- Vertical stack of icon-first tool buttons
- Tool families visually separated by thin dividers
- Utility controls pinned to bottom
- Scroll-free by default

### Tool Button Size
- 30 × 30px
- 4px vertical gap
- icon-first presentation with tooltip or flyout label

### Tool Order
Group A: Move, Marquee, Lasso

Group B: Quick Selection, Crop, Eyedropper

Group C: Brush, Clone Stamp, History Brush

Group D: Eraser, Gradient/Paint Bucket

Group E: Blur/Sharpen/Smudge, Dodge/Burn/Sponge

Group F: Pen, Type, Path Selection, Shape, Hand, Zoom

Only one representative button per family is required in the early fixed shell; nested tools may be shown later via flyout affordances.

### Bottom Cluster
- Foreground/background color chips
- Swap colors
- Reset colors
- Quick mask placeholder or compact toggle
- Screen mode placeholder or compact toggle

### Layout Example
```text
┌────┐
│Mv  │
│Mq  │
│Ls  │
│----│
│Sel │
│Crp │
│Eye │
│----│
│Br  │
│Cln │
│His │
│----│
│Er  │
│Grd │
│----│
│Pen │
│Txt │
│Pth │
│Shp │
│Hnd │
│Zum │
│----│
│FGBG│
└────┘
```

### Illustrative Structure
```text
<LeftToolRail width="60">
  <ToolButton id="move"/>
  <ToolButton id="marquee"/>
  <ToolButton id="brush"/>
  <ToolButton id="eraser"/>
  <ToolButton id="hand"/>
  <ToolButton id="zoom"/>
  <Spacer/>
  <ColorChipPair/>
  <IconButton id="swap-colors"/>
  <IconButton id="default-colors"/>
  <!-- quick-mask later -->
  <!-- screen-mode later -->
</LeftToolRail>
```

---

## 4.5 Center Document Region

The center region is the visual anchor of the application.

### Internal Vertical Layout
1. Document tabs
2. Document ruler/header strip when enabled
3. Canvas viewport
4. Optional contextual overlay
5. Status bar is external to document region

### Width Behavior
- Takes all remaining space after left rail and right dock
- Must never collapse below 600px visible canvas width
- If right dock is too wide, canvas wins

---

## 4.6 Document Tabs

### Height
- 34px

### Tab Metrics
- Min width: 120px
- Preferred width: 180px
- Tab text should include document name and may include zoom and color mode in compact metadata form

### Behavior
- Tabs feel like document-window headers rather than browser pills
- Active tab has a stronger border and background separation from the canvas surround
- Close affordance remains on the tab itself
- Multi-document behavior is future work, but the layout should already reserve for it

## 4.7 Document Window

### Framing
- The document window is visually distinct from the pasteboard and should feel like a bounded editing surface within a darker workspace
- Horizontal and vertical ruler rails should be supported in the shell layout even if they begin as visual placeholders
- The status readout for zoom and dimensions should anchor to the lower-left of the document region or status bar

### Canvas Behavior
- The pasteboard fills unused space around the canvas
- Canvas centering is preserved by default
- Hand and Zoom tools are treated as navigation tools, not content-editing panels
- Decorative ruler rails may be present before full ruler interaction lands, as long as the document window framing is preserved

## 4.8 Right Dock Region

### Purpose
Provide dense, tabbed inspection and editing surfaces grouped in a familiar pro-editor dock structure.

### Structure
- Far-left narrow icon rail for collapsed panel-group shortcuts
- Main dock column composed of vertically stacked panel groups
- Panel groups use tabs rather than independent card headers whenever the content belongs to a shared region

### Default Panel Groups
Top group: Color | Swatches | Gradients

Middle group: Properties | Adjustments | History

Bottom group: Layers | Channels | Paths

### Rules
- Layers remains the lowest and largest persistent group by default
- Panel content density is higher than dialogs or settings surfaces
- Collapse and drag behavior is later scope; visual grouping is required now
- Group tabs should read like professional utility panels rather than app navigation
- Tab clicks should drive visible state changes instead of static active styling only
- Max width: 240px
- Close icon zone: 22px
- Internal horizontal padding: 10px

### Tab Content
- Thumbnail optional later
- File name
- Dirty dot
- Close button

### Behavior
- Drag to reorder
- Middle click closes
- Overflow becomes scrollable
- Plus button optionally opens a new document dialog

### Example
```text
[ banner-edit.ptx ● ] [ texture-test.png ] [ icon-sheet.ptx ] [+]
```

---

## 4.7 Canvas Viewport

### Purpose
Display and edit the active document.

### Viewport Layers
1. Canvas surround
2. Rulers optional
3. Checkerboard transparency
4. Image content
5. Selection overlays
6. Transform handles
7. Guide/grid overlays
8. Brush cursor preview
9. Contextual action strip

### Canvas Surround
- Darker than panels
- Neutral and low-distraction
- No gradients by default

### Content Alignment
- Center document when opening
- Preserve user zoom/pan during edit session
- Zoom around cursor position
- Pan with spacebar drag
- Double-click hand tool fits canvas
- 100% view centers active document

### Pixel Grid
- Hidden below 800%
- Visible at 800%+
- Thin, low-contrast lines

### Rulers
- Optional in MVP
- 20px thickness if enabled
- Origin can be dragged later

### Canvas Example
```text
┌──────────────────────────────────────────────────────────────┐
│ Tabs                                                         │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│                dark surround / working space                 │
│           ┌───────────────────────────────────────┐          │
│           │ checkerboard + image + overlays       │          │
│           │                                       │          │
│           │               active doc              │          │
│           │                                       │          │
│           └───────────────────────────────────────┘          │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## 4.8 Right Dock Region

### Purpose
Hold stacked inspector panels for the fixed-layout shell.

### Default Width
- 336px

### Dock Layout
- Vertical stack of panels
- Resize handles between panel groups
- Panels may be collapsed
- Panels may later support tabs

### Default Panel Stack
1. Properties
2. Color
3. Layers
4. History

Optional later panel:
- Swatches

### Recommended Heights
- Properties: 180px
- Color: 180px
- Layers: flexible, priority grow
- History: 150px

If Swatches is enabled later:
- Swatches: 110px

### Vertical Split Example
```text
┌──────────────────┐
│ Properties       │ 180
├──────────────────┤
│ Color            │ 180
├──────────────────┤
│ Layers           │ flexible
├──────────────────┤
│ History          │ 150
└──────────────────┘
```

---

## 4.9 Properties Panel

### Header
- Title left
- Panel menu right
- Collapse icon optional

### Body Layout
- Section stack
- 8px panel padding
- 6px row gaps
- Labels above controls for narrow mode
- Labels left of controls for wide mode

### Section Examples
- Layer
- Transform
- Mask later
- Tool Properties
- Document

### Illustrative Structure
```text
<Panel id="properties">
  <PanelHeader title="Properties"/>
  <Section title="Layer"/>
  <Section title="Transform"/>
</Panel>
```

---

## 4.10 Color Panel

### Layout
- Color square at top
- Hue slider below
- Numeric tabs below
- Recent colors footer

### Recommended Heights
- Color square: 120px
- Hue slider: 16px
- Numeric section: 72px

### Controls
- RGB
- HSV
- Hex
- Foreground/background preview

---

## 4.11 Swatches Panel

This panel is optional and later-phase. It should not be treated as required MVP scope unless the roadmap changes.

### Layout
- Dense grid of squares
- 8 columns default
- Swatch size 20 × 20px
- 4px gaps

### Behavior
- Single click sets foreground
- Alt click sets background
- Right click opens swatch menu later

---

## 4.12 Layers Panel

### Importance
This is the highest-priority dock panel and should expand to fill spare height.

### Header
- Blend mode dropdown
- Opacity control may live in header or body top row

Group and mask-specific controls are later additions and should not shape the first MVP implementation.

### Row Height
- Compact: 28px
- Comfortable: 32px

If groups are added later:
- Group rows may be 30px

### Row Structure
- Visibility toggle: 18px
- Optional disclosure: 14px
- Thumbnail: 24px
- Name + badges: flexible
- Lock state icon: 16px

Later additions:
- Mask thumbnail: 20px

### Footer Buttons
- New layer
- Delete

Later additions:
- New group
- New mask

### Layer Row Wireframe
```text
[👁] [thumb] Background Copy                       [lock]
[👁] [thumb] Character Paint
[👁] [thumb] Highlights
```

### Illustrative Structure
```text
<LayersPanel>
  <Header/>
  <LayerList/>
  <Footer>
    <IconButton id="new-layer"/>
    <!-- new-group later -->
    <!-- new-mask later -->
    <IconButton id="delete-layer"/>
  </Footer>
</LayersPanel>
```

---

## 4.13 History Panel

### Row Height
- 26px

### Layout
- Scrollable list
- Active state highlighted
- Minimal chrome
- Optional snapshot bookmarks later

### Visible Count
- Aim for 5 to 8 entries without scrolling in default layout

---

## 4.14 Status Bar

### Height
- 24px

### Layout Zones
- Left: zoom, doc size
- Center: cursor position, selection bounds
- Right: render status, color readout, future memory use

### Example
```text
100% | 1920×1080 px | Cursor 842,391 | Sel 220×140 | FG #4C8DFF
```

### Rules
- Low visual prominence
- Always readable
- No large buttons

---

## 4.15 Floating Context Strip

### Purpose
Provide action shortcuts near the active task.

### Placement
- Default bottom-center of canvas viewport
- Can move later
- Must avoid covering the active subject when possible

### Size
- Height: 36px
- Radius: 8px
- Padding: 6px horizontal
- Gap: 6px between controls

### Usage Cases
- Transform apply/cancel
- Selection refine
- Crop confirm/cancel
- Mask quick actions
- Future text formatting

### Illustrative Structure
```text
<FloatingContextStrip visible="true">
  <Button text="Apply"/>
  <Button text="Cancel"/>
  <Button text="Flip H"/>
  <Button text="Flip V"/>
</FloatingContextStrip>
```

---

## 5. Docking and Resize Rules

### Allowed in MVP
- Vertical resizing of dock panels
- Collapse panel sections
- Toggle panels from Window menu

### Allowed Later
- Tabbed panel groups
- Floating panels
- Detachable docks
- Saved workspaces
- Icon-collapsed dock rails

### Resize Priorities
1. Canvas gets first priority for extra width
2. Layers gets first priority for extra height in right dock
3. History compresses before Layers
4. Properties can collapse sections before full shrink

If Swatches is enabled later:
- Swatches compresses before Color

---

## 6. Alignment and Spacing Rules

### Core Spacing Scale
- 2px
- 4px
- 6px
- 8px
- 10px
- 12px
- 16px

### Usage
- Tiny gaps: 2–4px
- Button spacing: 6px
- Panel padding: 8px
- Section spacing: 10–12px
- Major region separation: 12–16px

### Separator Lines
- 1px subtle border
- No thick bevels
- No embossed panel chrome

---

## 7. Interaction Model

### Hover
- Slight surface lift
- Subtle background fill
- No bright outlines unless focused

### Active Tool
- Accent indicator
- Stronger contrast fill
- Icon tinted accent color

### Pressed
- Darkened pressed state
- 60–100ms transition max

### Disabled
- 40–50% opacity
- No hover response

### Focus
- Keyboard focus ring must be visible
- 1px accent border or glow

---

## 8. Responsive Rules

### Narrow Width Behavior
When the app width becomes constrained:
1. If enabled, hide swatches first
2. Convert lower-priority right panels into tabs only in a later docking-capable phase
3. Reduce tab widths
4. Collapse labels into icons in options bar where safe
5. Never hide the layer panel completely if a document is open

### Large Width Behavior
When the app has abundant width:
1. Expand canvas first
2. Expand layers panel second
3. Allow larger document tabs
4. Keep left tool rail fixed

---

## 9. Workspace Presets

### Planned Presets
- Essentials
- Painting
- Compositing
- Minimal
- Debug

### What a Workspace Saves
- Visible panels
- Panel order
- Panel sizes
- Tab group arrangement later
- Optional shortcut set later

---

## 10. Accessibility and Precision Rules

- Minimum hit area for icon buttons: 28 × 28px
- Text contrast must remain readable in dark mode
- Sliders must support mouse drag, wheel adjustment, and typed input when applicable
- Numeric fields should accept keyboard arrow incrementing
- Tooltips always show label and shortcut

---

## 11. GTK-Oriented Component Hierarchy

This is structural pseudocode for planning. It is not literal GTK widget code.

```text
<AppWindow>
  <TitleBar/>
  <MenuBar/>
  <ToolOptionsBar/>
  <HorizontalLayout>
    <LeftToolRail/>
    <VerticalLayout>
      <DocumentTabs/>
      <CanvasViewport/>
    </VerticalLayout>
    <RightDock>
      <PropertiesPanel/>
      <ColorPanel/>
      <LayersPanel/>
      <HistoryPanel/>
    </RightDock>
  </HorizontalLayout>
  <StatusBar/>
  <FloatingContextStrip/>
</AppWindow>
```

---

## 12. Suggested View Model Structure

```rust
pub struct WorkspaceLayout {
    pub left_toolbar_width: f32,
    pub right_dock_width: f32,
    pub status_bar_height: f32,
    pub options_bar_height: f32,
    pub panels: Vec<PanelLayout>,
    pub active_workspace: String,
}

pub struct PanelLayout {
    pub id: String,
    pub visible: bool,
    pub collapsed: bool,
    pub height: f32,
    pub order: usize,
}

pub struct DocumentTabVm {
    pub id: String,
    pub title: String,
    pub dirty: bool,
    pub active: bool,
}

pub struct CanvasViewportVm {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub show_rulers: bool,
    pub show_grid: bool,
    pub show_pixel_grid: bool,
}

pub struct ToolOptionsVm {
    pub active_tool: String,
    pub controls: Vec<ToolControlVm>,
}

pub struct ToolControlVm {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub enabled: bool,
}
```

---

## 13. MVP Layout Scope

Implement in MVP:
- titlebar
- menu bar
- tool options bar
- left tool rail
- document tabs
- canvas viewport
- properties panel
- color panel
- layers panel
- history panel
- status bar

Defer:
- advanced docking
- floating detached panels
- workspace preset persistence
- swatches panel
- navigator panel
- panel search
- panel icon-rail collapse

The deferred list should stay aligned with the product roadmap. If the roadmap changes, update this document rather than letting it drift.

---

## 14. Acceptance Criteria

The layout is complete when:
- the application opens with a stable three-column workspace
- the canvas remains central and visually dominant
- tool switching is fast
- right-side panels are usable without crowding the canvas
- the layer panel supports realistic editing workflows
- the layout remains coherent at 1280×800 and 2560×1440
- the structure is componentized for GTK4 without coupling document logic to widget state
