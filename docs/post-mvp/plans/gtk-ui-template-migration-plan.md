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