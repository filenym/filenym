# Isolated Windows desktop acceptance

The approved A/B/E matrix and G1–G9 journeys are in [Milestone 05](../../plans/phase-1/05-desktop-ui.md#windows-end-to-end-acceptance-protocol). React/coordinator tests alone do not establish GUI acceptance.

Use only owned fixtures beneath repository `temp/` and a separate application identifier/profile. Never remove or alter the normal `app.filenym.desktop` registry. Keep failed databases and evidence for review.

## Prepare and run

1. Create ignored `temp/m05.config.json` as UTF-8 JSON without a BOM, with a unique identifier such as `{"identifier":"app.filenym.m05-validation-20261016"}`. Build and launch using this override. Verify the actual registry and WebView profile beneath `%LOCALAPPDATA%/<test identifier>` before any fixture operation.
2. Prepare `temp/m05-fixtures/sentinel.txt`, sibling sentinel, empty and Unicode files, and a local directory junction to an owned target. Record original hashes, exact paths, sizes and modification times. Hashing belongs only to the test harness.
3. Record the exact test executable PID. Enable CDP only when needed, in the launching process environment, bound to loopback. Do not add debug flags, unrestricted capabilities or testing commands to product configuration.

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223 --remote-debugging-address=127.0.0.1 --force-renderer-accessibility'
npm.cmd run tauri:dev -- --config temp/m05.config.json
```

Use a separate terminal for these commands, supplying the observed PID and actual repository fixture path:

```powershell
./tests/desktop/smoke.ps1 -TargetProcessId <pid> -Action List
./tests/desktop/journey.ps1 -TargetProcessId <pid> -FixturePath <absolute-repository-temp-path>/m05-fixtures/sentinel.txt
node tests/desktop/controls.mjs
./tests/desktop/smoke.ps1 -TargetProcessId <pid> -Action Close
```

`journey.ps1` requires a fresh **zero-record test registry**. It exercises native picker cancellation/selection, three aliases, case-only/identical rename, delete Cancel/confirm and all duplicate choices. It waits for controls before dispatch; it never repeats a dispatched action after an uncertain failure. It leaves two canonical records, one with `context.PDF` and `Third.txt`.

`controls.mjs` requires that fixture state, the actual FileNym WebView at port 9223, and Node 22's built-in WebSocket/fetch. It selects the sentinel with the most aliases, verifies the saved test-fixture path, rejects invalid/duplicate names, and adds Unicode/boundary aliases if absent. It exercises trusted keyboard input, modal focus, themes and viewport/media emulation. Evidence is written beneath ignored `temp/m05-evidence/`, distinguished as development/production. These are mutating acceptance journeys for isolated fixtures, not commands for a user registry.

`smoke.ps1` scopes UI Automation to windows of the explicit FileNym PID and their WebView descendants. It uses Invoke/ExpandCollapse/Value patterns. When native picker patterns are absent, `native-controls.cs` uses bounded WM_SETTEXT/BM_CLICK after checking PID and control class. Close uses WindowPattern and waits for clean exit. There is no mouse fallback, forced termination, SQL or fixture deletion in these helpers. Reconfirm the PID after every restart.

## Complete acceptance beyond the scripts

The scripts do not replace every G1–G9 assertion. Follow the plan for junction selection; external-writer conflict and Busy; pending-write shutdown; all recorded lifecycle states; missing-database safety; native window minimum/maximize/restore; physical sentinel integrity; Narrator; actual Windows scaling; and rendered contrast. Seed only owned fixture databases using repository APIs or reviewed test-only SQL against the unchanged schema. Never add production fault-injection IPC.

Close development cleanly, stop its Vite session, and verify port 5173 is no longer listening. Build the final production executable with the isolated identifier:

```powershell
npm.cmd run tauri:build -- --no-bundle --config temp/m05.config.json
```

Launch only that verified isolated executable and repeat production acceptance without Vite. Preserve the earlier fixture registry if a separate fresh production first-run fixture is needed; validate every absolute source/destination remains beneath the exact isolated profile before moving it. Do not delete a marker/database or touch production data to force a first run. A missing DB with its marker must remain missing and block writes.

Record test method precisely: UI Automation/native messages, CDP input, emulated media/viewport/font sizes, actual Windows actions, or human report. WebView DPR emulation is not an OS DPI change, font scaling is not native browser zoom, and accessibility-tree assertions are not Narrator speech. Keep E1–E4 provider/device/platform evidence separate. No helper proves physical identity, universal hydration safety or hardware durability.

After acceptance, close only owned windows/processes/listeners, verify fixture hashes/size/mtime and no physical alias files, and retain evidence. Check resolved cleanup targets before any recursive removal; no cleanup deletion is required by these scripts.

## Recorded acceptance, 2026-10-16–17

Development and final production native-picker/CRUD journeys passed. Real SQLite conflict/Busy, responsive event loop, pending-write close/drain, lifecycle metadata preservation, missing-DB refusal, theme restart, browser fallback, contrast and sentinel integrity passed. Window geometry was measured at actual 125% DPI; additional viewport/media/text resizing was explicitly emulated. The user separately confirmed Narrator/keyboard/dialog checks and actual Windows 100/125/150/200% layouts passed, with normal scaling restored. See the plan for detailed evidence and remaining optional limitations.

The earlier Cancel accessibility limitation was resolved by bounded native-control messages, without the previously declined mouse fallback. Initial harness failures are not counted as passes. No hosted GUI CI suite or real cloud/provider/physical-device acceptance is claimed.
