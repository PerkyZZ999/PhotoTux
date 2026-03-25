# Post-MVP Plan: GTK UI Template Migration

## Purpose

Define a safe, staged path for migrating stable GTK shell structure from Rust-built widget trees to `.ui` template files using a clear three-layer model:

- structure in `.ui` templates
- behavior in Rust
- appearance in GTK CSS classes

The goal is to improve maintainability without weakening PhotoTux's architecture boundaries.

## Why This Is A Natural Later Step

During MVP and early post-MVP work, building shell widgets directly in Rust is the fastest way to iterate on layout, menus, and panel behavior.

Later, once major shell surfaces stabilize, `.ui` files can help by:

- reducing boilerplate in repetitive widget trees
- making panel structure easier to scan and review
- improving reuse for dialogs and stable panel shells
- separating static widget composition from runtime behavior wiring

This is not a styling replacement. GTK CSS remains the styling system. `.ui` files are a structural and templating tool, and they should be paired with Rust-side widget binding instead of replacing Rust application logic.

The intended default direction after MVP is:

- use `.ui` files for windows, dialogs, sidebars, preferences, and repeated widget trees
- use Rust subclassing plus `CompositeTemplate` for complex custom widgets and signal wiring
- use CSS classes for spacing, accent treatment, surface styling, and stateful appearance changes
- follow GNOME HIG interaction patterns first, then add PhotoTux-specific visual personality second

## Goal

Adopt `.ui` templates selectively for stable GTK shell surfaces while keeping behavior in Rust, preserving the document-first architecture, and avoiding a risky all-at-once rewrite.

The migration target is not merely “some XML files.” It is a cleaner shell architecture where declarative GTK structure, imperative Rust behavior, and CSS-based appearance are intentionally separated.

## Scope

### In Scope

- selective migration of stable shell structure to `.ui` files
- use of `gtk::Builder` or `CompositeTemplate` for static widget trees
- panel, dialog, and reusable shell-widget extraction where structure is mostly declarative
- CSS class and widget-name cleanup that supports template-based shells
- `TemplateChild`-driven access to template widgets where it materially improves clarity
- GNOME HIG alignment for stable shell controls and layouts where appropriate
- tests and validation for template loading and widget wiring

### Explicitly Out Of Scope

- replacing GTK CSS with XML styling
- moving document, tool, or renderer logic into UI templates
- rewriting dynamic canvas or renderer-adjacent behavior into declarative markup
- broad framework migration unrelated to GTK templates
- all-at-once shell conversion
- using CSS to re-implement the toolkit instead of styling it lightly

## Recommended Delivery Order

1. define template boundaries and migration rules
2. establish the three-layer shell conventions and styling rules
3. migrate stable dialogs and simple shell surfaces
4. migrate stable sidebar panel shells
5. extract reusable composite widgets where it clearly reduces duplication
6. evaluate whether the top-level shell layout should remain Rust-built or only partially templated

## Work Breakdown

### Phase 1: Define Template Boundaries

Deliverables:

- documented rules for what belongs in `.ui` files versus Rust
- naming conventions for template files, IDs, CSS classes, and widget ownership
- loading strategy decision: builder-based loading versus `CompositeTemplate` by surface type
- explicit shell rule that layout belongs in templates, state and events belong in Rust, and theme-level appearance belongs in CSS

Key design rules:

- `.ui` files own static widget structure only
- runtime behavior, command routing, and state ownership remain in Rust
- document state must never migrate into GTK template-owned logic
- CSS should stay class-based and light, rather than trying to simulate custom toolkit behavior

Exit criteria:

- the team can decide consistently whether a new shell surface should be Rust-built or template-backed

#### UIT01 Decisions And Conventions

Template location:

- `.ui` templates live under `crates/ui_shell/src/ui/`
- organize templates by surface type:
  - `dialogs/` for standalone dialogs and simple modal surfaces
  - `panels/` for stable sidebar panel shells
  - `fragments/` for shared structural fragments only after repetition is proven
- keep one root surface per file and use kebab-case file names such as `about-dialog.ui`, `layers-panel.ui`, or `panel-header.ui`
- in the first migration pass, templates are checked into the repository as source files and embedded from Rust-owned loading helpers rather than loaded from arbitrary runtime disk paths

Naming conventions:

- root template object IDs use snake_case and name the surface first, for example `about_dialog`, `layers_panel`, or `history_panel`
- child widget IDs used through `gtk::Builder` or `TemplateChild` also use snake_case and stay surface-prefixed when the role is not obvious, for example `layers_panel_toolbar`, `layers_panel_list`, or `import_report_summary`
- Rust helper names, binding functions, and callback methods stay snake_case and should mirror the semantic role of the template widgets they bind
- CSS classes referenced from templates remain kebab-case and role-oriented so they fit the existing shell vocabulary such as `panel-group`, `tool-chip`, or `status-label`

Builder versus `CompositeTemplate`:

- use `gtk::Builder` for low-risk, one-off dialogs or simple shell surfaces with shallow structure and limited internal state
- use `CompositeTemplate` for reusable or stateful panel shells and custom widgets that benefit from typed `TemplateChild` access and subclass-owned wiring
- keep template loading behind shared `ui_shell` helpers so bad loads and missing IDs fail consistently instead of producing per-surface ad hoc lookup code
- keep the top-level workspace shell, canvas host, and highly dynamic snapshot-driven rows Rust-built unless a later task proves a stable template boundary

Three-layer ownership rule:

- `.ui` files own static widget hierarchy, structural containers, default labels, tooltips, accessibility metadata, and CSS class attachment
- Rust owns signal wiring, controller calls, state projection, conditional behavior, background-job result handling, and any rows or controls built from live controller snapshots
- GTK CSS owns appearance through shared classes only; do not use templates or CSS to recreate widget behavior that GTK already provides natively
- document state, tool state, and renderer state remain outside template widgets; templates describe shell structure, not source-of-truth editor state

### Phase 2: Establish The Three-Layer Shell Conventions

Deliverables:

- conventions for using `.ui` for layout, Rust for state/events, and CSS for appearance
- guidelines for when to introduce `CompositeTemplate`, `TemplateChild`, and template callbacks
- shell styling rules that prefer GTK-native widgets and small class-based CSS

Key design rules:

- prefer native GTK widget behavior over custom-drawn shell controls unless a real product need exists
- keep CSS small and class-based
- keep WGPU reserved for the canvas and rendering-heavy path, not surrounding shell controls

Exit criteria:

- the migration effort is guided by a stable UI architecture model, not just by file-format preference

#### UIT02 Styling Conventions

Styling rules for template-backed UI:

- prefer existing role-based CSS classes in templates before creating new ones; template-backed surfaces should reuse shell vocabulary such as `panel-group`, `panel-group-body`, `panel-tab`, `tool-chip`, and `status-label` where those classes already express the right role
- add new CSS classes only for template-local spacing or typography that the existing class vocabulary does not already cover, for example a small dialog-specific stack such as `template-dialog-content` or `template-dialog-title`
- use widget names for stable roots, targeted debugging, accessibility review, or test/automation lookups; do not rely on widget names as the primary styling hook for normal shell appearance
- keep selectors shallow and class-oriented; do not introduce ID-heavy or deeply nested CSS selectors that make template structure changes fragile
- do not use CSS to imitate behavior that GTK already owns natively, such as fake tab systems, faux button state machines, or custom-drawn shell chrome
- keep dialog and panel actions on normal GTK widgets with light styling only, and let runtime state changes continue to come from Rust-owned controller updates

GNOME-HIG-aligned defaults for this track:

- simple informational dialogs should keep a clear title/body/detail hierarchy, a modest action count, and end-aligned confirmation or close actions
- template-backed panels should preserve native focus, sensitivity, and disabled-state behavior instead of styling inactive controls to look interactive
- template-backed shells should stay visually quiet around the canvas, using CSS for clarity and hierarchy rather than decorative density

### Phase 3: Migrate Stable Dialogs And Simple Surfaces

Deliverables:

- template-backed about, settings, recovery, or future stable dialogs where appropriate
- template loading helpers in `ui_shell`
- validation that CSS classes and accessibility labels survive template loading cleanly
- proof that template-backed surfaces remain easy to wire using Rust-owned behavior

Key design rules:

- start with low-risk surfaces that do not define the canvas workflow
- avoid mixing template migration with behavior redesign in the same step

Exit criteria:

- at least one non-trivial shell surface is template-backed without behavioral regression

### Phase 4: Migrate Stable Sidebar Panel Shells

Deliverables:

- template-backed panel containers for Color, Properties, Layers, or History where the structure is stable
- clear Rust-side widget binding and update hooks
- cleaner separation between panel structure and controller-driven state updates
- adoption of `CompositeTemplate` for panels that are complex enough to benefit from typed template children

Key design rules:

- panel shells may be templated, but state updates still route through `app_core` and controller snapshots
- templates must not become hidden state containers

Exit criteria:

- stable shell panels are easier to maintain without changing document or controller ownership

### Phase 5: Extract Reusable Composite Widgets

Deliverables:

- reusable template-backed widgets for repeated shell patterns
- shared loading and binding conventions
- reduced duplication in panel headers, rows, and common shell elements where justified
- explicit use of `CompositeTemplate` and `TemplateChild` where those APIs make custom widgets clearer than ad hoc builder lookups

Key design rules:

- only extract reusable widgets where repetition is real and stable
- do not create a template abstraction layer that is heavier than the duplication it removes

Exit criteria:

- the shell has less structural duplication and no major increase in indirection cost

#### UIT07 And UIT08 Reuse Decisions

Reusable fragment direction:

- repeated sidebar panel shells now justify a shared fragment, so `ui_shell` should prefer a single builder-backed `panel-group` fragment under `crates/ui_shell/src/ui/fragments/` over one `.ui` file per structurally identical panel shell
- the shared fragment owns only the repeated shell structure: root container, header strip, tab buttons, and body container
- Rust still configures surface-specific tab labels, widget names, body spacing, and vexpand behavior after loading the fragment
- keep this reuse layer builder-backed for now; do not introduce `CompositeTemplate` until a reusable shell widget owns enough internal behavior or typed child access to justify subclass overhead

Selective panel migration guidance:

- use the shared panel fragment for stable sidebar shells such as Color, Properties, Layers, and History where the outer structure is repeated and low-volatility
- keep dynamic panel contents, history rows, layer rows, tool-state summaries, and controller-driven action wiring in Rust even when the surrounding shell is template-backed
- do not migrate highly volatile or deeply stateful row composition into `.ui` markup just because the outer panel shell is now templated

### Phase 6: Reassess Top-Level Shell Composition

Deliverables:

- explicit decision on whether the top-level application shell should stay mostly Rust-built or adopt partial templates
- migration notes for any remaining high-value shell surfaces
- documented reasons for keeping dynamic surfaces in Rust where appropriate

Key design rules:

- keep fast-changing or behavior-heavy surfaces in Rust if templates do not materially help
- do not chase template usage for its own sake
- keep the canvas shell boundary clear: WGPU remains responsible for the canvas path, GTK remains responsible for surrounding panels and controls

Exit criteria:

- the project has an intentional long-term split between template-backed structure and Rust-built dynamic composition

#### UIT09 Top-Level Shell Decisions

The current intentional split is:

- keep the top-level application shell Rust-built: root window composition, header bar, menu bar and popovers, tool-options bar, left tool rail, document workspace assembly, document tabs, right-sidebar docking, and status-bar composition
- keep the canvas host and all WGPU-adjacent surfaces Rust-built because they own input controllers, renderer lifetime, and low-latency state projection
- keep snapshot-driven row builders Rust-built, including live layer rows, history rows, properties summaries, guide state summaries, and other surfaces whose structure changes with controller state
- use `.ui` templates for stable dialogs and repeated shell containers where the structure is mostly declarative and the runtime wiring remains straightforward in Rust

This split should remain the default until a later task can show that a higher-level shell section has become structurally stable enough to template without hiding dynamic ownership.

#### UIT10 Validation And Contributor Notes

Validation rules for future template-backed surfaces:

- add default automated coverage for embedded template source metadata, required builder IDs, CSS classes, and stable widget names where practical
- keep GTK builder-runtime assertions as ignored or manual checks when they require the real process main thread instead of the normal Cargo test harness
- manually verify CSS class application, labels, tooltips, focus behavior, sensitivity, and dialog or panel layout on a real GTK session after migrating a surface

Contributor rules for extending this track:

- add new `.ui` files under the documented `dialogs/`, `panels/`, or `fragments/` directories rather than inventing a new layout
- prefer reusing `ui_shell::ui_templates` loading helpers and error reporting instead of building one-off builder lookups in feature code
- if a surface still changes often or derives its structure from controller snapshots, keep it Rust-built and document why instead of forcing template usage

## Main Risks

- migrating too early can freeze shell evolution before workflow design settles
- template IDs and bindings can become brittle if naming rules are weak
- composite template refactors can blur ownership if behavior starts leaking into widget setup code
- excessive template abstraction can make the shell harder, not easier, to trace
- over-styling with CSS can still produce fragile shells even after template migration if GTK-native patterns are ignored

## Validation Requirements

- template load coverage for any reusable or high-value template-backed widget
- regression checks that controller wiring still reflects live application state
- manual validation for CSS class application, accessibility labels, and widget lookup correctness
- shell behavior checks to ensure no document ownership moves into GTK widgets
- review stable shell surfaces against GNOME HIG expectations where that improves clarity and consistency

## Success Condition

PhotoTux gains a cleaner, more maintainable GTK shell structure in stable areas while preserving its current architecture boundaries and keeping dynamic behavior where Rust remains the better tool.

## References

- GTK4 Rust Book: `https://gtk-rs.org/gtk4-rs/stable/latest/book/`
- GTK4 Composite Templates: `https://gtk-rs.org/gtk4-rs/git/book/composite_templates.html`
- `CompositeTemplate` derive macro: `https://docs.rs/gtk4-macros/latest/gtk4_macros/derive.CompositeTemplate.html`
- GTK4 CSS properties reference: `https://docs.gtk.org/gtk4/css-properties.html`
- GNOME Human Interface Guidelines: `https://developer.gnome.org/hig/`
- Blueprint templates reference: `https://gnome.pages.gitlab.gnome.org/blueprint-compiler/reference/templates.html`
- PhotoTux UI research note: `docs/research/research-ui-design.md`
