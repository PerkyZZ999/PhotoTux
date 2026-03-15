# Release And Distribution Tasks

## Purpose

This task list turns the release/distribution plan into an implementation sequence for making PhotoTux easier to validate, package, and ship.

## Principles

- Linux-first remains the primary delivery target
- reproducibility and validation come before packaging breadth
- do one packaging path well before adding many

## Task List

### REL01 - Define release-quality validation policy

- [ ] Status: not started
- Outcome: release automation aligns with actual project risk instead of ad hoc checks
- Includes:
  - required checks for merge and release
  - release-blocking bug categories
  - warning/failure policy for CI
- Depends on: none
- Done when:
  - the project has a clear release validation standard

### REL02 - Add CI for formatting, linting, testing, and release build

- [ ] Status: not started
- Outcome: the core validation path runs automatically on clean environments
- Includes:
  - `cargo fmt --check`
  - clippy policy
  - `cargo test`
  - release build validation
- Depends on: REL01
- Done when:
  - CI reliably validates normal changes and release candidates

### REL03 - Review release profile and build settings

- [ ] Status: not started
- Outcome: release builds are intentionally tuned rather than default-only artifacts
- Includes:
  - release profile review
  - symbol/size tradeoff decisions
  - startup and runtime sanity checks
- Depends on: none
- Done when:
  - release build behavior is documented and justified

### REL04 - Document runtime prerequisites for Linux targets

- [ ] Status: not started
- Outcome: build and launch expectations are explicit for testers and users
- Includes:
  - GTK and GPU/runtime notes
  - Wayland/X11 expectations where relevant
  - install prerequisites for supported environments
- Depends on: none
- Done when:
  - testers know what environment assumptions PhotoTux currently makes

### REL05 - Choose the first supported packaging target

- [ ] Status: not started
- Outcome: packaging work has a clear first target rather than fragmenting
- Includes:
  - compare Arch packaging, AppImage, and Flatpak-first approaches
  - decide primary first packaging route
- Depends on: REL02, REL04
- Done when:
  - one packaging path is selected and documented

### REL06 - Implement the first packaging path

- [ ] Status: not started
- Outcome: non-developer installation becomes realistic
- Includes:
  - package metadata and assets
  - build instructions or automation
  - launch validation from packaged output
- Depends on: REL05
- Done when:
  - a supported package can be built and installed successfully

### REL07 - Add packaged-build smoke validation

- [ ] Status: not started
- Outcome: packaged output is verified as an application, not just an archive
- Includes:
  - packaged launch check
  - open/save/export smoke flow
  - crash-recovery smoke flow where practical
- Depends on: REL06
- Done when:
  - the package path has a repeatable smoke-test routine

### REL08 - Define versioning and release checklist

- [ ] Status: not started
- Outcome: shipping becomes a disciplined workflow
- Includes:
  - versioning scheme
  - release checklist
  - pre-release and post-release tasks
- Depends on: REL02, REL06
- Done when:
  - a release can be prepared from a written checklist instead of tribal knowledge

### REL09 - Add user-facing installation and release notes docs

- [ ] Status: not started
- Outcome: users and testers can install and evaluate builds without guessing
- Includes:
  - install instructions
  - supported-environment notes
  - release notes template
- Depends on: REL06, REL08
- Done when:
  - release artifacts are accompanied by enough guidance to be usable

### REL10 - Evaluate whether a second packaging path is justified

- [ ] Status: not started
- Outcome: packaging breadth expands only if the first path is stable
- Includes:
  - assess feedback from the first packaging route
  - decide whether to add AppImage, Flatpak, or distro-specific packaging next
- Depends on: REL07, REL09
- Done when:
  - the next distribution investment is chosen intentionally

## Suggested Execution Order

1. REL01
2. REL02
3. REL03
4. REL04
5. REL05
6. REL06
7. REL07
8. REL08
9. REL09
10. REL10

## Notes

- If CI stability is weak, fix that before spending time on packaging breadth.
- Avoid cross-platform packaging expansion until Linux distribution is repeatable and trusted.