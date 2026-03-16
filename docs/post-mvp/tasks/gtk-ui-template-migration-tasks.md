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

- [ ] Status: not started
- Outcome: future template work follows one consistent model
- Includes:
  - decide where `.ui` files live under `ui_shell`
  - define file naming, widget ID naming, and CSS class conventions
  - define when to use `gtk::Builder` versus `CompositeTemplate`
  - codify the three-layer rule: layout in `.ui`, behavior in Rust, appearance in CSS
- Depends on: none
- Done when:
  - template-backed shell work has explicit project rules instead of ad hoc choices

### UIT02 - Define shell styling rules for template-backed UI

- [ ] Status: not started
- Outcome: template-backed UI follows a lightweight GTK-native presentation model
- Includes:
  - define class-based CSS rules for template-backed widgets
  - define when CSS classes versus widget names should be used
  - define a rule against using CSS to re-implement native widget behavior
  - align stable shell expectations with GNOME HIG where practical
- Depends on: UIT01
- Done when:
  - template-backed UI work has consistent styling rules that remain GTK-native and lightweight

### UIT03 - Add a minimal template loading foundation in `ui_shell`

- [ ] Status: not started
- Outcome: the shell can load `.ui` files predictably and testably
- Includes:
  - helper utilities for loading builder files or embedded template strings/resources
  - error handling for missing IDs and bad template loads
  - initial tests for template loading behavior where practical
- Depends on: UIT01, UIT02
- Done when:
  - one template-backed surface can be loaded safely through shared infrastructure

### UIT04 - Migrate one low-risk dialog to a `.ui` template

- [ ] Status: not started
- Outcome: the migration pattern is proven on a low-risk shell surface
- Includes:
  - choose a stable dialog or simple shell surface
  - move static structure into `.ui`
  - keep actions, signal routing, and controller interaction in Rust
- Depends on: UIT03
- Done when:
  - a template-backed surface behaves identically to the Rust-built version

### UIT05 - Validate CSS and accessibility behavior for template-backed widgets

- [ ] Status: not started
- Outcome: template migration does not silently regress appearance or usability metadata
- Includes:
  - confirm CSS classes and widget names are applied as expected
  - confirm labels, tooltips, and accessibility-relevant names remain intact
  - document any GTK template caveats found during migration
- Depends on: UIT04
- Done when:
  - template-backed widgets integrate cleanly with the existing CSS and shell expectations

### UIT06 - Migrate one stable sidebar panel shell

- [ ] Status: not started
- Outcome: stable panel structure becomes easier to maintain without changing app behavior
- Includes:
  - choose one panel with mostly static structure
  - move static rows/containers/header structure into `.ui`
  - prefer `CompositeTemplate` if the panel is complex enough to justify typed template children
  - retain Rust-driven updates for live values and controller actions
- Depends on: UIT05
- Done when:
  - one real panel shell is template-backed and still reflects live application state correctly

### UIT07 - Extract shared template-backed shell fragments only where repetition is proven

- [ ] Status: not started
- Outcome: duplication drops without creating unnecessary abstraction
- Includes:
  - identify repeated panel/header/row patterns
  - extract reusable widgets or fragments only where maintenance cost clearly improves
  - use `CompositeTemplate` and `TemplateChild` where that makes custom shell widgets clearer
  - keep the resulting API readable from Rust call sites
- Depends on: UIT06
- Done when:
  - template reuse reduces duplication without obscuring shell ownership

### UIT08 - Migrate additional stable dialogs and panels selectively

- [ ] Status: not started
- Outcome: the template approach expands only where it keeps paying off
- Includes:
  - migrate additional low-volatility surfaces
  - preserve Rust-built construction for dynamic or frequently changing areas
  - keep refactors incremental and reviewable
- Depends on: UIT07
- Done when:
  - the stable shell surfaces have a deliberate split between template-backed and Rust-built construction

### UIT09 - Reassess top-level shell composition boundaries

- [ ] Status: not started
- Outcome: the project intentionally decides what should never be moved to `.ui`
- Includes:
  - review menu bar, workspace shell, and panel composition boundaries
  - decide whether any top-level shell sections benefit from partial templating
  - explicitly document why dynamic sections remain Rust-built
- Depends on: UIT08
- Done when:
  - the top-level shell strategy is documented rather than implied

### UIT10 - Add regression coverage and migration notes

- [ ] Status: not started
- Outcome: template-backed UI stays maintainable and understandable over time
- Includes:
  - test coverage for template loading or widget lookup where practical
  - manual validation notes for CSS/application-state integration
  - manual validation notes for GNOME HIG consistency where relevant
  - migration notes for future contributors extending template-backed surfaces
- Depends on: UIT09
- Done when:
  - the `.ui` migration path is repeatable and documented for future work

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