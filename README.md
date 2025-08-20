<div align="center">

⚠️ <strong>Important Notice</strong>

<strong>This repository was fully “vibe coded” with assistance from Codex and GPT‑5.</strong>

<strong>Expect experimental/rough edges — review carefully and use at your own discretion.</strong>

</div>

---

# LLM Log Viewer

Fast, minimal, single-binary desktop app to view ChatGPT-like conversations from JSON/JSONL logs. System messages are always visible, and the UI focuses on quick inspection and copying content.

Why this exists: Debugging LLM runs is painful when logs are raw JSON. This app renders them as a clean conversation so you can skim, verify, and copy quickly.

## Features

- Single binary: No installers or services; just run the executable.
- Drag & drop: Drop `.json` or `.jsonl` to render immediately.
- Auto detect: Switches between JSON array and JSONL automatically.
- Markdown rendering: Renders message content with code blocks preserved and scrollable.
- System card: System prompt is pinned at the top in a distinct card.
- Per-message copy: Copy any message in Markdown; copy whole chat via the toolbar.
- Theming & text size: Light/Dark toggle and adjustable text scale.
- Resilient parsing: Shows warnings, continues on partial JSONL errors, and handles large files with care.

## Install

- Prebuilt binaries: Download from GitHub Releases (see “Release Builds”).
- Build from source:
  - Prerequisites: Rust (stable) via `rustup`. OpenGL-capable system (macOS/Windows, most Linux desktops with X11/Wayland).
  - Build: `cargo build --release`
  - Run: `target/release/llm_log_viewer`

## Usage

- Open: Drag & drop a `.json` or `.jsonl` file onto the window, or click `Open file…`.
- Clear: Reset the view with `Clear`.
- Theme: Toggle `Theme: Light/Dark`.
- Copy: `Copy as Markdown` copies the entire conversation (including System). Each message also has a contextual Copy.
- Status line: Shows file name, message count, and warnings.

## Input Formats

- Baseline schema: `{ "role": "system" | "user" | "assistant", "content": "string" }`
- Unknown roles: Rendered with a neutral bubble and a role badge; extra fields are ignored.

### Example JSON (array)

```json
[
  {"role": "system", "content": "You are a world-class math agent."},
  {"role": "user", "content": "Please explain what is 1 + 1?"},
  {"role": "assistant", "content": "Of course! The answer is 2. Let's break down why. ..."},
  {"role": "user", "content": "Next question is ..."},
  {"role": "assistant", "content": "Amazing! ..."}
]
```

### Example JSONL

```json
{"role": "system", "content": "You are a world-class math agent."}
{"role": "user", "content": "Please explain what is 1 + 1?"}
{"role": "assistant", "content": "Of course! The answer is 2. Let's break down why. ..."}
{"role": "user", "content": "Next question is ..."}
{"role": "assistant", "content": "Amazing! ..."}
```

## Samples

- `samples/sample.json`
- `samples/sample.jsonl`

## Build and Run

- Dev run: `cargo run --release`
- Release binary: `cargo build --release` → `target/release/llm_log_viewer` (Windows: `.exe`)

## Release Builds

- CI releases: Pushing a tag like `v0.1.0` triggers GitHub Actions to build macOS/Windows/Linux binaries and attach ZIPs to the Release.
- Artifacts:
  - macOS: `llm_log_viewer-macOS-x86_64.zip`
  - Windows: `llm_log_viewer-Windows-x86_64.zip`
  - Linux: `llm_log_viewer-Linux-x86_64.zip`
- Manual run: You can run the workflow manually to get build artifacts (they appear under the workflow run’s “Artifacts”).

### Platform Notes

- macOS: If not codesigned, right-click → Open on first run due to Gatekeeper. Optional: bundle/codesign/notarize if distributing broadly.
- Windows: Unsigned builds may warn on SmartScreen; optional code signing recommended for distribution.
- Linux: Requires an OpenGL-capable environment on X11 or Wayland. If shipping to very old distros, consider building on an older glibc or providing a containerized format.

### Optional Signing

- macOS: codesign/notarize using your Developer ID; see steps below.
- Windows: sign with `signtool` and a code signing certificate.

### macOS Codesigning/Notarization (optional)

1. Create an app bundle if you prefer a `.app` (optional). Minimal approach:
   - Make a bundle folder: `LLM Log Viewer.app/Contents/MacOS/llm_log_viewer` (copy the binary there)
   - Add `LLM Log Viewer.app/Contents/Info.plist` (CFBundleName, Identifier, Version)
2. Codesign: `codesign --deep --force --sign "Developer ID Application: Your Name (TEAMID)" LLM\ Log\ Viewer.app`
3. Notarize with `xcrun notarytool submit` (Apple developer account required).
4. Optionally wrap into a DMG (e.g., `create-dmg`).

## Troubleshooting

- Blank or crashed window: Ensure GPU drivers/OpenGL are available. Try updating graphics drivers or running on a different GPU.
- Non‑UTF8 files: The app warns and skips invalid encoding; convert to UTF‑8 if needed.
- Very large logs: Files > ~20MB may be slower; the app surfaces a warning and still attempts to render.
- JSONL errors: Invalid lines are skipped; a warning displays the count of failed lines.


## License

- MIT License — see `LICENSE` for details.
