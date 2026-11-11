---
name: argand-ui-acceptance
description: Plan and verify Argand UI changes in the running production application, recording appearance, interaction and acceptance limits. Use for UI implementation or acceptance, not prose-only edits or headless checks alone.
---

# Argand UI acceptance

This repository-local workflow implements the canonical
[UI acceptance policy](../../../CONTRIBUTING.md#ui-acceptance); it does not replace
that policy or authorize additional actions. The coordinator includes acceptance
in the task plan, the executor runs this workflow without requiring the owner to
name it, and the reviewer assesses the resulting evidence.

## Establish the scenario

Read the agreed criteria and relevant [UI architecture](../../../docs/architecture/ui.md).
Identify the affected fields, command entry points, shared consumers and an existing
panel/control or approved reference for comparison. Specify concrete expected
appearance and behavior before checking it. Choose relevant cases, not an automatic
matrix of every control or platform.

For example, a numeric-field change shared by two panels needs a scenario for each:
focus the field, edit and commit it, step with keys/pointer, open and dismiss its
Select if present, and compare the neighboring panel's focus and formatting.
Use the expected transaction for each panel; shared appearance does not imply that
preview, commit or Escape semantics are identical.

## Prepare a bounded run

Use the current production binary after the checks/build required by
[implementation validation](../../../CONTRIBUTING.md#implementation-and-validation).
Record the tested revision and local-diff identity in the acceptance report, together
with the build command/result and binary path; do not put commit hashes in plans.
Do not substitute a test-support build or an old installed binary.

Inspect current config/session lookup and supported launch options before choosing
isolation. Use disposable test data and an isolated supported configuration/session
setup, verifying that the run cannot overwrite the owner's settings or working
files. Do not invent environment variables or assume a config flag exists. Preserve
the owner's running application; if safe isolation cannot be established, report
that limitation rather than changing the working setup. Keep host-specific setup
outside shared repository instructions.

### Isolate the display with Xvfb

Run all UI acceptance launches and interaction checks of the production application
on an isolated Xvfb virtual X11 display. Never launch on the owner's desktop or
physical monitors, including either the main or a secondary monitor. Do not inherit
the desktop's `DISPLAY` or `WAYLAND_DISPLAY` into the application child: set its
`DISPLAY` to the verified Xvfb server, unset its Wayland selector, and verify the
application's supported backend selection will use X11. Apply these changes only
to task-owned child processes; leave the owner's desktop environment unchanged.

Before launching, verify the Xvfb server's identity and readiness, the child display
and backend selection, and supported isolation of configuration, session and cache.
For example, inheriting `DISPLAY=:0` while isolating only settings is not display
isolation. If Xvfb is missing, the isolated display is unavailable, or the X11
backend cannot run, report the affected checks blocked/incomplete. Do not fall back
to a visible desktop or install packages automatically. On cleanup, stop only
verified task-owned application, helper and Xvfb processes.

Record actual production rendering and interaction in Xvfb separately from GPUI
tests, other headless checks and simulations. An Xvfb run does not establish native
Wayland behavior or other platform-specific acceptance; report those limits.

Discover available, permitted display and input capabilities; neither a tool nor
native access is guaranteed. Permission to view screenshots is not permission to
inject input. Respect denied input/remote-control routes and never bypass a denial
through another mechanism. Use only authorized interaction or explicitly identified
owner-provided observations; otherwise the interaction checks remain unperformed.

## Exercise and compare

Run the application and exercise the actual affected scenario with relevant pointer
and keyboard input. Check transitions as well as the resting picture: hover/tooltips,
click/focus, numeric commit/step, panel switching, Select open/choose/dismiss,
Enter/Escape, disabled/busy states and overlay/window close where affected. Verify
each affected consumer of shared behavior. Include long values/labels, narrow windows
and light/dark themes when they can expose the changed layout or behavior.

Compare appearance and behavior with the agreed reference, recording discrepancies.
A screenshot supports visual comparison; it does not establish that a click, focus
change or dismissal worked. Stop an unsafe action, preserve the test setup and
route any defect under the existing Issue/scope rules. Do not silently change the
expected behavior to make a failing scenario pass.

## Report evidence and limits

For each criterion record expected result, actual observed result, reproduction
steps, evidence and unperformed checks. Identify platform, actual display backend
(or unknown), window/theme conditions and build. Separate direct native observations,
owner-provided observations, automated/headless results and simulations. Never
label a desk check or static capture as an executed application scenario.

Report passed, failed or incomplete criteria, not just a test count. A defect is a
failed criterion; unavailable execution or required interaction is incomplete.
Neither supports calling the UI accepted/ready or moving its PR to Ready under the
canonical policy. Identify the remaining verification needed without promising
unavailable access. Refresh evidence invalidated by later changes. This skill does
not trigger commits, pushes, merges or an additional platform matrix.
