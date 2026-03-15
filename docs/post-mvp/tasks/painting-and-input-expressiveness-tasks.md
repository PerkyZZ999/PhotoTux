# Painting And Input Expressiveness Tasks

## Purpose

This task list turns the painting/input plan into an implementation sequence for improving illustration-oriented workflows.

## Principles

- preserve the current low-latency paint path
- do not let richer input regress mouse workflows
- keep destructive operations off the UI thread

## Task List

### PAINT01 - Audit current brush/input extension points

- [ ] Status: not started
- Outcome: richer input work starts from measured extension points rather than assumptions
- Includes:
  - identify current brush parameter boundaries
  - identify where stylus pressure can enter safely
  - document current latency-sensitive hotspots
- Depends on: none
- Done when:
  - the brush/input expansion path is technically mapped

### PAINT02 - Add pressure data plumbing through the interaction path

- [ ] Status: not started
- Outcome: stylus data can reach brush evaluation cleanly
- Includes:
  - shell input plumbing for pressure where supported
  - controller/tool-system propagation of pressure samples
  - mouse fallback behavior
- Depends on: PAINT01
- Done when:
  - brush evaluation can receive pressure information without changing document ownership boundaries

### PAINT03 - Add initial pressure mapping controls

- [ ] Status: not started
- Outcome: pressure support becomes useful instead of merely available
- Includes:
  - pressure-to-size mapping
  - pressure-to-opacity or flow mapping
  - toggle or preset support
- Depends on: PAINT02
- Done when:
  - pressure-sensitive devices materially affect brush behavior in a controlled way

### PAINT04 - Improve brush dynamics and parameter range

- [ ] Status: not started
- Outcome: brush behavior becomes more expressive for both mouse and stylus use
- Includes:
  - spacing improvements
  - flow behavior review
  - hardness behavior refinement
  - parameter validation and regression tests
- Depends on: PAINT01
- Done when:
  - brush behavior has broader control without becoming unstable

### PAINT05 - Add brush preset structure if needed

- [ ] Status: not started
- Outcome: richer brush behavior stays usable in practice
- Includes:
  - preset model
  - preset switching hooks in shell/controller
  - persistence rules if presets become user data
- Depends on: PAINT03, PAINT04
- Done when:
  - users can return to useful brush setups quickly

### PAINT06 - Strengthen direct-manipulation preview for richer brushes

- [ ] Status: not started
- Outcome: preview quality keeps pace with more expressive brush behavior
- Includes:
  - preview-path refinement
  - brush cursor/preview improvements where appropriate
  - latency regression checks
- Depends on: PAINT03, PAINT04
- Done when:
  - richer brushes still feel immediate during normal use

### PAINT07 - Add regression and profiling coverage for paint-heavy paths

- [ ] Status: not started
- Outcome: richer painting features do not silently degrade responsiveness
- Includes:
  - repeated stroke stress checks
  - medium-canvas painting validation
  - pressure-enabled manual validation notes
- Depends on: PAINT06
- Done when:
  - the richer brush path has explicit performance guardrails

### PAINT08 - Define destructive filter command model

- [ ] Status: not started
- Outcome: later filter work can plug into the existing architecture cleanly
- Includes:
  - filter command routing
  - history integration rules
  - worker/job execution rules
- Depends on: none
- Done when:
  - destructive raster operations have a clean architectural slot

### PAINT09 - Implement a minimal destructive filter set

- [ ] Status: not started
- Outcome: the first non-paint raster operations become available safely
- Includes:
  - choose a narrow first filter set
  - CPU-side implementation in `image_ops` or appropriate boundary
  - undo/redo and save/load validation
- Depends on: PAINT08
- Done when:
  - at least one destructive filter workflow is end-to-end and trustworthy

### PAINT10 - Validate stylus and paint ergonomics on target Linux systems

- [ ] Status: not started
- Outcome: the expressive-painting track is grounded in real device behavior
- Includes:
  - stylus manual validation
  - mouse regression validation
  - latency and responsiveness notes
- Depends on: PAINT03, PAINT06, PAINT09
- Done when:
  - painting improvements are validated beyond unit correctness

## Suggested Execution Order

1. PAINT01
2. PAINT02
3. PAINT03
4. PAINT04
5. PAINT06
6. PAINT07
7. PAINT05
8. PAINT08
9. PAINT09
10. PAINT10

## Notes

- If pressure input is inconsistent across devices, ship a narrow pressure subset before broad brush dynamics.
- If richer brush behavior hurts latency, prioritize path optimization over adding more brush parameters.