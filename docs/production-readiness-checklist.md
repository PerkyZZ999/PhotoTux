# PhotoTux Production Readiness Checklist

## Purpose

This checklist tracks the remaining work needed to make PhotoTux production-ready from an application-quality perspective.

This file intentionally excludes release and distribution work such as CI, packaging, installers, release tagging, and publish automation. Those belong in a separate release/distribution track.

## Principles

- protect user work before adding polish
- keep startup, interaction, and export behavior trustworthy
- prioritize latency, bounded redraw work, and predictable shell behavior
- prefer measured performance work over speculative optimization
- keep production hardening aligned with the existing document-first architecture
- treat `docs/design-ui/mockup/index.html` and `docs/design-ui/mockup/Screenshot.png` as the UI source of truth, not the older design notes in `docs/design-ui/`
- prefer GtkBuilder with separate `.ui` files for stable shell structure so Rust code can focus on wiring, state, and behavior

## Task List

### PROD01 - Protect dirty documents during open and import flows

- [x] Status: completed
- Outcome: opening a project or importing content no longer risks silent loss of unsaved work
- Includes:
  - save-discard-cancel prompt before replacing a dirty document
  - routing through existing save and recovery behavior
  - open/import regression coverage for dirty and clean states
- Depends on: none
- Done when:
  - document replacement behaves as safely as window close

Progress notes:
- `ui_shell` now routes both menu-driven and shortcut-driven open/import actions through a replacement prompt when the current document is dirty instead of replacing the document immediately.
- the replacement prompt mirrors close protection with save, discard, and cancel options, and it defers the actual open/import action until an async save has completed successfully.
- discarding before replacement now clears the current recovery file before loading the next document, which avoids stale autosave prompts after intentionally abandoning unsaved edits.

### PROD02 - Move autosave and recovery paths to install-safe application storage

- [x] Status: completed
- Outcome: unsaved documents and recovery state do not depend on the process working directory
- Includes:
  - XDG-friendly autosave and recovery storage rules
  - predictable naming and cleanup behavior
  - fallback behavior for unwritable paths
  - save, reopen, and recovery regression coverage for the new storage path
- Depends on: none
- Done when:
  - recovery works the same from a source checkout, a desktop launcher, or a packaged install

Progress notes:
- autosave and recovery files now resolve into an application-state recovery directory instead of deriving the unsaved-document path from the process working directory.
- saved documents now use deterministic recovery filenames keyed by both the project filename and a stable hash of the full project path, which avoids collisions while keeping recovery storage predictable.
- recovery storage creation now prefers XDG state directories and falls back to a temp-backed PhotoTux recovery directory when the preferred location is unavailable or not writable.
- controller and file-IO coverage now exercise unsaved autosave, startup recovery loading, discard behavior, state-root naming, and fallback resolution for invalid storage roots.

### PROD03 - Make UI resources install-safe

- [x] Status: completed
- Outcome: icons, logo assets, and any future startup art no longer depend on source-tree-relative paths
- Includes:
  - runtime resource-loading strategy
  - install-safe icon and logo resolution
  - failure behavior when optional assets are missing
- Depends on: none
- Done when:
  - the installed app renders the expected branding and icons without relying on the repository layout

Progress notes:
- `ui_shell` now compiles its logo and shell-icon assets into a bundled GTK resource pack at build time and registers that pack during shell startup instead of resolving assets from `../../assets/...` paths.
- shell image construction now resolves logo and icon resources through resource URIs, which keeps branding and the available bundled icons working the same from a source checkout, launcher, or installed build.
- missing optional icon assets now fall back to a stable theme icon and emit a one-time warning instead of silently depending on repository-relative files that may not exist at runtime.

### PROD04 - Harden project and recovery-file corruption handling

- [x] Status: completed
- Outcome: malformed `.ptx` or autosave files fail clearly instead of risking invalid in-memory state
- Includes:
  - validation for empty or structurally invalid document payloads
  - clearer user-facing failure messaging
  - recovery-file corruption tests
  - stress cases such as interrupted autosave and stale recovery cleanup
- Depends on: none
- Done when:
  - malformed files cannot produce invalid live document state silently

Progress notes:
- `.ptx` loading now validates core structural invariants before a document is restored, including non-zero canvas dimensions, unique layer identities, mask-payload consistency, duplicate tile coordinates, and exact tile payload sizes for both raster and mask data.
- project load failures now carry clearer path-aware context through the file-IO layer, and the async controller surfaces the full error chain so open/recovery failures describe the real corruption cause instead of collapsing into a vague generic failure.
- regression coverage now exercises corrupt project opens, corrupt recovery loads, zero-sized canvas payloads, and truncated tile payloads to keep malformed project data from silently producing invalid live document state.

### PROD05 - Tighten Linux windowing and shell behavior for normal desktop use

- [x] Status: completed
- Outcome: startup, focus, close, move, and resize behavior feel native enough for daily use
- Includes:
  - review of custom undecorated window behavior versus native decorations
  - fix any missing move, resize, and close affordances
  - startup focus and dialog handoff checks
  - shell validation on Wayland and common high-DPI setups
- Depends on: none
- Done when:
  - the shell behaves predictably as a normal Linux desktop application without requiring developer workarounds

Progress notes:
- the custom PhotoTux header now lives in GTK's actual window titlebar slot instead of being embedded as a normal content widget, which restores native client-side titlebar semantics for moving, resizing, maximize/minimize, and compositor integration.
- GTK-managed title buttons are now enabled on the shell header, so normal Linux desktop window controls are available without adding a second, conflicting server-side frame.
- startup focus now explicitly returns to the canvas surface and the canvas widget is focusable/click-focusable, which improves first-interaction behavior after the shell appears.

### PROD06 - Make PSD import truthful and bounded in user-facing workflows

- [x] Status: completed
- Outcome: PSD import behaves predictably even when the sidecar is missing, slow, or returns unsupported structure
- Includes:
  - clearer shell messaging when the sidecar is not configured
  - timeout or cancellation strategy for the sidecar path
  - user-visible diagnostics review for fallback imports
  - regression coverage for sidecar failure modes
- Depends on: none
- Done when:
  - PSD import either succeeds within the documented subset or fails clearly without hanging or misleading the user

Progress notes:
- PSD sidecar execution is now time-bounded, and timed-out or failing helpers are terminated and surfaced as explicit bounded-workflow failures instead of hanging indefinitely.
- shell messaging now makes the current PSD contract clearer: missing sidecar configuration, limited layered-subset imports, and flattened-fallback imports each present distinct user-facing wording instead of collapsing into a generic warning.
- PSD import reporting now distinguishes editable layered-subset imports from flattened composite fallback imports, and regression coverage now exercises sidecar startup failure, timeout handling, flattened fallback reporting, and layered-subset warning reporting.

### PROD07 - Clean up strict lint blockers and runtime hardening warnings

- [x] Status: completed
- Outcome: the application is clean under the intended strict lint policy and the remaining riskier runtime assumptions are deliberate
- Includes:
  - fix current clippy failures
  - review runtime `expect` and panic paths outside test-only code
  - narrow or document any justified exceptions
- Depends on: none
- Done when:
  - strict linting supports the real production-hardening workflow instead of immediately failing on known issues

Progress notes:
- workspace strict linting now passes under `cargo clippy --workspace --all-targets -- -D warnings`, including the earlier `file_io` arithmetic issue and the `ui_shell` text-session API shape that previously tripped `too_many_arguments`.
- text-session updates now flow through a dedicated `ShellTextUpdate` payload instead of a long positional-argument list, which reduces callsite fragility while keeping the current controller boundaries intact.
- test-only hardening cleanup removed remaining `unwrap()` calls that strict clippy flagged in controller tests, and unnecessary clones on `Copy` text-transform values were dropped.
- runtime panic review removed the non-test PSD workspace timestamp `expect(...)` in `file_io` and converted it into a contextual fallible error; the remaining panic/expect sites surfaced in the scan are internal invariants or test-only assertions.

### PROD08 - Add better user-facing error and busy-state surfaces

- [x] Status: completed
- Outcome: failures and long operations are understandable without reading logs
- Includes:
  - clear shell messaging for open, import, save, export, recovery, and filter failures
  - consistent busy-state communication
  - review of which runtime failures should stay internal logs versus user-visible notices
- Depends on: PROD01, PROD04
- Done when:
  - normal users can understand what failed and what to do next from the app itself

Progress notes:
- `app_core` now emits structured shell alerts for the main user-visible failure paths: save, open, import, export, recovery load, recovery discard, unsupported file choices, missing PSD sidecar configuration, and destructive-filter failures.
- `ui_shell` now presents those alerts through a reusable dialog surface while keeping the status bar as the lightweight always-on channel for operation progress and final summaries.
- shell snapshots now expose explicit busy flags for user-visible file jobs and autosave jobs, so status styling reflects real activity instead of inferring busy/error state from message wording alone.
- intentionally non-fatal workflow guards such as “another file operation is already in progress” or “commit/cancel text editing before filtering” remain status-bar-only feedback instead of spawning extra dialogs, which keeps high-frequency interaction errors from becoming noisy.

### PROD09 - Refactor stable shell UI to GtkBuilder and separate `.ui` files

- [x] Status: completed
- Outcome: the UI implementation becomes easier to maintain, test, and restyle without burying widget structure inside large Rust files
- Includes:
  - identify which shell surfaces should migrate first (`ApplicationWindow`, menu/header structure, tool options bar, key panels, dialogs)
  - move stable widget structure into GtkBuilder-backed `.ui` resources
  - keep controller ownership and document-state boundaries unchanged while migrating view structure
  - define builder object IDs, CSS classes, and resource-loading rules clearly enough for automated validation
  - add or extend shell checks for required IDs, metadata, and template wiring
- Depends on: PROD03
- Done when:
  - the major shell surfaces are defined through GtkBuilder-backed `.ui` files and the Rust layer is noticeably less entangled with raw widget construction

Progress notes:
- `ui_shell` now defines the stable shell chrome through dedicated GtkBuilder `.ui` fragments for the titlebar, tool-options strip, document tabs, and status bar, while the previously migrated dialogs and panel groups continue using the same builder-driven pattern.
- the Rust layer now focuses on wiring dynamic behavior into those templates: menu/popover actions, tool selection, icon/resource assignment, controller bindings, and status refresh logic remain in code, but the static widget hierarchy is no longer hand-built inline.
- new template loaders and validation tests now cover the added shell fragments, including required object IDs, CSS classes, and GTK-available structure checks so future UI edits can fail loudly instead of silently drifting.
- the migration keeps explicit fallback builders for the newly templated shell surfaces, which preserves startup resilience while the project continues moving more UI structure out of oversized Rust functions.

### PROD10 - Align the shell layout and styling to the design mockup

- [x] Status: completed
- Outcome: the app visually matches the intended professional shell direction while preserving PhotoTux's real feature set and architecture
- Includes:
  - audit `docs/design-ui/mockup/index.html` and `docs/design-ui/mockup/Screenshot.png` against the current shell
  - update shell layout, pane sizing, chrome, panel hierarchy, spacing, and visual density to better match the mockup direction
  - preserve PhotoTux-specific commands and workflows instead of copying mockup-only controls blindly
  - reconcile titlebar/header, menu bar, options bar, toolbar, canvas framing, right dock, and status bar styling into one consistent system based on the mockup
  - validate the resulting shell at the documented desktop sizes and scaling targets
- Depends on: PROD09
- Done when:
  - the app presents a cohesive shell that clearly reflects the mockup and design tokens without introducing off-scope feature creep

Progress notes:
- the shell chrome now uses a denser Photoshop-like layout direction from the mockup: tighter menu/titlebar spacing, a more form-like options strip, wider right dock proportions, refined toolbar sizing, and more compact panel/header spacing.
- the canvas presentation now matches the mockup much more closely with a darker framed stage, a top-center canvas info pill, and a bottom contextual task bar that exposes real PhotoTux actions instead of placeholder Photoshop-only controls.
- the menu bar now includes a right-side zoom readout, and the shell’s visual system is more consistent across titlebar, menu bar, options bar, document tabs, canvas frame, right sidebar, and status bar.
- the alignment work stayed within PhotoTux’s actual feature set: no mockup-only workflows were added, and the new contextual controls are wired to existing zoom, selection, and layer-editing commands rather than decorative dead buttons.

### PROD11 - Establish a measured performance and latency baseline

- [x] Status: done
- Outcome: performance work is driven by evidence, budgets, and repeatable checks instead of guesswork
- Includes:
  - startup timing baseline
  - medium-canvas painting latency baseline
  - pan/zoom responsiveness baseline
  - export and autosave timing checks
- bounded dirty-region and upload validation for common edits
- Depends on: none
- Done when:
  - the project has named performance budgets and repeatable checks for the main interaction paths
- Progress notes:
  - added named automated headless budgets for controller startup, initial canvas raster, representative medium-canvas brush work, PNG export, autosave, and repeated viewport pan/zoom math.
  - the representative pressure-enabled paint fixture now also asserts a bounded dirty upload set after a 12-stroke medium-canvas pass so common edits cannot silently expand into whole-document invalidation.
  - the baseline budgets live in the test suite and are documented in `docs/testing-strategy.md`, giving PROD12 a concrete floor for optimization work instead of ad hoc profiling.

### PROD12 - Optimize hot paths for fluidity under real workloads

- [x] Status: done
- Outcome: brush strokes, viewport interaction, flattening, and file operations stay smooth under representative load
- Includes:
  - profiling and reducing avoidable work in brush interpolation, tile invalidation, and canvas refresh
  - profiling and reducing avoidable work in pan, zoom, and overlay redraw paths
  - profiling and reducing avoidable work in flatten/export paths used during normal workflows
  - stress validation for longer editing sessions and larger layered documents
- Depends on: PROD11
- Done when:
  - the common editing paths feel consistently fluid on the target Linux environment rather than merely passing correctness tests
- Progress notes:
  - brush-segment cache refresh now coalesces touched tile updates into a single unioned recomposition region instead of walking the full layer hierarchy once per changed tile.
  - move interactions now union old/new affected bounds before refreshing the cached flattened canvas, avoiding duplicate regional recomposition on every pointer update.
  - `app_core` now memoizes layer-panel projection data between shell snapshot refreshes and invalidates it only on layer, group, mask, visibility, opacity, text-commit, selection-target, load, and undo/redo changes instead of rebuilding the sidebar model every time the shell asks for a snapshot.
  - selection, guide, and active-edit-target changes now preserve the cached flattened canvas because they only affect overlays or shell state, avoiding unnecessary full-raster recomposition after non-raster edits.
  - guide projection data is now memoized between snapshot refreshes as well, so repeated shell snapshots reuse the same `ShellGuide` list until actual guide state changes.
  - single-layer visual edits such as blend-mode changes, visibility toggles, and opacity changes now refresh only the affected layer or text bounds in the cached flattened canvas instead of invalidating and rebuilding the whole composite.
  - completing rectangular or freeform selection interactions now preserves the flattened raster cache too, so selection-only edits no longer trigger a useless full recomposite at mouse-up.
  - group visibility toggles and layer-mask add/remove/enable state changes now reuse bounded cached refreshes as well, and the same behavior now holds when those mask-state edits are replayed through undo/redo.
  - move and transform commit/history paths now union the layer's old and new bounds and refresh only that region in the cached flattened canvas, eliminating another full-composite invalidation path during common iteration and undo/redo flows.
  - brush undo/redo and destructive-filter undo/redo now also preserve the cached flattened canvas by refreshing only the touched layer region instead of invalidating the entire composite during history replay.
  - text layer commit, delete, and text history replay now refresh only the union of the text layer's before/after bounds in the cached flattened canvas, removing another full invalidation path from common text-edit workflows.
  - live text and transform previews now start from the committed flattened canvas and recomposite only the affected preview region against a temporary preview document, avoiding full-canvas flatten work while those interactive sessions are active.
  - empty raster-layer creation now keeps the cached flattened canvas intact, while duplicate/delete raster-layer actions refresh only the affected layer bounds instead of forcing another full composite.
  - creating a brand-new default group around the active layer now preserves the cached flattened canvas through both the command and its undo/redo history path because that hierarchy wrapper is visually a no-op.
  - ungrouping that same default single-child wrapper now preserves the cached flattened canvas too, while still falling back to full invalidation for groups whose visibility or opacity could change the composite.
  - live text-layer dragging now mirrors raster-layer move handling by refreshing only the union of the text layer's old and new bounds in the cached flattened canvas instead of dropping the whole composite on each pointer update.
  - the remaining full invalidations are now concentrated in generic dirty-document paths and correctness-first broad structural changes, rather than in the common interactive paint, move, text, undo/redo, export, or snapshot-refresh hot paths.

### PROD13 - Add startup splash screen and renderer warm-up path

- [x] Status: done
- Outcome: startup feels intentional and the first real canvas interaction pays less one-time initialization cost
- Includes:
  - a borderless splash window using the official logo
  - progress feedback for startup phases
  - warm-up/preload path for renderer initialization, shader or pipeline compilation, and any other safe startup caches
  - clean handoff from splash screen to the main shell without focus glitches
  - startup timing validation for cold and warm launches
- Depends on: PROD03, PROD11
- Done when:
  - the app can show a polished startup splash while safely preloading startup-critical rendering work and avoiding a jarring first-use hitch
- Progress notes:
  - startup now defers shell construction by one GTK idle turn so a borderless logo splash can paint first, then steps through real startup phases while the shell and canvas host initialize.
  - the canvas host now performs a one-shot offscreen warm-up render before the main window is presented, so the first visible workspace frame arrives with renderer setup and an initial canvas frame already prepared.
  - splash teardown is now tied to the main workspace window actually mapping instead of an immediate close after `present()`, reducing the chance of focus glitches during startup handoff on slower systems.
  - startup logging now emits a single structured `startup_summary` line with shell-init, warm-up, handoff, total, and renderer-warmed fields, and the handoff timing is recorded when the main window actually maps so cold and warm launch validation reflects the real visible transition.
  - the main-window map callback is now a one-shot handoff path that focuses the canvas and runs the startup teardown/metrics hook exactly once, keeping focus transfer and visible startup completion aligned.

### PROD14 - Keep large modules from becoming release-risk bottlenecks

- [x] Status: done
- Outcome: the most critical controller, shell, and persistence code is easier to reason about and less fragile during hardening
- Includes:
  - split especially large modules along real ownership boundaries
  - isolate startup, file-workflow, and performance-critical code paths where helpful
  - keep test coverage intact during refactors
- Depends on: none
- Done when:
  - future hardening work no longer has to land inside a few oversized catch-all files
- Progress notes:
  - extracted the splash, warm-up, and startup-timing orchestration out of `crates/ui_shell/src/lib.rs` into a dedicated `crates/ui_shell/src/startup.rs` module so the new startup path is isolated instead of growing the main shell module further.
  - extracted the top-level GTK shell/window assembly (`build_ui`, titlebar construction, and workspace body assembly) into `crates/ui_shell/src/layout.rs`, keeping startup orchestration and shell composition out of the same catch-all source file.
  - extracted the menu-bar builder cluster into `crates/ui_shell/src/menus.rs`, moving the large file/edit/image/layer/select/filter/view/window/help menu construction logic out of `ui_shell/src/lib.rs`.
  - extracted the open/import/export/save chooser helpers and shared info-dialog path into `crates/ui_shell/src/file_workflow.rs`, so production-critical file workflows no longer live inside the main shell module and remain reusable from both shortcuts and menu actions.
  - extracted the document-region, workspace/sidebar assembly, document-tab fallback, and shared panel/status shell builders into `crates/ui_shell/src/shell_chrome.rs`, keeping the remaining shell composition code grouped by ownership instead of spread across `ui_shell/src/lib.rs`.
  - extracted the shared icon/resource loaders, menu widget builders, and tool-icon/shortcut mapping into `crates/ui_shell/src/ui_support.rs`, so the remaining shell modules now depend on a single support layer instead of reaching back into `ui_shell/src/lib.rs` for common chrome helpers.
  - folded the remaining tool-options bar, tool rail, and swatch-stack builders into `crates/ui_shell/src/shell_chrome.rs`, finishing the move of the shell-assembly widgets out of the main shell module.
  - extracted the status/notice copy and styling logic into `crates/ui_shell/src/status_presenter.rs`, so status-surface formatting and tests now depend on a presentation-focused module instead of `ui_shell/src/lib.rs`.
  - extracted the canvas widget wiring, viewport/render host state, and brush-preview geometry into `crates/ui_shell/src/canvas_host.rs`, isolating the renderer-facing interaction loop from the rest of the shell and removing another large ownership cluster from `ui_shell/src/lib.rs`.
  - extracted the color, properties, layers, and history panel refresh/render cluster into `crates/ui_shell/src/panels.rs`, moving the remaining panel presentation logic out of `ui_shell/src/lib.rs` and leaving the root shell module focused on orchestration instead of widget-specific refresh code.
  - after the startup, layout, menu, file workflow, shell chrome, support, status, canvas host, and panels splits, new hardening work no longer has to land in a single monolithic `ui_shell` source file, which closes the release-risk bottleneck this item targeted.

## Suggested Execution Order

1. PROD01
2. PROD02
3. PROD03
4. PROD04
5. PROD07
6. PROD08
7. PROD05
8. PROD06
9. PROD09
10. PROD10
11. PROD11
12. PROD12
13. PROD13
14. PROD14

## Notes

- Release and distribution work stays separate on purpose; this checklist is about making the app itself trustworthy and polished.
- The splash screen should be used to hide real startup work, not to add artificial delay.
- If a performance optimization conflicts with correctness or data safety, correctness still wins first.
- `docs/design-ui/mockup/index.html` and `docs/design-ui/mockup/Screenshot.png` are the UI reference to follow. The older files in `docs/design-ui/` are not the source of truth for the current UI direction.
