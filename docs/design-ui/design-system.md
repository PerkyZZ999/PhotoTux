# PhotoTux Design Tokens
Version: 1.0
Theme: Dark Pro
Purpose: Define reusable tokens for color, spacing, typography, borders, shadows, icon sizing, states, animation, and panel styling.

Scope note: this token system is intended for the fixed-layout UI defined for the early PhotoTux releases. It should not be read as a commitment to advanced docking, broader tool coverage, or features that are currently deferred in the roadmap.

---

## 1. Design Principles

The design system should feel:
- professional
- restrained
- dense but readable
- neutral around artwork
- consistent across panels and dialogs
- optimized for long sessions
- familiar to users of pro creative tools without reproducing any specific product skin

The canvas must remain the hero.
Panels must support heavy information density without looking cluttered.
Accent usage must be selective.

---

## 2. Color Tokens

## 2.1 Base Surfaces

```json
{
  "color.bg.app": "#1B1D21",
  "color.bg.chrome": "#202329",
  "color.bg.panel": "#252930",
  "color.bg.panel.alt": "#2A2F37",
  "color.bg.panel.header": "#2C3139",
  "color.bg.canvas.surround": "#14161A",
  "color.bg.canvas.pasteboard": "#101216",
  "color.bg.input": "#1F232A",
  "color.bg.menu": "#252A31",
  "color.bg.tooltip": "#2C3138",
  "color.bg.overlay": "rgba(10,12,16,0.72)"
}
```

## 2.2 Borders and Dividers

```json
{
  "color.border.subtle": "#313741",
  "color.border.default": "#3A414D",
  "color.border.strong": "#4A5361",
  "color.border.accent": "#4F8CFF",
  "color.border.danger": "#C95A5A"
}
```

## 2.3 Text

```json
{
  "color.text.primary": "#E8ECF3",
  "color.text.secondary": "#B3BCC8",
  "color.text.muted": "#8A94A3",
  "color.text.disabled": "#677180",
  "color.text.inverse": "#0E1116",
  "color.text.link": "#7CB3FF"
}
```

## 2.4 Accent and Semantic Colors

```json
{
  "color.accent.primary": "#4F8CFF",
  "color.accent.primary.hover": "#67A0FF",
  "color.accent.primary.active": "#3B79F1",
  "color.accent.selection": "#4F8CFF",
  "color.success": "#4DAA6D",
  "color.warning": "#D2A24A",
  "color.danger": "#C95A5A",
  "color.info": "#5AA8E5"
}
```

## 2.5 Layer and Overlay Helpers

```json
{
  "color.overlay.selection.fill": "rgba(79,140,255,0.14)",
  "color.overlay.selection.stroke": "#74A7FF",
  "color.overlay.transform.handle": "#EAF1FF",
  "color.overlay.transform.line": "#77A8FF",
  "color.overlay.guide": "#D56CFF",
  "color.overlay.grid": "rgba(255,255,255,0.08)",
  "color.overlay.pixelgrid": "rgba(255,255,255,0.12)",
  "color.overlay.mask": "rgba(255,0,0,0.18)"
}
```

## 2.6 Checkerboard Tokens

```json
{
  "color.checker.light": "#50555F",
  "color.checker.dark": "#3E434C",
  "checker.size.default": 12,
  "checker.size.zoomed": 16
}
```

---

## 3. Typography Tokens

## 3.1 Font Families

```json
{
  "font.family.ui": "IBM Plex Sans, Noto Sans, system-ui, sans-serif",
  "font.family.mono": "JetBrains Mono, Cascadia Code, monospace"
}
```

## 3.2 Font Sizes

```json
{
  "font.size.xs": 9,
  "font.size.sm": 10,
  "font.size.md": 11,
  "font.size.lg": 12,
  "font.size.xl": 16
}
```

## 3.3 Font Weights

```json
{
  "font.weight.regular": 400,
  "font.weight.medium": 500,
  "font.weight.semibold": 600,
  "font.weight.bold": 700
}
```

## 3.4 Text Styles

```json
{
  "textstyle.menu.size": 11,
  "textstyle.menu.weight": 500,
  "textstyle.panel.title.size": 10,
  "textstyle.panel.title.weight": 600,
  "textstyle.panel.title.case": "title",
  "textstyle.label.size": 10,
  "textstyle.label.weight": 500,
  "textstyle.body.size": 10,
  "textstyle.body.weight": 400,
  "textstyle.input.size": 10,
  "textstyle.input.weight": 500,
  "textstyle.tooltip.size": 10,
  "textstyle.tooltip.weight": 400,
  "textstyle.status.size": 10,
  "textstyle.status.weight": 500
}
```

## 3.5 Line Heights

```json
{
  "lineheight.tight": 1.15,
  "lineheight.normal": 1.3,
  "lineheight.relaxed": 1.45
}
```

---

## 4. Spacing Tokens

```json
{
  "space.0": 0,
  "space.2": 2,
  "space.4": 4,
  "space.6": 6,
  "space.8": 8,
  "space.10": 10,
  "space.12": 12,
  "space.16": 16,
  "space.20": 20,
  "space.24": 24,
  "space.32": 32
}
```

### Usage Rules
- Icon gaps: 4–6
- Button gaps: 4–6
- Panel padding: 4–8
- Dialog padding: 16
- Section spacing: 6–10
- Large region padding: 16–24

---

## 5. Radius Tokens

```json
{
  "radius.none": 0,
  "radius.xs": 3,
  "radius.sm": 4,
  "radius.md": 6,
  "radius.lg": 8,
  "radius.xl": 10,
  "radius.pill": 999
}
```

### Usage
- Inputs: 3 or 4
- Buttons: 3 or 4
- Floating bars: 6-8
- Dialogs: 8
- Small chips: 2 or 3

## 5.1 Workspace Density Guidance

- Titlebar, menu bar, and options bar should read as compact chrome, not card stacks
- Toolbars and docks favor small radii and tight spacing over large rounded surfaces
- Panel groups use shared headers with tabs instead of repeating large standalone card headers
- The document window should remain visually framed even when the surrounding chrome becomes dense

---

## 6. Border Tokens

```json
{
  "border.width.hairline": 1,
  "border.width.thin": 1,
  "border.width.medium": 2
}
```

### Usage
- Panel separators: 1
- Input borders: 1
- Focus rings: 1 or 2
- Active tool outline: 1
- Panel group tab separators: 1
- Document window frame: 1

---

## 7. Shadow Tokens

```json
{
  "shadow.none": "none",
  "shadow.sm": "0 1px 2px rgba(0,0,0,0.18)",
  "shadow.md": "0 4px 12px rgba(0,0,0,0.28)",
  "shadow.lg": "0 10px 24px rgba(0,0,0,0.34)",
  "shadow.float": "0 8px 24px rgba(0,0,0,0.36)"
}
```

### Usage
- Panels: none or sm
- Menus: md
- Dialogs: lg
- Floating context strip: float
- Tooltips: md

---

## 8. Size Tokens

## 8.1 Icon Sizes

```json
{
  "icon.size.xs": 12,
  "icon.size.sm": 14,
  "icon.size.md": 16,
  "icon.size.lg": 18,
  "icon.size.xl": 20,
  "icon.stroke.default": 1.8
}
```

## 8.2 Control Heights

```json
{
  "control.height.xs": 24,
  "control.height.sm": 28,
  "control.height.md": 32,
  "control.height.lg": 36,
  "control.height.xl": 40
}
```

## 8.3 Button Width Guidance

```json
{
  "button.width.icon": 28,
  "button.width.compact": 72,
  "button.width.default": 96,
  "button.width.wide": 128
}
```

## 8.4 Panel Metrics

```json
{
  "panel.header.height": 30,
  "panel.footer.height": 32,
  "panel.padding": 8,
  "panel.section.gap": 10,
  "panel.row.gap": 6
}
```

## 8.5 Toolbar Metrics

```json
{
  "toolbar.width": 60,
  "toolbar.button.size": 36,
  "toolbar.button.gap": 6,
  "toolbar.section.gap": 10
}
```

## 8.6 Tab Metrics

```json
{
  "tab.height": 34,
  "tab.min.width": 120,
  "tab.pref.width": 180,
  "tab.max.width": 240,
  "tab.close.zone": 22
}
```

---

## 9. State Tokens

## 9.1 Opacity States

```json
{
  "opacity.disabled": 0.45,
  "opacity.subtle": 0.72,
  "opacity.hover.overlay": 0.08,
  "opacity.active.overlay": 0.16,
  "opacity.selected.overlay": 0.20
}
```

## 9.2 Button States

```json
{
  "button.bg.default": "transparent",
  "button.bg.hover": "rgba(255,255,255,0.06)",
  "button.bg.pressed": "rgba(255,255,255,0.10)",
  "button.bg.active": "rgba(79,140,255,0.18)",
  "button.border.active": "#4F8CFF",
  "button.text.default": "#E8ECF3",
  "button.text.muted": "#B3BCC8"
}
```

## 9.3 Input States

```json
{
  "input.bg.default": "#1F232A",
  "input.bg.hover": "#232832",
  "input.bg.focus": "#232A35",
  "input.border.default": "#3A414D",
  "input.border.focus": "#4F8CFF",
  "input.text": "#E8ECF3",
  "input.placeholder": "#7C8696"
}
```

## 9.4 Panel States

```json
{
  "panel.bg.default": "#252930",
  "panel.bg.hover": "#292E36",
  "panel.header.bg": "#2C3139",
  "panel.header.active": "#313843",
  "panel.border": "#313741"
}
```

---

## 10. Layer Panel Tokens

```json
{
  "layer.row.height.compact": 28,
  "layer.row.height.comfortable": 32,
  "layer.thumb.size": 24,
  "layer.mask.size": 20,
  "layer.indent": 14,
  "layer.visibility.zone": 18,
  "layer.badge.gap": 4,
  "layer.row.bg.hover": "rgba(255,255,255,0.05)",
  "layer.row.bg.selected": "rgba(79,140,255,0.16)",
  "layer.row.border.selected": "#4F8CFF"
}
```

### Layer Name Rules
- Truncate with ellipsis
- Preserve left alignment
- Use semibold for active layer
- Use muted text for hidden or locked variants

---

## 11. Menu and Tooltip Tokens

## 11.1 Menu Tokens

```json
{
  "menu.bg": "#252A31",
  "menu.border": "#3A414D",
  "menu.item.height": 28,
  "menu.item.padding.x": 10,
  "menu.item.padding.y": 6,
  "menu.item.hover": "rgba(255,255,255,0.06)",
  "menu.item.active": "rgba(79,140,255,0.16)"
}
```

## 11.2 Tooltip Tokens

```json
{
  "tooltip.bg": "#2C3138",
  "tooltip.border": "#3E4652",
  "tooltip.text": "#F1F5FB",
  "tooltip.padding.x": 8,
  "tooltip.padding.y": 6,
  "tooltip.radius": 4
}
```

---

## 12. Dialog Tokens

```json
{
  "dialog.width.sm": 420,
  "dialog.width.md": 560,
  "dialog.width.lg": 760,
  "dialog.padding": 16,
  "dialog.section.gap": 14,
  "dialog.footer.gap": 8,
  "dialog.radius": 8,
  "dialog.bg": "#252A31",
  "dialog.border": "#3A414D"
}
```

### Modal Overlay
```json
{
  "overlay.modal.bg": "rgba(10,12,16,0.72)"
}
```

---

## 13. Motion Tokens

### Duration
```json
{
  "motion.instant": 0,
  "motion.fast": 80,
  "motion.normal": 140,
  "motion.slow": 220
}
```

### Easing
```json
{
  "easing.standard": "cubic-bezier(0.2, 0.0, 0.2, 1)",
  "easing.emphasized": "cubic-bezier(0.2, 0.0, 0, 1)",
  "easing.exit": "cubic-bezier(0.4, 0.0, 1, 1)"
}
```

### Motion Rules
- Hover transitions: fast
- Panel expand/collapse: normal
- Dialog open/close: normal
- Tool switch highlight: fast
- Do not animate canvas content transforms with UI easing

---

## 14. Cursor Tokens

```json
{
  "cursor.default": "arrow",
  "cursor.clickable": "pointer",
  "cursor.text": "ibeam",
  "cursor.drag": "grab",
  "cursor.drag.active": "grabbing",
  "cursor.resize.h": "ew-resize",
  "cursor.resize.v": "ns-resize",
  "cursor.crosshair": "crosshair"
}
```

---

## 15. Focus Tokens

```json
{
  "focus.ring.width": 1,
  "focus.ring.color": "#4F8CFF",
  "focus.ring.offset": 1,
  "focus.glow": "0 0 0 2px rgba(79,140,255,0.18)"
}
```

### Accessibility Rules
- Every keyboard-focusable item must show visible focus
- Focus indication must differ from hover
- High-contrast mode may later swap focus to stronger outlines

---

## 16. Iconography Tokens

### Icon Style Rules
- Stroke icons only for primary chrome
- Filled icons allowed for state badges and visibility indicators
- Monochrome by default
- Accent tint only for active tools and key actions
- No photorealistic or glossy icons
- Consistent 16px and 18px grid

### Icon Sizes by Area
```json
{
  "icon.toolbar": 18,
  "icon.panel.header": 14,
  "icon.menu": 14,
  "icon.status": 12,
  "icon.dialog": 16
}
```

---

## 17. Canvas Overlay Tokens

```json
{
  "canvas.handle.size": 8,
  "canvas.handle.border": "#0E1116",
  "canvas.handle.fill": "#F3F7FF",
  "canvas.transform.line.width": 1,
  "canvas.guide.width": 1,
  "canvas.selection.dash.length": 4,
  "canvas.selection.dash.gap": 4
}
```

### Overlay Rules
- Keep overlays crisp and high-contrast
- Avoid thick bounding boxes
- Use thin lines with clear handles
- Marching ants can be introduced later

---

## 18. Rust Token Export Example

This Rust example is illustrative only. It does not define a required final API shape.

```rust
pub struct ThemeTokens {
    pub bg_app: &'static str,
    pub bg_panel: &'static str,
    pub bg_panel_header: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub accent_primary: &'static str,
    pub border_subtle: &'static str,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub panel_padding: f32,
    pub control_height_md: f32,
    pub toolbar_width: f32,
    pub panel_header_height: f32,
    pub tab_height: f32,
}
```

---

## 19. GTK4-Friendly Theme Mapping

This mapping is GTK-oriented reference material, not a locked final implementation.

```css
:root {
  --bg-app: #1B1D21;
  --bg-panel: #252930;
  --bg-panel-header: #2C3139;
  --text-primary: #E8ECF3;
  --text-secondary: #B3BCC8;
  --accent-primary: #4F8CFF;
  --border-subtle: #313741;

  --radius-sm: 4px;
  --radius-md: 6px;
  --panel-padding: 8px;
  --control-height-md: 32px;
  --toolbar-width: 60px;
  --panel-header-height: 30px;
  --tab-height: 34px;
}
```

---

## 20. Semantic Component Mapping

### Buttons
- Use transparent default background
- Hover with soft light overlay
- Active with blue-tinted overlay
- Danger buttons get red border or fill only when needed

### Panels
- Flat dark surfaces
- Thin separators
- Slightly brighter headers
- Minimal border noise

### Inputs
- Slight inset effect through darker fill
- Accent border on focus
- No glowing neon treatment

### Tabs
- Flat, compact, readable
- Active tab is brighter
- Inactive tabs blend into chrome

Tabs are appropriate for document switching in the fixed-layout shell. They are not a commitment to full dock-tab systems in MVP.

---

## 21. Density Modes

### Compact
```json
{
  "density.toolbar.button": 32,
  "density.panel.row": 26,
  "density.input.height": 28,
  "density.tab.height": 30
}
```

### Standard
```json
{
  "density.toolbar.button": 36,
  "density.panel.row": 28,
  "density.input.height": 32,
  "density.tab.height": 34
}
```

### Comfortable
```json
{
  "density.toolbar.button": 40,
  "density.panel.row": 32,
  "density.input.height": 36,
  "density.tab.height": 38
}
```

Default should be `standard`.

---

## 22. Theme Constraints

Do:
- keep the palette neutral
- reserve saturation for selected and actionable states
- preserve strong contrast for artwork visibility
- keep panel chrome understated

Do not:
- overuse gradients
- add glassmorphism
- use strong drop shadows everywhere
- use colorful panel backgrounds
- reduce contrast in the name of elegance

---

## 23. Acceptance Criteria

The token system is complete when:
- all primary UI surfaces map to tokens
- no hardcoded colors remain in production UI code
- spacing is normalized to the scale
- controls share consistent heights and radii
- active, hover, disabled, and focus states are visually distinct
- the canvas remains visually separated from the chrome
