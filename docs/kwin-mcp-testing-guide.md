# KWin MCP Testing Guide

## Purpose

This guide explains how to install, configure, validate, and adopt `kwin-mcp` and `kwin-mcp-cli` for PhotoTux so they can be used later for desktop automation and end-to-end testing.

The goal is to make PhotoTux testable as a Linux desktop application in a way that is closer to Playwright-style automation, but adapted for native GTK and Wayland workflows.

## Why This Stack

`kwin-mcp` is currently the strongest candidate for PhotoTux because it provides:

- isolated virtual KWin Wayland sessions
- AT-SPI2 accessibility-tree inspection
- screenshot capture
- mouse, keyboard, and touch automation
- window management and element polling
- a matching CLI surface via `kwin-mcp-cli`

This fits the project well because PhotoTux is Linux-first, Wayland-sensitive, GTK-based, and already documents KDE Plasma as an important target environment.

## Scope Of This Guide

This guide covers:

- host prerequisites
- `kwin-mcp` installation
- `kwin-mcp-cli` verification
- VS Code MCP configuration for this workspace
- a recommended staged adoption plan for PhotoTux
- limitations and operational caveats

This guide does not yet implement automated test cases inside the repository. It prepares the environment so those tests can be added later with much less friction.

Related document:

- `kwin-mcp-test-checklist.md`

## Preconditions

Before you start, confirm that the machine you want to use for GUI automation satisfies these conditions:

1. Linux system running KDE Plasma 6 on Wayland.
2. Python 3.12 or newer is available.
3. `kwin_wayland` supports the `--virtual` mode used by `kwin-mcp`.
4. AT-SPI2 is available.
5. D-Bus support is available.
6. You are comfortable running a local MCP server that can launch apps and inject input.

Recommended optional tools:

- `wl-clipboard`
- `wtype`
- `wayland-info`

## Phase 1: Install Host Dependencies

Use the upstream `kwin-mcp` README as the source of truth for distro-specific packages. The exact package names vary by distribution.

At a minimum, the host should provide these capabilities:

- KDE Plasma 6 Wayland session components
- `kwin_wayland`
- `spectacle`
- `at-spi2-core`
- PyGObject support
- D-Bus Python bindings

Recommended validation commands:

```bash
python3 --version
kwin_wayland --version
command -v spectacle
command -v dbus-run-session
command -v at-spi-bus-launcher
```

If any of those are missing, install the required system packages first and stop here until they are available.

## Phase 2: Install `uv`

The recommended installation path for `kwin-mcp` is `uv`.

If `uv` is not already installed, install it using the method you trust for your Linux environment.

Validation command:

```bash
uv --version
```

## Phase 3: Install `kwin-mcp`

Install the package globally for your user with `uv`:

```bash
uv tool install kwin-mcp
```

This is the preferred route because it keeps the server and CLI easy to update and avoids per-project Python environment drift.

Validation commands:

```bash
kwin-mcp --help
kwin-mcp-cli --help
```

Expected result:

- both commands resolve successfully
- the CLI is available on `PATH`

If `kwin-mcp-cli` is not found but `kwin-mcp` is installed, check the upstream release notes and installed scripts to confirm the exact entry-point name exposed by your version.

## Phase 4: Validate The CLI Before MCP Integration

Before wiring the server into VS Code, verify that the underlying automation engine works on this machine.

Recommended first check:

1. Start the CLI.
2. Start an isolated session.
3. Launch a trivial GUI app such as `kcalc`.
4. Capture a screenshot.
5. Inspect the accessibility tree.
6. Stop the session.

The exact commands may evolve by release, so use the built-in help from `kwin-mcp-cli` on your installed version.

Success criteria for this phase:

- the virtual session starts successfully
- the sample app launches inside the session
- screenshots work
- the accessibility tree returns structured output

If this phase fails, do not proceed to VS Code integration yet. Fix the host environment first.

## Phase 5: Configure VS Code MCP For This Workspace

Because this environment uses VS Code with Copilot chat tooling, configure the MCP server in the workspace-local file:

- `.vscode/mcp.json`

This is the recommended workspace configuration:

```json
{
  "servers": {
    "kwin-mcp": {
      "type": "stdio",
      "command": "uvx",
      "args": ["kwin-mcp"]
    }
  }
}
```

Why this shape:

- workspace-local configuration makes the server available specifically in PhotoTux
- `uvx kwin-mcp` avoids baking an absolute path into the config
- it matches the stdio MCP model expected by VS Code

Alternative configuration if you prefer the globally installed script directly:

```json
{
  "servers": {
    "kwin-mcp": {
      "type": "stdio",
      "command": "kwin-mcp"
    }
  }
}
```

Use the `uvx` variant unless you have a reason to pin the installed binary explicitly.

## Phase 6: Start And Trust The Server In VS Code

Once `.vscode/mcp.json` exists:

1. Open the Command Palette.
2. Run `MCP: List Servers`.
3. Confirm that `kwin-mcp` appears.
4. Start the server if it is not already running.
5. Accept the trust prompt after reviewing the configuration.

Useful VS Code commands during setup:

- `MCP: List Servers`
- `MCP: Open Workspace Folder Configuration`
- `MCP: Reset Trust`

If the server fails to start, inspect its output through the MCP server management flow in VS Code before changing project files.

## Phase 7: Verify MCP Access From Chat

After the server is running in VS Code, run a minimal chat-driven verification before attempting PhotoTux automation.

Use a simple prompt against a trivial GUI app first, such as:

```text
Start an isolated KWin session, launch kcalc, find the calculator buttons through the accessibility tree, take a screenshot, and stop the session.
```

Success criteria:

- the server tools are visible to the agent
- a session can be started and stopped
- an app can be launched
- an accessibility-tree read works
- a screenshot is captured

If this succeeds, the workspace is ready for later PhotoTux-specific automation work.

## Phase 8: Adopt It In The PhotoTux Repository

Do this only after the host and VS Code integration are proven.

Recommended repository additions:

### 1. Add Workspace MCP Configuration

Commit `.vscode/mcp.json` once the configuration is stable and the team agrees to share it.

Do not commit it earlier if the machine-specific environment is still unstable.

### 2. Add A GUI Testing Area

Create a dedicated directory for future desktop automation artifacts and scripts, for example:

```text
tests/gui/
tests/gui/README.md
tests/gui/scenarios/
tests/gui/artifacts/
```

Recommended purpose:

- `README.md`: how GUI tests are expected to run
- `scenarios/`: human-readable scenario definitions or scripts
- `artifacts/`: screenshots and captured outputs kept out of version control unless intentionally checked in

### 3. Add A Small First Scenario Set

Start with a tiny high-value smoke suite:

1. launch PhotoTux
2. verify main shell regions appear
3. create a document if that workflow exists
4. switch tools
5. perform basic zoom or pan
6. save or export through the shell once those flows are complete

Do not start with brush-stroke fidelity or complex canvas assertions. Establish shell stability first.

### 4. Add Documentation For Human Operators

Once the setup is confirmed, add references from:

- `docs/testing-strategy.md`
- any future GUI testing README
- post-MVP shell or release planning docs if those tracks adopt GUI automation formally

## Recommended Adoption Order For PhotoTux

Use this sequence:

1. confirm host prerequisites
2. install `uv`
3. install `kwin-mcp`
4. validate `kwin-mcp-cli`
5. add `.vscode/mcp.json`
6. validate `kwin-mcp` from VS Code chat
7. create `tests/gui/` scaffolding
8. add one shell-level smoke scenario
9. expand to save/open/export and recovery flows
10. expand to canvas interactions only after shell and accessibility stability is proven

## What To Test First In PhotoTux

Good first candidates:

- app launch
- window focus and single main window detection
- presence of tool rail, panels, status bar, and canvas host
- menu or toolbar command reachability
- open/save/export dialog flows once implemented
- recovery prompt behavior

Defer until later:

- detailed brush rendering assertions
- transform fidelity assertions
- pixel-precise canvas correctness

Those are still better handled by the existing headless Rust tests until the GUI automation path is trusted.

## Limitations You Should Expect

Important constraints from `kwin-mcp` itself:

- KDE Plasma 6 Wayland is required
- AT-SPI coverage varies by application and widget
- AT-SPI coordinates are window-relative rather than globally reliable in every multi-window scenario
- some popup and native menu flows may not appear fully in AT-SPI
- `keyboard_type` assumes US QWERTY for plain typing

PhotoTux-specific implication:

- shell chrome and many GTK widgets are likely better automation targets than the canvas itself in the first pass

## Security And Trust Notes

This server can:

- launch applications
- read accessibility state
- capture screenshots
- inject mouse and keyboard input

Only enable it on a machine and in a workspace you trust.

Prefer workspace-local configuration over global configuration until you are sure you want it available everywhere.

## Troubleshooting Checklist

If `kwin-mcp` does not work, check these in order:

1. Are you on KDE Plasma 6 Wayland?
2. Does `kwin_wayland --virtual` exist on this machine?
3. Does `kwin-mcp-cli` work before MCP configuration is introduced?
4. Can the sample app be launched in an isolated session?
5. Does `MCP: List Servers` show the server in VS Code?
6. Did VS Code trust get accepted for the workspace server?
7. Do the MCP output logs show missing runtime dependencies?

If `kwin-mcp-cli` works but VS Code integration fails, the problem is likely the MCP configuration.

If neither works, the problem is likely the host environment.

## Definition Of Done For Setup

This setup is complete when all of the following are true:

1. `kwin-mcp` is installed and on the machine.
2. `kwin-mcp-cli` can start an isolated session and interact with a trivial GUI app.
3. `.vscode/mcp.json` is configured for this workspace.
4. VS Code can start and trust the `kwin-mcp` server.
5. A chat-driven smoke interaction succeeds in the PhotoTux workspace.

Once those are true, the project is ready for later GUI automation work.

## Upstream References

- Repository: `https://github.com/isac322/kwin-mcp`
- PyPI: `https://pypi.org/project/kwin-mcp/`
- VS Code MCP setup docs: `https://code.visualstudio.com/docs/copilot/chat/mcp-servers`