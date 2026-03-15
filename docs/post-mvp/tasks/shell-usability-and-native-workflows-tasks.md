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

- [ ] Status: not started
- Outcome: saving no longer depends on implicit current-directory behavior
- Includes:
  - save-as routing
  - overwrite handling
  - dirty-state integration
  - recovery-file handling alignment
- Depends on: none
- Done when:
  - project save behavior is complete enough for normal use

### SHELL02 - Add open document flow

- [ ] Status: not started
- Outcome: document loading is a first-class shell action
- Includes:
  - native file picker integration
  - recent path handling if appropriate
  - job-system integration for load
  - shell status and error messaging
- Depends on: none
- Done when:
  - users can open existing documents through normal shell workflows

### SHELL03 - Add explicit export workflow

- [ ] Status: not started
- Outcome: flattened export becomes a real user-facing command set
- Includes:
  - export-format selection
  - path/overwrite handling
  - worker/job integration for export
  - export error presentation
- Depends on: none
- Done when:
  - export no longer depends on internal helpers only

### SHELL04 - Replace placeholder menu bar with real command routing

- [ ] Status: not started
- Outcome: menus reflect actual editor capabilities
- Includes:
  - File/Edit/Layer/Select/View menu routing
  - command availability state
  - menu/shortcut consistency checks
- Depends on: SHELL01, SHELL02, SHELL03
- Done when:
  - the menu bar is no longer decorative

### SHELL05 - Improve recovery UX beyond passive status text

- [ ] Status: not started
- Outcome: recovery behavior is visible and clear during startup
- Includes:
  - explicit recover/discard workflow
  - shell messaging for autosave state
  - result handling for recovery choice
- Depends on: none
- Done when:
  - crash recovery is understandable to a normal user without logs

### SHELL06 - Add unsaved-change close protection

- [ ] Status: not started
- Outcome: normal document closing no longer risks avoidable loss
- Includes:
  - dirty-state prompt behavior
  - close/cancel/save routing
  - interaction with recovery files
- Depends on: SHELL01, SHELL05
- Done when:
  - closing a dirty document behaves predictably and safely

### SHELL07 - Improve document tab behavior

- [ ] Status: not started
- Outcome: multi-document use becomes intentional instead of cosmetic
- Includes:
  - dirty indicator on tabs
  - tab close affordances if multi-document support is active
  - tab title/path handling
- Depends on: SHELL02, SHELL06
- Done when:
  - document tabs communicate state clearly

### SHELL08 - Improve command discoverability in the shell

- [ ] Status: not started
- Outcome: power features are easier to learn without external docs
- Includes:
  - shortcut hints in menus or labels
  - richer status/help text where useful
  - properties-panel command affordance cleanup
- Depends on: SHELL04
- Done when:
  - major commands are discoverable through the UI itself

### SHELL09 - Add clearer operation-status and error surfaces

- [ ] Status: not started
- Outcome: file and job behavior becomes easier to understand during normal use
- Includes:
  - save/export/open progress messaging
  - concise error notifications
  - status bar cleanup for operation state
- Depends on: SHELL01, SHELL02, SHELL03
- Done when:
  - common operation states are visible without reading logs

### SHELL10 - Validate Linux-native workflow behavior

- [ ] Status: not started
- Outcome: shell polish is verified on the target platform rather than assumed
- Includes:
  - Wayland dialog and focus checks
  - fractional-scaling validation
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