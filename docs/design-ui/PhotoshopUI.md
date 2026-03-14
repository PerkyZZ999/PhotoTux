# PHOTOSHOP UI

***

# 📑 DOCUMENT 1: Application Layout & Core Architecture

## 1. Overview
The user interface is divided into five primary structural zones. The recommended underlying web layout approach is CSS Grid or CSS Flexbox to manage these fixed and fluid zones.

1. **Menu Bar:** Top-most fixed horizontal bar.
2. **Options Bar:** Secondary horizontal bar situated immediately below the Menu Bar.
3. **Tools Bar (Toolbar):** Fixed vertical sidebar on the extreme left.
4. **Panels:** Collapsible/resizable vertical sidebar(s) on the extreme right.
5. **Document Window:** The flexible, central canvas area occupying the remaining screen space.

## 2. High-Level Structural Grid
If mapping this to a standard UI grid:
* **Top Area (100% width):** Stacked Menu Bar + Options Bar.
* **Middle Area (100% width, flexible height):**
  * **Left:** Tools Bar (Fixed width, e.g., 40px to 60px).
  * **Center:** Document Window (Flex-grow: 1, dynamic width/height).
  * **Right:** Panels (Fixed but resizable width, e.g., 250px to 300px).

---

# 📑 DOCUMENT 2: Top Navigation (Menu & Options Bars)

## 1. Menu Bar
This is the global application control center. It houses dropdown menus for application and file-level management.

**Standard Dropdown Menus to Implement:**
* **File:** New, Open, Save, Export, Print, Close.
* **Edit:** Undo, Redo, Cut, Copy, Paste, Transform, Preferences.
* **Image:** Image Size, Canvas Size, Adjustments, Mode.
* **Layer:** New Layer, Duplicate, Merge, Layer Styles.
* **Type:** Font selection, Typography settings.
* **Select:** All, Deselect, Inverse, Masking options.
* **Filter:** Blur, Sharpen, Distortion, Stylize effects.
* **View:** Zoom In/Out, Rulers, Grid, Guides, Full Screen.
* **Window:** Toggles to show/hide specific right-side Panels, Workspace presets.
* **Help:** Search, About, Shortcuts, Documentation.

## 2. Options Bar (Contextual Toolbar)
This bar is **dynamic**. Its contents change entirely based on which tool is currently selected in the Left Tools Bar.
* **UI Behavior:** When the user clicks a tool (e.g., Brush), the Options Bar updates to show Brush-specific settings.
* **Typical Elements (Example for Brush Tool):**
  * Current Tool Icon (far left).
  * Tool Presets dropdown.
  * Size/Hardness slider.
  * Blending Mode dropdown.
  * Opacity slider (%).
  * Flow slider (%).

---

# 📑 DOCUMENT 3: Tools Bar (Left Sidebar)

This is a fixed vertical column. A crucial UI behavior to implement is the **"Fly-out" or "Nested" menus**. Many tool buttons have a small triangle in the bottom-right corner, indicating that clicking and holding the button will reveal a sub-menu of related tools.

Here is the exact layout of the tools grouped by their functional categories based on the standard Photoshop layout.

## Group A: Move & Selection
* **Move Tool (V)** -> *Nested: Artboard Tool*
* **Rectangular Marquee Tool (M)** -> *Nested: Elliptical Marquee, Single Row Marquee, Single Column Marquee*
* **Lasso Tool (L)** -> *Nested: Polygonal Lasso, Magnetic Lasso*

## Group B: Advanced Selection, Crop & Measure
* **Quick Selection Tool (W)** -> *Nested: Magic Wand Tool*
* **Crop Tool (C)** -> *Nested: Perspective Crop, Slice Tool, Slice Select Tool*
* **Eyedropper Tool (I)** -> *Nested: 3D Material Eyedropper, Color Sampler, Ruler, Note, Count Tool*

## Group C: Retouching & Painting
* **Spot Healing Brush Tool (J)** -> *Nested: Healing Brush, Patch Tool, Content-Aware Move, Red Eye Tool*
* **Brush Tool (B)** -> *Nested: Pencil, Color Replacement, Mixer Brush*
* **Clone Stamp Tool (S)** -> *Nested: Pattern Stamp Tool*

## Group D: History, Erase & Fill
* **History Brush Tool (Y)** -> *Nested: Art History Brush*
* **Eraser Tool (E)** -> *Nested: Background Eraser, Magic Eraser*
* **Gradient Tool (G)** -> *Nested: Paint Bucket Tool, 3D Material Drop Tool*

## Group E: Modification (Blur, Dodge, Burn)
* **Blur Tool** -> *Nested: Sharpen Tool, Smudge Tool*
* **Dodge Tool (O)** -> *Nested: Burn Tool, Sponge Tool*

## Group F: Vector, Type & Navigation
* **Pen Tool (P)** -> *Nested: Freeform Pen, Add Anchor Point, Delete Anchor Point, Convert Point*
* **Horizontal Type Tool (T)** -> *Nested: Vertical Type, Vertical Type Mask, Horizontal Type Mask*
* **Path Selection Tool (A)** -> *Nested: Direct Selection Tool*
* **Rectangle Tool (U)** -> *Nested: Rounded Rectangle, Ellipse, Polygon, Line, Custom Shape*
* **Hand Tool (H)** -> *Nested: Rotate View Tool*
* **Zoom Tool (Z)**

## Bottom of Tools Bar
* **Color Swatches:** Two overlapping square buttons representing Foreground Color and Background Color.
* **Quick Mask Mode:** Toggle button.
* **Screen Mode:** Toggle to switch between Standard, Full Screen with Menu, and Full Screen.

---

# 📑 DOCUMENT 4: Panels (Right Sidebar)

The right sidebar is modular. It consists of individual "Panels" that can be grouped together using tabs. The AI should build this as a docking system where panels can be collapsed, expanded, or dragged to create new groups.

## Core Panels to Implement
1. **Layers Panel (Crucial):**
   * **List Area:** Displays stacked layers with a visibility toggle (eye icon) and lock toggle.
   * **Bottom Toolbar:** Buttons for Delete Layer, New Layer, New Group (folder), Adjustment Layer, Layer Mask, and Layer Styles.
   * **Top Area:** Blending Mode dropdown, Opacity slider, Fill slider.
2. **Properties Panel:** Context-sensitive. If a text layer is selected, it shows typography settings. If a shape is selected, it shows fill/stroke settings.
3. **Color & Swatches Panel:** A color picker UI (RGB/CMYK sliders or a color wheel) and a grid of saved color swatches.
4. **History Panel:** A vertical list of actions the user has taken, allowing them to click back to an earlier state (Undo history).
5. **Adjustments Panel:** A grid of icons for adding non-destructive adjustment layers (e.g., Brightness/Contrast, Hue/Saturation, Curves).

## Panel Grouping Layout
By default, panels are stacked vertically in tabbed groups. A standard default view looks like:
* **Top Group:** Color | Swatches | Gradients (Tabs)
* **Middle Group:** Properties | Adjustments (Tabs)
* **Bottom Group:** Layers | Channels | Paths (Tabs)

---

# 📑 DOCUMENT 5: Document Window (Center Area)

This is the main working area. It is relatively empty but has specific UI framing components.

## 1. Document Tabs (Top)
* Located directly below the Options Bar, acting as the header of the Document Window.
* Functions like browser tabs to switch between multiple open images/projects.
* **Tab Data:** File Name, Zoom Level (e.g., 66.7%), and Color Mode (RGB/8#).
* Has a small 'x' to close the document.

## 2. Canvas (Center)
* The actual artboard or image container.
* Supports visual Rulers (horizontal across the top edge, vertical down the left edge).
* Needs to support panning (scrollbar or Hand Tool) and zooming natively.

## 3. Status Bar (Bottom)
* A small strip at the very bottom-left of the Document Window.
* Displays the current Zoom percentage.
* Displays document metadata (e.g., Document File Size, Color Profile, or Dimensions).
