## Goal

Create a **single-binary desktop app in Rust** that renders a ChatGPT-like conversation UI from a dropped **JSON or JSONL** file containing OpenAI-style `messages` with `role` and `content`. **Unlike ChatGPT, the System message must be visible in the GUI.** Minimal, fast, no external services.

## Tech constraints

* Language: **Rust**
* GUI: **eframe/egui** (native; no webview)
* Dependencies allowed: `eframe`, `egui`, `egui_extras` (for Markdown), `serde`, `serde_json`, `anyhow`, `arboard` (optional copy), `rfd` (optional file dialog)
* Target: cross-platform (macOS/Windows/Linux)
* Single crate. No network access.

## Input formats

* Accept either:

  1. A **JSON array** of messages (the common OpenAI shape)
  2. **JSONL**: one JSON object per line (each line = one message)
* Message schema (baseline):

  ```json
  { "role": "system" | "user" | "assistant", "content": "string" }
  ```
* If `role` is unknown (e.g., "tool"), render it as a neutral bubble with the role label.
* Ignore extra fields safely.

### Example inputs

**JSON (array)**

```json
[
  {"role": "system", "content": "あなたは日本語ネイティブで親切なAIアシスタントです。"},
  {"role": "user", "content": "こんにちは。ご機嫌いかがですか？"},
  {"role": "assistant", "content": "とても元気です。あなたのお役にたてることがあれば何なりとお尋ねください。"},
  {"role": "user", "content": "{question}"},
  {"role": "assistant", "content": ""}
]
```

**JSONL**

```
{"role":"system","content":"あなたは日本語ネイティブで親切なAIアシスタントです。"}
{"role":"user","content":"こんにちは。ご機嫌いかがですか？"}
{"role":"assistant","content":"とても元気です。あなたのお役にたてることがあれば何なりとお尋ねください。"}
{"role":"user","content":"{question}"}
{"role":"assistant","content":""}
```

## UX requirements

* Window title: `LLM Log Viewer`
* **Drag & Drop**: user can drop a `.json` or `.jsonl` file onto the window to load.
* **Menu/Buttons** (top bar):

  * “Open file…” (fallback if drag & drop unavailable)
  * “Clear”
  * “Theme: Light/Dark” toggle
  * “Copy as Markdown” (copies reconstructed conversation including System)
* **System message is visible** at top in a dedicated card:

  * Title “System”
  * Body rendered as Markdown
  * Subtle background (distinct from user/assistant)
* **Conversation area** (scrollable, auto-scroll to top on load):

  * **User**: right-aligned “bubble”
  * **Assistant**: left-aligned “bubble”
  * **Other roles**: left-aligned bubble with role badge
  * Render `content` as **Markdown** (use `egui_extras::markdown`)
* **Status line** at bottom: filename, number of turns, parse errors (if any)

## Visual details

* Use `egui` spacing/paddings; rounded rectangles for bubbles.
* Colors:

  * System card: subtle yellow/gray background
  * User bubble: slightly accented background
  * Assistant bubble: neutral background
  * Unknown roles: gray with a small badge label (e.g., “tool”)
* Monospace for code blocks via Markdown renderer.

## Parsing rules & validation

* Detect format:

  * Trim leading whitespace; if first non-space char is `[` → parse as JSON array.
  * Else → treat as JSONL (split by lines, ignore empty/whitespace lines).
* Robust error handling:

  * If file can’t parse, show a non-blocking error banner and keep previous state.
  * If some lines fail in JSONL, load the valid ones and list how many failed.
* Normalize whitespace and preserve line breaks in `content`.
* If `content` is empty, show “(empty)”.

## Architecture

* `main.rs` launches `eframe::run_native`.
* `AppState`:

  ```rust
  struct AppState {
      theme_dark: bool,
      file_name: Option<String>,
      system: Option<String>,
      messages: Vec<Msg>, // user/assistant/other (excluding system)
      errors: Vec<String>,
  }

  #[derive(serde::Deserialize, serde::Serialize, Clone)]
  struct RawMsg { role: String, content: String }

  enum Role { System, User, Assistant, Other(String) }

  struct Msg { role: Role, content: String }
  ```
* Functions:

  * `load_from_path(path: &Path) -> Result<Loaded, anyhow::Error>`
  * `parse_json(bytes: &[u8]) -> Result<Vec<RawMsg>>`
  * `parse_jsonl(bytes: &[u8]) -> Result<Vec<RawMsg>>`
  * `normalize(raw: Vec<RawMsg>) -> Loaded` (extract first/earliest system to `state.system`, keep remaining in `state.messages`)
  * `render_markdown(ui: &mut egui::Ui, text: &str)` using `egui_extras::markdown`
  * `to_markdown(state: &AppState) -> String` (see below)
* Drag & Drop: handle `ctx.input(|i| i.raw.dropped_files.clone())`.

## “Copy as Markdown” format

Generate a single Markdown string such as:

```md
# System
あなたは日本語ネイティブで親切なAIアシスタントです。

---

**User**  
こんにちは。ご機嫌いかがですか？

**Assistant**  
とても元気です。あなたのお役にたてることがあれば何なりとお尋ねください。

**User**  
{question}

**Assistant**  
(empty)
```

* Preserve original message order.
* Include unknown roles as `**<role>**`.

## Edge cases

* Multiple system messages: show the **first** as System card, then render others inline as “System (extra)” bubbles.
* Very long conversations: ensure scroll performance is acceptable.
* Extremely long lines/code blocks: ensure horizontal scrolling inside code blocks works (Markdown defaults).
* Non-UTF8 files: show an error.
* Files larger than \~20MB: show a warning but attempt to load.

## Non-goals

* No editing of messages.
* No network calls.
* No embedding or model execution.

## Deliverables

1. `Cargo.toml` with listed deps.
2. `main.rs` implementing the full app as above.
3. `README.md` with:

   * Build/run:

     ```
     cargo run --release
     ```
   * Usage:

     * Drag & drop `.json` or `.jsonl`
     * Or “Open file…”
     * Theme toggle, Clear, Copy as Markdown
   * Input spec & examples (include the two examples above).
4. A small `sample.json` and `sample.jsonl` in `samples/`.

## Acceptance criteria

* Drag & drop a JSON **array** file → Renders with System card + bubbles.
* Drag & drop a **JSONL** file → Same result.
* Top bar shows file name, message count; bottom status shows any parse warnings.
* System appears and is visually distinct.
* User = right-aligned, Assistant = left-aligned.
* Markdown (headers, lists, code blocks) renders correctly.
* “Copy as Markdown” places full conversation into clipboard (or shows in a modal if clipboard unsupported).
* No panics on malformed lines; errors are surfaced gracefully.

## Nice-to-have (optional if trivial)

* Keyboard shortcut: ⌘/Ctrl+O (Open), ⌘/Ctrl+L (Clear), ⌘/Ctrl+C (Copy as Markdown), ⌘/Ctrl+J (Theme toggle).
* Drop zone overlay (“Drop JSON/JSONL here”).

---

**Implement now.**
