# GTK UI Template Migration Tasks

## Purpose

This task list turns the GTK `.ui` migration plan into a staged refactor path that improves maintainability without destabilizing shell behavior.

## Principles

- use `.ui` files for stable structure, not for replacing GTK CSS or Rust behavior
- keep controller wiring and document ownership in Rust
- keep appearance in class-based GTK CSS rather than per-widget or inline styling logic
- migrate low-risk surfaces before touching core shell composition
- prefer selective conversion over all-at-once rewrites
- prefer `CompositeTemplate` for complex stable custom widgets instead of ad hoc builder lookups
- follow GNOME HIG patterns first, then layer PhotoTux personality on top

## Task List

### UIT01 - Define `.ui` migration rules and naming conventions

- [x] Status: completed
- Outcome: future template work follows one consistent model
- Includes:
  - decide where `.ui` files live under `ui_shell`
  - define file naming, widget ID naming, and CSS class conventions
  - define when to use `gtk::Builder` versus `CompositeTemplate`
  - codify the three-layer rule: layout in `.ui`, behavior in Rust, appearance in CSS
- Depends on: none
- Done when:
  - template-backed shell work has explicit project rules instead of ad hoc choices

Progress notes:
- the GTK template migration plan now defines `crates/ui_shell/src/ui/` as the single home for template-backed shell structure, with `dialogs/`, `panels/`, and later `fragments/` subdirectories instead of ad hoc per-surface placement
- `.ui` files now have concrete naming rules: template files stay kebab-case, while root object IDs and child lookup IDs stay snake_case and name the owning surface first
- the loading split is now explicit before any refactor starts: `gtk::Builder` is the default for simple one-off dialogs and low-risk surfaces, while `CompositeTemplate` is reserved for reusable or stateful panel shells and custom widgets that benefit from typed `TemplateChild` bindings
- the migration track now has an explicit ownership model: static shell structure lives in `.ui`, behavior and controller wiring stay in Rust, appearance stays in GTK CSS, and the top-level workspace shell plus dynamic snapshot-driven surfaces remain Rust-built until a later task proves a better boundary

### UIT02 - Define shell styling rules for template-backed UI

- [x] Status: completed
- Outcome: template-backed UI follows a lightweight GTK-native presentation model
- Includes:
  - define class-based CSS rules for template-backed widgets
  - define when CSS classes versus widget names should be used
  - define a rule against using CSS to re-implement native widget behavior
  - align stable shell expectations with GNOME HIG where practical
- Depends on: UIT01
- Done when:
  - template-backed UI work has consistent styling rules that remain GTK-native and lightweight

Progress notes:
- the GTK template migration plan now defines class-first styling rules for template-backed widgets, explicitly preferring existing shell classes such as `panel-group`, `panel-group-body`, `panel-tab`, and `tool-chip` before introducing any template-local CSS
- widget names are now reserved for stable roots, debugging, accessibility review, and tests or automation rather than becoming the primary styling mechanism for normal shell appearance
- the track now explicitly rejects ID-heavy CSS and behavior-shaped styling; GTK keeps native widget behavior while CSS stays shallow and class-oriented
- GNOME-HIG-aligned defaults are now written down for this migration track, including restrained dialog action counts, clear text hierarchy, and disabled inactive controls instead of fake interactive shells

### UIT03 - Add a minimal template loading foundation in `ui_shell`

- [x] Status: completed
- Outcome: the shell can load `.ui` files predictably and testably
- Includes:
  - helper utilities for loading builder files or embedded template strings/resources
  - error handling for missing IDs and bad template loads
  - initial tests for template loading behavior where practical
- Depends on: UIT01, UIT02
- Done when:
  - one template-backed surface can be loaded safely through shared infrastructure

Progress notes:
- `ui_shell` now has a dedicated `ui_templates` module that embeds `.ui` files from `crates/ui_shell/src/ui/` with `include_str!`, keeping template loading repo-managed instead of depending on arbitrary runtime file paths
- template loading now validates required builder IDs through a shared helper, so missing widgets fail with the template path and missing object ID rather than degrading into ad hoc `None` lookups
- the first embedded templates now live under the documented `dialogs/` and `panels/` subdirectories, which proves the UIT01 directory convention in real code instead of only in docs
- focused unit coverage now validates embedded template source metadata in the default test suite, while GTK builder-runtime checks are kept as ignored manual tests because GTK widget construction must run on the real process main thread and cannot be exercised through the standard Cargo test harness here

### UIT04 - Migrate one low-risk dialog to a `.ui` template

- [x] Status: completed
- Outcome: the migration pattern is proven on a low-risk shell surface
- Includes:
  - choose a stable dialog or simple shell surface
  - move static structure into `.ui`
  - keep actions, signal routing, and controller interaction in Rust
- Depends on: UIT03
- Done when:
  - a template-backed surface behaves identically to the Rust-built version

Progress notes:
- the reusable info-dialog surface used by the Help menu now loads its content structure from `src/ui/dialogs/info-dialog.ui` instead of building all labels and actions inline in Rust
- dialog lifecycle still stays in Rust: `show_info_dialog` creates the transient GTK dialog, binds the title and detail text, wires the close action, and owns explicit error reporting if the template fails to load
- the migrated template now carries the static label hierarchy, close button metadata, widget names, and CSS class attachment, which proves the intended layout-in-XML and behavior-in-Rust split on a low-risk surface

### UIT05 - Validate CSS and accessibility behavior for template-backed widgets

- [x] Status: completed
- Outcome: template migration does not silently regress appearance or usability metadata
- Includes:
  - confirm CSS classes and widget names are applied as expected
  - confirm labels, tooltips, and accessibility-relevant names remain intact
  - document any GTK template caveats found during migration
- Depends on: UIT04
- Done when:
  - template-backed widgets integrate cleanly with the existing CSS and shell expectations

Progress notes:
- the first template-backed dialog now carries explicit widget names plus class-based styling hooks, and the default unit suite verifies that the embedded `.ui` sources retain the required IDs, classes, and close-button tooltip metadata
- the migrated info dialog keeps human-readable text hierarchy intact by binding the title, body, and secondary detail labels explicitly in Rust instead of depending on implicit shell state
- the first discovered template-validation caveat is now explicit: real GTK builder object checks require GTK initialization on the real process main thread, so those runtime checks are kept as ignored manual tests while the default suite still validates template metadata headlessly

### UIT06 - Migrate one stable sidebar panel shell

- [x] Status: completed
- Outcome: stable panel structure becomes easier to maintain without changing app behavior
- Includes:
  - choose one panel with mostly static structure
  - move static rows/containers/header structure into `.ui`
  - prefer `CompositeTemplate` if the panel is complex enough to justify typed template children
  - retain Rust-driven updates for live values and controller actions
- Depends on: UIT05
- Done when:
  - one real panel shell is template-backed and still reflects live application state correctly

Progress notes:
- the History panel is now the first template-backed sidebar shell, with its root container, header, active tab button, and body container loaded from `src/ui/panels/history-panel.ui`
- live application behavior still stays in Rust: `ShellUiState::refresh_history_panel` continues to rebuild the history rows and action buttons from controller snapshots, while the template owns only the stable structural shell
- this first panel migration stayed intentionally builder-based rather than introducing `CompositeTemplate`, because the History shell is structurally simple and does not yet justify a subclass-backed widget

### UIT07 - Extract shared template-backed shell fragments only where repetition is proven

- [x] Status: completed
- Outcome: duplication drops without creating unnecessary abstraction
- Includes:
  - identify repeated panel/header/row patterns
  - extract reusable widgets or fragments only where maintenance cost clearly improves
  - use `CompositeTemplate` and `TemplateChild` where that makes custom shell widgets clearer
  - keep the resulting API readable from Rust call sites
- Depends on: UIT06
- Done when:
  - template reuse reduces duplication without obscuring shell ownership

Progress notes:
- the repeated sidebar panel shell is now extracted into a shared builder-backed fragment at `crates/ui_shell/src/ui/fragments/panel-group.ui`, replacing the earlier history-only panel template with one reusable structural path
- `ui_shell::ui_templates` now configures that fragment from Rust with surface-specific tab labels, widget names, spacing, and vexpand settings, which keeps the API readable at call sites while avoiding per-panel copy-paste XML
- this task intentionally stopped short of introducing `CompositeTemplate`: the shared panel shell remains structurally simple enough that builder-backed reuse is still clearer than adding subclass machinery

### UIT08 - Migrate additional stable dialogs and panels selectively

- [x] Status: completed
- Outcome: the template approach expands only where it keeps paying off
- Includes:
  - migrate additional low-volatility surfaces
  - preserve Rust-built construction for dynamic or frequently changing areas
  - keep refactors incremental and reviewable
- Depends on: UIT07
- Done when:
  - the stable shell surfaces have a deliberate split between template-backed and Rust-built construction

Progress notes:
- the shared panel-group fragment now backs the Color, Properties, Layers, and History sidebar shells instead of limiting template-backed panel structure to History alone
- the migration stayed selective: the outer panel shells are now template-backed, while the dynamic panel contents and action wiring still refresh from Rust-owned controller snapshots
- the current split is now deliberate rather than opportunistic: stable dialogs and repeated panel shells can use `.ui`, but frequently changing or snapshot-shaped surfaces remain Rust-built

### UIT09 - Reassess top-level shell composition boundaries

- [x] Status: completed
- Outcome: the project intentionally decides what should never be moved to `.ui`
- Includes:
  - review menu bar, workspace shell, and panel composition boundaries
  - decide whether any top-level shell sections benefit from partial templating
  - explicitly document why dynamic sections remain Rust-built
- Depends on: UIT08
- Done when:
  - the top-level shell strategy is documented rather than implied

Progress notes:
- the GTK migration plan now records the top-level shell decision explicitly: root window assembly, menus, tool rail, workspace composition, document tabs, status bar, and the canvas host remain Rust-built for now
- the same notes now explain why: those surfaces still own dynamic composition, input controllers, renderer-adjacent state, or snapshot-shaped structure that would become less clear if moved into declarative markup
- this closes the ambiguity around “template everything later”; the project now has an intentional long-term boundary instead of an implied one

### UIT10 - Add regression coverage and migration notes

- [x] Status: completed
- Outcome: template-backed UI stays maintainable and understandable over time
- Includes:
  - test coverage for template loading or widget lookup where practical
  - manual validation notes for CSS/application-state integration
  - manual validation notes for GNOME HIG consistency where relevant
  - migration notes for future contributors extending template-backed surfaces
- Depends on: UIT09
- Done when:
  - the `.ui` migration path is repeatable and documented for future work

Progress notes:
- template-focused regression coverage now lives in `crates/ui_shell/src/ui_templates.rs`, where the default suite validates embedded `.ui` source metadata, required IDs, CSS classes, and stable widget names for the current template-backed surfaces
- the migration track now records the GTK main-thread caveat explicitly: true builder-runtime checks remain ignored or manual coverage because they require the real process main thread rather than the default Cargo test harness
- contributor-facing migration notes now live in the GTK migration plan, including where new `.ui` files belong, when to keep a surface Rust-built, and how to extend the shared template-loading path without inventing one-off builder code

## Suggested Execution Order

1. UIT01
2. UIT02
3. UIT03
4. UIT04
5. UIT05
6. UIT06
7. UIT07
8. UIT08
9. UIT09
10. UIT10

## Notes

- If template migration starts increasing indirection without reducing boilerplate, stop and keep that surface Rust-built.
- If a surface is still changing weekly, it is probably a poor candidate for `.ui` migration.
- Reserve WGPU for the canvas and rendering-heavy path; surrounding panels, controls, and forms should stay normal GTK widgets.
- Treat `docs/research/research-ui-design.md` and the GTK links in `docs/useful-links.md` as the reference baseline for this migration track.

## References

- GTK4 Rust Book: `https://gtk-rs.org/gtk4-rs/stable/latest/book/`
- GTK4 Composite Templates: `https://gtk-rs.org/gtk4-rs/git/book/composite_templates.html`
- `CompositeTemplate` derive macro: `https://docs.rs/gtk4-macros/latest/gtk4_macros/derive.CompositeTemplate.html`
- GTK4 CSS properties reference: `https://docs.gtk.org/gtk4/css-properties.html`
- GNOME Human Interface Guidelines: `https://developer.gnome.org/hig/`
- Blueprint templates reference: `https://gnome.pages.gitlab.gnome.org/blueprint-compiler/reference/templates.html`
- PhotoTux UI research note: `docs/research/research-ui-design.md`
