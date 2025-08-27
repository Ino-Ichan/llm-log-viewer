<div align="center">

⚠️ <strong>Important Notice</strong>

<strong>This repository was fully “vibe coded” with assistance from Codex and GPT-5.</strong>

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

---

## Quick Start (Build & Run from Source)

**Prerequisites**
- Rust (stable). If you hit toolchain errors, install/update via `rustup`.
- OpenGL-capable environment (macOS/Windows; Linux with X11/Wayland).

```bash
# Build
cargo build --release

# Run
target/release/llm_log_viewer   # Windows: target\release\llm_log_viewer.exe
````

> Tip: Dev run with hot rebuilds (faster feedback):
>
> ```bash
> cargo run --release
> ```

---

## Platform Packaging (clickable apps without a terminal)

### macOS – make a `.app` bundle (no Terminal window)

**1) Ensure bundle metadata (already present in Cargo.toml)**

```toml
[package.metadata.bundle]
name = "LLM Log Viewer"
identifier = "com.example.llm-log-viewer"
icon = ["assets/icon.icns"]
category = "public.app-category.utilities"
short_description = "LLM Log Viewer for macOS"
```

**2) Install cargo-bundle and build the app bundle**

If your Rust is older than the required version, use a per-command toolchain prefix.

```bash
# Install a compatible toolchain once (keeps your global toolchain untouched)
rustup toolchain install 1.86.0

# Install cargo-bundle with that toolchain
cargo +1.86.0 install cargo-bundle --locked

# Produce .app
cargo +1.86.0 bundle --release

# Open it
open "target/release/bundle/osx/LLM Log Viewer.app"
```

> Gatekeeper: Unsigned apps may be blocked on first run. Use **Right click → Open** once to trust it.
> Notarization/codesign steps are in **Optional Signing** below.

**Make an .icns from a PNG (once)**

```bash
mkdir -p app.iconset
sips -z 16 16     icon.png --out app.iconset/icon_16x16.png
sips -z 32 32     icon.png --out app.iconset/icon_16x16@2x.png
sips -z 32 32     icon.png --out app.iconset/icon_32x32.png
sips -z 64 64     icon.png --out app.iconset/icon_32x32@2x.png
sips -z 128 128   icon.png --out app.iconset/icon_128x128.png
sips -z 256 256   icon.png --out app.iconset/icon_128x128@2x.png
sips -z 256 256   icon.png --out app.iconset/icon_256x256.png
sips -z 512 512   icon.png --out app.iconset/icon_256x256@2x.png
cp icon.png app.iconset/icon_512x512@2x.png  # 1024x1024 if available
iconutil -c icns app.iconset -o assets/icon.icns
```

---

### Linux (Ubuntu) – desktop launcher & .deb package

**A) Quick local launcher (.desktop)**

```bash
cargo build --release

mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/llm_log_viewer.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=LLM Log Viewer
Comment=Viewer for LLM logs
Exec=/ABS/PATH/TO/REPO/target/release/llm_log_viewer
Icon=/ABS/PATH/TO/REPO/assets/icon.png
Terminal=false
Categories=Development;Utility;
EOF
chmod +x ~/.local/share/applications/llm_log_viewer.desktop
```

> Some desktops require marking the file as “trusted” in file properties on first run.

**B) Distribution-ready `.deb` (Debian/Ubuntu)**

1. Add minimal packaging metadata to `Cargo.toml`:

   ```toml
   [package.metadata.deb]
   maintainer = "Your Name <you@example.com>"
   assets = [
     ["target/release/llm_log_viewer", "usr/bin/llm_log_viewer", "755"],
     ["assets/icon.png", "usr/share/icons/hicolor/256x256/apps/llm_log_viewer.png", "644"],
     ["packaging/linux/llm_log_viewer.desktop", "usr/share/applications/llm_log_viewer.desktop", "644"],
   ]
   ```
2. Create `packaging/linux/llm_log_viewer.desktop`:

   ```ini
   [Desktop Entry]
   Type=Application
   Name=LLM Log Viewer
   Comment=Viewer for LLM logs
   Exec=llm_log_viewer
   Icon=llm_log_viewer
   Terminal=false
   Categories=Development;Utility;
   ```
3. Build the package:

   ```bash
   cargo install cargo-deb
   cargo build --release
   cargo deb
   # => target/debian/llm-log-viewer_*.deb
   ```

---

### Windows – no console window & optional installer

**Hide the console window (GUI subsystem)**
Add this at the top of `src/main.rs`:

```rust
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
```

**Embed a .ico into the EXE (optional)**
`Cargo.toml`:

```toml
[build-dependencies]
winres = "0.1"
```

`build.rs` (project root):

```rust
fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico"); // include 16..256px sizes
        res.compile().unwrap();
    }
}
```

**Build**

```powershell
cargo build --release
# target\release\llm_log_viewer.exe  (double-click launches without a console window)
```

**MSI installer (optional)**

```powershell
cargo install cargo-wix
cargo wix init   # first time only (generates wix.xml)
cargo wix        # target\wix\*.msi
```

---

## Usage

* Open: Drag & drop a `.json` or `.jsonl` file onto the window, or click **Open file…**.
* Clear: Reset the view with **Clear**.
* Theme: Toggle **Theme: Light/Dark**.
* Copy: **Copy as Markdown** copies the entire conversation (including System). Each message also has a contextual **Copy**.
* Status line: Shows file name, message count, and warnings.

---

## Input Formats

* Baseline schema: `{ "role": "system" | "user" | "assistant", "content": "string" }`
* Unknown roles: Rendered with a neutral bubble and a role badge; extra fields are ignored.

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

---

## Samples

* `samples/sample.json`
* `samples/sample.jsonl`

---

## Release Builds (CI)

* Tagging (e.g. `v0.1.0`) triggers GitHub Actions to build binaries for macOS/Windows/Linux and attach ZIPs.
* Artifacts (example names):

  * macOS: `llm_log_viewer-macOS-x86_64.zip`
  * Windows: `llm_log_viewer-Windows-x86_64.zip`
  * Linux: `llm_log_viewer-Linux-x86_64.zip`
* You can also run the workflow manually and download artifacts from the run page.

> If you want CI to produce a **macOS .app** or **Linux .deb**/**Windows .msi**, integrate the above packaging steps (`cargo bundle`, `cargo deb`, `cargo wix`) into your workflow.

---

## Optional Signing

* **macOS**: codesign/notarize using your Apple Developer ID.

  ```bash
  codesign --deep --force --sign "Developer ID Application: YOUR NAME (TEAMID)" "LLM Log Viewer.app"
  xcrun notarytool submit "LLM Log Viewer.app" --apple-id you@example.com --team-id TEAMID --keychain-profile "notary-profile" --wait
  ```

  After notarization, optionally wrap into a DMG (e.g., `create-dmg`).
* **Windows**: sign with `signtool` and a code-signing certificate.

---

## Troubleshooting

* **Rust too old / `cargo-bundle` fails**
  Install a newer toolchain and prefix commands:

  ```bash
  rustup toolchain install 1.86.0
  cargo +1.86.0 install cargo-bundle --locked
  cargo +1.86.0 bundle --release
  ```
* **Blank/crashed window**
  Ensure OpenGL/graphics drivers are available; try updating drivers or switching GPUs.
* **Gatekeeper blocks app (macOS)**
  Right click → **Open** once to trust.
* **Linux `.desktop` not launching**
  Ensure absolute paths in `Exec`/`Icon`, mark file as executable, and mark as trusted in file properties if required.
* **Very large logs**
  Files > \~20MB may be slower; the app warns but still attempts to render.

---

## License

MIT — see `LICENSE` for details.
