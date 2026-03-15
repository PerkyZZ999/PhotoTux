# Post-MVP Plan: Release And Distribution Hardening

## Purpose

Turn the post-MVP codebase into something easier to test, ship, and trust outside a development environment.

## Why This Is A Natural Next Step

Once the MVP workflow is complete and stabilized, the next trust multiplier is not a single editing feature. It is repeatability:

- reproducible builds
- CI validation
- packaging
- release discipline

Without this track, every future feature still lands in a product that is harder to distribute and harder to validate consistently.

## Goal

Establish a release-quality workflow for Linux-first distribution while preserving the project’s current architecture and task boundaries.

## Scope

### In Scope

- CI checks for fmt, clippy, test, and release build
- release profile review
- packaging strategy for Linux targets
- installation and launch validation
- release checklist and versioning conventions

### Explicitly Out Of Scope

- cross-platform expansion as a primary goal
- Windows and macOS parity work
- auto-update infrastructure unless specifically adopted later

## Recommended Delivery Order

1. CI baseline hardening
2. release build validation
3. package output plan for Linux
4. manual release checklist and smoke validation
5. optional packaging automation expansion

## Work Breakdown

### Phase 1: CI Baseline

Deliverables:

- CI for `cargo fmt --check`
- CI for clippy with agreed warning policy
- CI for `cargo test`
- CI for release build validation

Key design rules:

- keep the validation set close to the actual stabilization requirements already documented
- avoid adding heavyweight infrastructure before the core pipeline is reliable

Exit criteria:

- pull requests and mainline changes receive consistent automated validation

### Phase 2: Release Build Quality

Deliverables:

- release-profile review
- symbol, size, and startup checks where useful
- documented GPU/runtime prerequisites for Linux systems

Key design rules:

- performance and binary-size decisions should be explicit, not incidental

Exit criteria:

- release builds are consistently reproducible and behave like supported artifacts

### Phase 3: Packaging

Deliverables:

- decision on first supported package path
- packaging instructions or automation for that path
- installation validation on target Linux environments

Likely candidates:

- Arch-oriented packaging
- AppImage
- Flatpak later

Key design rules:

- avoid packaging sprawl; choose one path to do well first

Exit criteria:

- a non-developer can install and run a supported package with predictable results

### Phase 4: Release Discipline

Deliverables:

- versioning guidance
- release checklist
- smoke-test checklist for shipped builds
- issue triage labels for release-blocking bugs

Exit criteria:

- shipping becomes a repeatable process rather than a one-off effort

## Main Risks

- packaging work can sprawl into cross-platform ambitions prematurely
- CI can become noisy if warnings and failure policy are not defined tightly
- release work can lag behind architecture if it is attempted before the validation path is stable

## Validation Requirements

- successful CI runs on fresh environments
- release build success in automation
- documented install and launch checks on target Linux systems

## Success Condition

PhotoTux can be validated, built, and shipped predictably enough for broader personal or early external use.