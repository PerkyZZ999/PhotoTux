# Shell Usability And Native Workflows Tasks

## Purpose

This task list turns the shell-usability plan into an implementation sequence focused on daily-use workflow polish.

## Principles

- keep command routing centralized in `app_core`
- keep long-running file operations off the UI thread
- preserve the fixed-layout shell rather than introducing docking
- prioritize daily-use friction reduction over visual ornament

## Task List

### SHELL01 - Replace quick-save-only behavior with full project save flow

- [x] Status: completed
- Outcome: saving no longer depends on implicit current-directory behavior
- Includes:
  - save-as routing
  - overwrite handling
  - dirty-state integration
  - recovery-file handling alignment
- Depends on: none
- Done when:
  - project save behavior is complete enough for normal use

Progress notes:
- `app_core` now distinguishes between save to an existing project path and explicit save-as, instead of silently deriving a first-save target from the working directory
- `ui_shell` now exposes `Save As...` through the File menu and routes unsaved `Save` and `Ctrl+S` interactions through a native GTK save dialog
- GTK native save flows now enable overwrite confirmation and normalize the saved extension to `.ptx`
- the active document tab now reflects dirty state directly, which closes the loop between unsaved edits and save behavior

### SHELL02 - Add open document flow

- [x] Status: completed
- Outcome: document loading is a first-class shell action
- Includes:
  - native file picker integration
  - recent path handling if appropriate
  - job-system integration for load
  - shell status and error messaging
- Depends on: none
- Done when:
  - users can open existing documents through normal shell workflows

Progress notes:
- the File menu now routes project open through a native GTK file picker restricted to `.ptx` documents
- project loading runs through the existing user-visible job system path in `app_core`, not on the UI thread
- controller status text now reflects open success and failure states cleanly enough for normal use
- recent-path handling is still deferred

### SHELL03 - Add explicit export workflow

- [x] Status: completed
- Outcome: flattened export becomes a real user-facing command set
- Includes:
  - export-format selection
  - path/overwrite handling
  - worker/job integration for export
  - export error presentation
- Depends on: none
- Done when:
  - export no longer depends on internal helpers only

Progress notes:
- the File menu now exposes explicit PNG, JPEG, and WebP export commands with native GTK save dialogs
- export commands suggest format-appropriate filenames, enforce matching file extensions, and enable overwrite confirmation
- export continues to run through the user-visible job system path with concise success and failure status text in the shell

### SHELL04 - Replace placeholder menu bar with real command routing

- [x] Status: completed
- Outcome: menus reflect actual editor capabilities
- Includes:
  - File/Edit/Layer/Select/View menu routing
  - command availability state
  - menu/shortcut consistency checks
- Depends on: SHELL01, SHELL02, SHELL03
- Done when:
  - the menu bar is no longer decorative

Progress notes:
- the File menu is now controller-backed for open, import, save, save-as, and export flows
- Edit, Layer, Select, and View now open real popover menus that route into existing controller or canvas commands instead of remaining fully decorative
- menu-item sensitivity now updates from live controller state when those menus open, including undo/redo availability, layer move/delete affordances, and selection-dependent actions
- Image and Filter now expose real controller-backed adjustment and transform actions instead of remaining static top-level labels
- Window now routes to live shell-panel visibility actions, and Help now exposes About plus a keyboard-shortcuts reference dialog
- the consistency sweep for menu labels versus shortcut hints is now complete for the shell commands that have real keyboard shortcuts, including file, edit, selection, and view actions

### SHELL05 - Improve recovery UX beyond passive status text

- [x] Status: completed
- Outcome: recovery behavior is visible and clear during startup
- Includes:
  - explicit recover/discard workflow
  - shell messaging for autosave state
  - result handling for recovery choice
- Depends on: none
- Done when:
  - crash recovery is understandable to a normal user without logs

Progress notes:
- `app_core` now exposes explicit recovery-offer state to the shell instead of only leaving recovery detection in controller status text
- startup recovery no longer relies on implicit auto-load behavior; `ui_shell` now presents an explicit recover-or-discard prompt when a recovery file is available
- recovery actions stay routed through `app_core`: recover uses the existing background recovery-load path, while discard removes the autosave file and clears the pending recovery offer cleanly
- recovery-focused controller tests now cover autosave creation, explicit recovery load, and recovery discard behavior

### SHELL06 - Add unsaved-change close protection

- [x] Status: completed
- Outcome: normal document closing no longer risks avoidable loss
- Includes:
  - dirty-state prompt behavior
  - close/cancel/save routing
  - interaction with recovery files
- Depends on: SHELL01, SHELL05
- Done when:
  - closing a dirty document behaves predictably and safely

Progress notes:
- `ui_shell` now intercepts window close requests for dirty documents and presents a native save-discard-cancel confirmation flow instead of allowing silent close
- save-on-close routes through the existing project save behavior: saved documents close after the async save completes, while first-save documents go through the native save dialog before closing
- discard-on-close clears any recovery file through `app_core` before allowing the window to close, which avoids stale autosave prompts on the next launch
- manual Linux-native validation of this close flow is still part of `SHELL10`, but the routing and safety behavior are now implemented

### SHELL07 - Improve document tab behavior

- [x] Status: completed
- Outcome: multi-document use becomes intentional instead of cosmetic
- Includes:
  - dirty indicator on tabs
  - tab close affordances if multi-document support is active
  - tab title/path handling
- Depends on: SHELL02, SHELL06
- Done when:
  - document tabs communicate state clearly

Progress notes:
- the active document tab now focuses on document identity and dirty state instead of repeating zoom and color-mode data that already belong in the status bar
- the tab now exposes project-path context through tooltip text, while unsaved documents are labeled clearly as unsaved in that same surface
- misleading inactive multi-document affordances were reduced: the decorative close glyph was removed from the single-document tab, and the add-tab button is now disabled with an explanatory tooltip until real multi-document support exists

### SHELL08 - Improve command discoverability in the shell

- [x] Status: completed
- Outcome: power features are easier to learn without external docs
- Includes:
  - shortcut hints in menus or labels
  - richer status/help text where useful
  - properties-panel command affordance cleanup
- Depends on: SHELL04
- Done when:
  - major commands are discoverable through the UI itself

Progress notes:
- top-level menus already exposed shortcut hints where real bindings exist, and the shell now extends that discoverability into the tool rail by showing tool shortcuts directly in tooltips
- the properties panel now includes tool-shortcut context, richer command tooltips on action chips, and compact workflow hint rows for save, zoom, selection, and transform interactions
- the status bar now includes contextual tool/help guidance instead of a static fake cursor readout, which keeps common editing shortcuts visible during normal use

### SHELL09 - Add clearer operation-status and error surfaces

- [x] Status: completed
- Outcome: file and job behavior becomes easier to understand during normal use
- Includes:
  - save/export/open progress messaging
  - concise error notifications
  - status bar cleanup for operation state
- Depends on: SHELL01, SHELL02, SHELL03
- Done when:
  - common operation states are visible without reading logs

Progress notes:
- the status bar no longer overloads the color-mode label with every operation message; operation feedback now lives in a dedicated centered status-notice surface
- status notices are now visually classified for busy, success, warning, and error states, making save/open/export/recovery activity easier to parse at a glance
- the remaining status-bar labels were cleaned up so document metrics, zoom, mode, and contextual guidance each have a distinct role instead of being mixed into a single text blob

### SHELL10 - Validate Linux-native workflow behavior

- [ ] Status: not started
- Outcome: shell polish is verified on the target platform rather than assumed
- Includes:
  - Wayland dialog and focus checks
  - fractional-scaling validation

Progress notes:
- an attempted KWin MCP validation run on 2026-03-16 could not start an isolated session in this environment because the host lacked a reachable `org.kde.KWin` D-Bus service, so `SHELL10` remains blocked rather than partially validated
- the shell code is ready for this validation pass, but the actual dialog, focus, and scaling checks still need to be run on a KDE Plasma Wayland setup with working KWin MCP session support
  - shortcut and menu validation
  - recovery and close-prompt manual checks
- Depends on: SHELL04, SHELL05, SHELL06, SHELL09
- Done when:
  - daily-use shell behavior feels native enough for repeated use

## Suggested Execution Order

1. SHELL01
2. SHELL02
3. SHELL03
4. SHELL04
5. SHELL05
6. SHELL06
7. SHELL07
8. SHELL08
9. SHELL09
10. SHELL10

## Notes

- If native dialog integration causes UI-thread blocking, route more of the surrounding operation through the existing job system rather than pushing logic into widgets.