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

## Task List

### PROD01 - Protect dirty documents during open and import flows

- [ ] Status: not started
- Outcome: opening a project or importing content no longer risks silent loss of unsaved work
- Includes:
  - save-discard-cancel prompt before replacing a dirty document
  - routing through existing save and recovery behavior
  - open/import regression coverage for dirty and clean states
- Depends on: none
- Done when:
  - document replacement behaves as safely as window close

### PROD02 - Move autosave and recovery paths to install-safe application storage

- [ ] Status: not started
- Outcome: unsaved documents and recovery state do not depend on the process working directory
- Includes:
  - XDG-friendly autosave and recovery storage rules
  - predictable naming and cleanup behavior
  - fallback behavior for unwritable paths
  - save, reopen, and recovery regression coverage for the new storage path
- Depends on: none
- Done when:
  - recovery works the same from a source checkout, a desktop launcher, or a packaged install

### PROD03 - Make UI resources install-safe

- [ ] Status: not started
- Outcome: icons, logo assets, and any future startup art no longer depend on source-tree-relative paths
- Includes:
  - runtime resource-loading strategy
  - install-safe icon and logo resolution
  - failure behavior when optional assets are missing
- Depends on: none
- Done when:
  - the installed app renders the expected branding and icons without relying on the repository layout

### PROD04 - Harden project and recovery-file corruption handling

- [ ] Status: not started
- Outcome: malformed `.ptx` or autosave files fail clearly instead of risking invalid in-memory state
- Includes:
  - validation for empty or structurally invalid document payloads
  - clearer user-facing failure messaging
  - recovery-file corruption tests
  - stress cases such as interrupted autosave and stale recovery cleanup
- Depends on: none
- Done when:
  - malformed files cannot produce invalid live document state silently

### PROD05 - Tighten Linux windowing and shell behavior for normal desktop use

- [ ] Status: not started
- Outcome: startup, focus, close, move, and resize behavior feel native enough for daily use
- Includes:
  - review of custom undecorated window behavior versus native decorations
  - fix any missing move, resize, and close affordances
  - startup focus and dialog handoff checks
  - shell validation on Wayland and common high-DPI setups
- Depends on: none
- Done when:
  - the shell behaves predictably as a normal Linux desktop application without requiring developer workarounds

### PROD06 - Make PSD import truthful and bounded in user-facing workflows

- [ ] Status: not started
- Outcome: PSD import behaves predictably even when the sidecar is missing, slow, or returns unsupported structure
- Includes:
  - clearer shell messaging when the sidecar is not configured
  - timeout or cancellation strategy for the sidecar path
  - user-visible diagnostics review for fallback imports
  - regression coverage for sidecar failure modes
- Depends on: none
- Done when:
  - PSD import either succeeds within the documented subset or fails clearly without hanging or misleading the user

### PROD07 - Clean up strict lint blockers and runtime hardening warnings

- [ ] Status: not started
- Outcome: the application is clean under the intended strict lint policy and the remaining riskier runtime assumptions are deliberate
- Includes:
  - fix current clippy failures
  - review runtime `expect` and panic paths outside test-only code
  - narrow or document any justified exceptions
- Depends on: none
- Done when:
  - strict linting supports the real production-hardening workflow instead of immediately failing on known issues

### PROD08 - Add better user-facing error and busy-state surfaces

- [ ] Status: not started
- Outcome: failures and long operations are understandable without reading logs
- Includes:
  - clear shell messaging for open, import, save, export, recovery, and filter failures
  - consistent busy-state communication
  - review of which runtime failures should stay internal logs versus user-visible notices
- Depends on: PROD01, PROD04
- Done when:
  - normal users can understand what failed and what to do next from the app itself

### PROD09 - Establish a measured performance and latency baseline

- [ ] Status: not started
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

### PROD10 - Optimize hot paths for fluidity under real workloads

- [ ] Status: not started
- Outcome: brush strokes, viewport interaction, flattening, and file operations stay smooth under representative load
- Includes:
  - profiling and reducing avoidable work in brush interpolation, tile invalidation, and canvas refresh
  - profiling and reducing avoidable work in pan, zoom, and overlay redraw paths
  - profiling and reducing avoidable work in flatten/export paths used during normal workflows
  - stress validation for longer editing sessions and larger layered documents
- Depends on: PROD09
- Done when:
  - the common editing paths feel consistently fluid on the target Linux environment rather than merely passing correctness tests

### PROD11 - Add startup splash screen and renderer warm-up path

- [ ] Status: not started
- Outcome: startup feels intentional and the first real canvas interaction pays less one-time initialization cost
- Includes:
  - a borderless splash window using the official logo
  - progress feedback for startup phases
  - warm-up/preload path for renderer initialization, shader or pipeline compilation, and any other safe startup caches
  - clean handoff from splash screen to the main shell without focus glitches
  - startup timing validation for cold and warm launches
- Depends on: PROD03, PROD09
- Done when:
  - the app can show a polished startup splash while safely preloading startup-critical rendering work and avoiding a jarring first-use hitch

### PROD12 - Keep large modules from becoming release-risk bottlenecks

- [ ] Status: not started
- Outcome: the most critical controller, shell, and persistence code is easier to reason about and less fragile during hardening
- Includes:
  - split especially large modules along real ownership boundaries
  - isolate startup, file-workflow, and performance-critical code paths where helpful
  - keep test coverage intact during refactors
- Depends on: none
- Done when:
  - future hardening work no longer has to land inside a few oversized catch-all files

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

## Notes

- Release and distribution work stays separate on purpose; this checklist is about making the app itself trustworthy and polished.
- The splash screen should be used to hide real startup work, not to add artificial delay.
- If a performance optimization conflicts with correctness or data safety, correctness still wins first.
