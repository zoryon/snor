# Snor development notes

- Read `README.md` and the relevant document in `docs/` before changing architecture.
- The product is a Tauri desktop app, not a website or a Node server.
- Keep the runtime lean: Rust talks directly to the official Claude Code executable.
- Preserve the distinction between real desktop execution and browser UI preview. Never fabricate model output, quota data or connection state.
- Never bypass tool approvals to make an integration test pass.
- Keep user text out of shell command strings; use process argument arrays and JSON pipes.
- Update `docs/ROADMAP.md` and `docs/VERIFICATION.md` when implementation or verification status changes.
- Validate meaningful changes with frontend checks/tests/build and relevant Rust tests. AI integration needs a separate real CLI verification.
- On Windows use `npm.cmd`. Do not change global PowerShell policy.
- Performance claims require release measurements including WebView2, Claude Code and tool processes. Frontend marks alone do not measure native startup.
- Keep instructions, examples and UI in Italian unless a technical protocol requires English.
