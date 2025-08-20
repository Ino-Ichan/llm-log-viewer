use anyhow::{anyhow, Context, Result};
use eframe::{egui, egui::{Align, Align2, Color32, Frame, Id, Label, Layout, RichText, Rounding, ScrollArea, Vec2}};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

fn main() -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(Vec2::new(900.0, 700.0)),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "LLM Log Viewer",
        native_options,
        Box::new(|cc| {
            // Default visuals
            let app = AppState::default();
            app.apply_theme(cc.egui_ctx.clone());
            Box::new(app)
        }),
    ) {
        eprintln!("eframe error: {e}");
    }
    Ok(())
}

struct AppState {
    theme_dark: bool,
    text_scale: f32,
    file_name: Option<String>,
    system: Option<String>,
    messages: Vec<Msg>,
    errors: Vec<String>,

    // UI helpers
    scroll_area_key: String,
    show_drop_overlay: bool,
    md_cache: CommonMarkCache,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct RawMsg {
    role: String,
    content: String,
}

#[derive(Clone, Debug)]
enum Role {
    System,
    User,
    Assistant,
    Other(String),
}

#[derive(Clone, Debug)]
struct Msg {
    role: Role,
    content: String,
}

#[derive(Default, Clone)]
struct Loaded {
    file_name: Option<String>,
    system: Option<String>,
    messages: Vec<Msg>,
    errors: Vec<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            theme_dark: true,
            text_scale: 1.0,
            file_name: None,
            system: None,
            messages: vec![],
            errors: vec![],
            scroll_area_key: String::new(),
            show_drop_overlay: false,
            md_cache: CommonMarkCache::default(),
        }
    }
}

impl AppState {
    fn apply_theme(&self, ctx: egui::Context) {
        if self.theme_dark {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }
    }

    fn set_loaded(&mut self, loaded: Loaded) {
        self.file_name = loaded.file_name;
        self.system = loaded.system;
        self.messages = loaded.messages;
        self.errors = loaded.errors;
        // Reset scroll position by changing the scroll area id key
        self.scroll_area_key = self
            .file_name
            .clone()
            .unwrap_or_else(|| "__empty__".to_string());
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Log", &["json", "jsonl"]) // not exclusive
                        .pick_file()
                    {
                        match load_from_path(&path) {
                            Ok(loaded) => self.set_loaded(loaded),
                            Err(e) => self.errors.push(format!("Failed to load: {e}")),
                        }
                    }
                }

                if ui.button("Clear").clicked() {
                    let keep_scale = self.text_scale;
                    *self = AppState { theme_dark: self.theme_dark, text_scale: keep_scale, ..Default::default() };
                    self.apply_theme(ctx.clone());
                }

                let theme_label = if self.theme_dark { "Theme: Dark" } else { "Theme: Light" };
                if ui.button(theme_label).clicked() {
                    self.theme_dark = !self.theme_dark;
                    self.apply_theme(ctx.clone());
                }

                if ui.button("Copy as Markdown").clicked() {
                    let md = to_markdown(self);
                    ui.output_mut(|o| o.copied_text = md);
                }

                if ui.button("Export HTML…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("HTML", &["html", "htm"]) // not exclusive
                        .set_file_name("chat.html")
                        .save_file()
                    {
                        let html = to_html(self);
                        if let Err(e) = fs::write(&path, html) {
                            self.errors.push(format!("Failed to export HTML: {e}"));
                        }
                    }
                }

                ui.separator();
                ui.label("Text size");
                let mut scale = self.text_scale;
                let before = scale;
                ui.add(egui::Slider::new(&mut scale, 0.8..=1.6).step_by(0.05));
                if (scale - before).abs() > f32::EPSILON {
                    self.text_scale = scale;
                }

                ui.separator();
                if let Some(name) = &self.file_name {
                    ui.label(RichText::new(name).italics());
                } else {
                    ui.label(RichText::new("No file loaded").italics());
                }
                ui.separator();
                ui.label(format!("Messages: {}", self.messages.len()));
            });
        });

        // Error banner (non-blocking)
        if !self.errors.is_empty() {
            egui::TopBottomPanel::top("error_bar").show(ctx, |ui| {
                Frame::none()
                    .fill(Color32::from_rgb(255, 235, 238))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let msg = self.errors.join(" • ");
                            ui.colored_label(Color32::from_rgb(183, 28, 28), msg);
                            if ui.button("Dismiss").clicked() {
                                self.errors.clear();
                            }
                        });
                    });
            });
        }

        // Central content with drag&drop handling
        egui::CentralPanel::default().show(ctx, |ui| {
            // Handle file drops without any overlay, to avoid interfering with text selection
            let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
            if !dropped_files.is_empty() {
                // Try loading first valid path or bytes
                for f in dropped_files {
                    if let Some(path) = f.path {
                        match load_from_path(&path) {
                            Ok(loaded) => {
                                self.set_loaded(loaded);
                                break;
                            }
                            Err(e) => self.errors.push(format!("Failed to load dropped file: {e}")),
                        }
                    } else if let Some(bytes) = f.bytes {
                        match load_from_bytes(&bytes) {
                            Ok(mut loaded) => {
                                loaded.file_name = Some("(dropped)".to_string());
                                self.set_loaded(loaded);
                                break;
                            }
                            Err(e) => self.errors.push(format!("Failed to parse dropped bytes: {e}")),
                        }
                    }
                }
            }

            // No drag & drop overlay; prioritize text selection UX

            // Conversation rendering
            let scroll_id = Id::new("scroll_conversation").with(self.scroll_area_key.clone());
            ScrollArea::vertical()
                .id_source(scroll_id)
                // Do not shrink horizontally (keep full width), but allow vertical to fit content
                .auto_shrink([false, true])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    ui.add_space(6.0);

                    // System card
                    if let Some(sys) = &self.system {
                        render_system_card(ui, sys, &mut self.md_cache, self.text_scale);
                        ui.add_space(6.0);
                    }

                    // Messages
                    let content_width = ui.available_width();
                    for (idx, msg) in self.messages.iter().enumerate() {
                        render_message_bubble(ui, msg, idx, content_width, self.theme_dark, &mut self.md_cache, self.text_scale);
                        ui.add_space(6.0);
                    }

                    // Ensure the last Copy bar isn't clipped at the bottom
                    ui.add_space(18.0);
                });
        });

        // Bottom status line
        egui::TopBottomPanel::bottom("status_line").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                let fname = self
                    .file_name
                    .clone()
                    .unwrap_or_else(|| "(no file)".to_string());
                ui.label(format!("File: {}", fname));
                ui.separator();
                ui.label(format!("Turns: {}", self.messages.len()));
                if !self.errors.is_empty() {
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(183, 28, 28), format!("Warnings: {}", self.errors.len()));
                }
            });
        });
    }
}

// ---------------- Parsing & Loading ----------------

fn load_from_path(path: &Path) -> Result<Loaded> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut loaded = load_from_bytes(&bytes)?;
    loaded.file_name = Some(
        path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string()),
    );
    Ok(loaded)
}

fn load_from_bytes(bytes: &[u8]) -> Result<Loaded> {
    if bytes.len() > 20 * 1024 * 1024 {
        // ~20MB warning
        // allocate after checking encoding; no extra temp needed
        let text = std::str::from_utf8(bytes).map_err(|_| anyhow!("Non-UTF8 file"))?;
        let (raws, mut warnings) = detect_and_parse(text)?;
        let mut l = normalize(raws);
        l.errors.append(&mut warnings);
        l.errors.push("File larger than ~20MB".to_string());
        return Ok(l);
    }

    let text = std::str::from_utf8(bytes).map_err(|_| anyhow!("Non-UTF8 file"))?;
    let (raws, warnings) = detect_and_parse(text)?;
    let mut l = normalize(raws);
    l.errors.extend(warnings);
    Ok(l)
}

fn detect_and_parse(text: &str) -> Result<(Vec<RawMsg>, Vec<String>)> {
    let first_non_ws = text.chars().find(|c| !c.is_whitespace());
    let mut warnings = Vec::new();
    let raws = match first_non_ws {
        Some('[') => parse_json(text.as_bytes())?,
        _ => {
            let (msgs, failed) = parse_jsonl_with_errors(text.as_bytes())?;
            if failed > 0 {
                warnings.push(format!("{} JSONL line(s) failed to parse", failed));
            }
            msgs
        }
    };
    Ok((raws, warnings))
}

fn parse_json(bytes: &[u8]) -> Result<Vec<RawMsg>> {
    let v: Vec<RawMsg> = serde_json::from_slice(bytes).context("JSON array parse error")?;
    Ok(v)
}

fn parse_jsonl(bytes: &[u8]) -> Result<Vec<RawMsg>> {
    // Basic version without exposing failures
    let text = std::str::from_utf8(bytes).map_err(|_| anyhow!("Non-UTF8 file"))?;
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() { continue; }
        match serde_json::from_str::<RawMsg>(line) {
            Ok(m) => out.push(m),
            Err(_e) => {
                // Skip silently here; load_from_bytes uses the variant with errors.
                eprintln!("Warning: failed to parse JSONL line {}", idx + 1);
            }
        }
    }
    Ok(out)
}

fn parse_jsonl_with_errors(bytes: &[u8]) -> Result<(Vec<RawMsg>, usize)> {
    let text = std::str::from_utf8(bytes).map_err(|_| anyhow!("Non-UTF8 file"))?;
    let mut out = Vec::new();
    let mut failed = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        match serde_json::from_str::<RawMsg>(line) {
            Ok(m) => out.push(m),
            Err(_) => failed += 1,
        }
    }
    Ok((out, failed))
}

fn normalize(raw: Vec<RawMsg>) -> Loaded {
    let mut system: Option<String> = None;
    let mut messages: Vec<Msg> = Vec::new();
    for rm in raw {
        let cleaned = trim_chat_whitespace(&rm.content);
        let content = if cleaned.trim().is_empty() { "(empty)".to_string() } else { cleaned };
        let role_lower = rm.role.to_lowercase();
        match role_lower.as_str() {
            "system" => {
                if system.is_none() {
                    system = Some(content);
                } else {
                    messages.push(Msg { role: Role::Other("System (extra)".into()), content });
                }
            }
            "user" => messages.push(Msg { role: Role::User, content }),
            "assistant" => messages.push(Msg { role: Role::Assistant, content }),
            other => messages.push(Msg { role: Role::Other(other.to_string()), content }),
        }
    }
    Loaded { file_name: None, system, messages, errors: Vec::new() }
}

// ---------------- Rendering helpers ----------------

fn render_system_card(ui: &mut egui::Ui, text: &str, cache: &mut CommonMarkCache, scale: f32) {
    let fill = ui.visuals().extreme_bg_color.linear_multiply(0.9);
    // Allocate a column with a right gutter so the card doesn't sit under the scrollbar
    let full = ui.available_width();
    let right_gutter = 20.0; // requested gutter
    let lane_w = (full - right_gutter).max(0.0);
    // Move the system card's right edge left so it doesn't intrude into the user icon + gap area
    let avatar_w = 28.0;
    let gap = 8.0;
    let sys_right_inset = avatar_w + gap; // align roughly with user's bubble右端
    let sys_w = (lane_w - sys_right_inset).max(0.0);
    ui.allocate_ui_with_layout(egui::vec2(sys_w, 0.0), Layout::top_down(Align::LEFT), |col| {
        Frame::group(col.style())
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, col.visuals().widgets.noninteractive.fg_stroke.color))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(col, |ui| {
                // Force the frame to take the conversation lane width (minus gutter and inset)
                ui.set_min_width(sys_w);
                ui.set_max_width(sys_w);
                ui.label(RichText::new("System").strong());
                ui.add_space(6.0);
                // Use the full message lane width so it aligns with chat lanes
                let w = sys_w;
                render_markdown_with_width(ui, text, w, cache, Some(scale), "sys");
            });
    });
}

fn render_message_bubble(ui: &mut egui::Ui, msg: &Msg, index: usize, content_width: f32, dark: bool, cache: &mut CommonMarkCache, scale: f32) {
    // Palette close to ChatGPT: assistant neutral gray, user green-tinted
    let (bg, align_right, role_badge, avatar_bg, avatar_fg, avatar_initial) = match &msg.role {
        Role::User => (
            if dark { Color32::from_rgb(20, 80, 60) } else { Color32::from_rgb(219, 247, 230) },
            true,
            None,
            if dark { Color32::from_rgb(48, 200, 120) } else { Color32::from_rgb(16, 163, 127) },
            Color32::WHITE,
            "U".to_string(),
        ),
        Role::Assistant => (
            if dark { Color32::from_rgb(45, 45, 45) } else { Color32::from_rgb(246, 246, 246) },
            false,
            None,
            if dark { Color32::from_rgb(100, 100, 100) } else { Color32::from_rgb(200, 200, 200) },
            if dark { Color32::WHITE } else { Color32::BLACK },
            "A".to_string(),
        ),
        Role::Other(r) => (
            if dark { Color32::from_rgb(60, 60, 60) } else { Color32::from_rgb(232, 232, 232) },
            false,
            Some(r.clone()),
            if dark { Color32::from_rgb(120, 120, 120) } else { Color32::from_rgb(180, 180, 180) },
            Color32::WHITE,
            r.chars().next().unwrap_or('?').to_ascii_uppercase().to_string(),
        ),
        Role::System => (
            if dark { Color32::from_rgb(90, 90, 20) } else { Color32::from_rgb(255, 250, 220) },
            false,
            Some("System".into()),
            if dark { Color32::from_rgb(160, 130, 20) } else { Color32::from_rgb(230, 200, 80) },
            Color32::BLACK,
            "S".to_string(),
        ),
    };

    let layout = if align_right {
        Layout::right_to_left(Align::TOP)
    } else {
        Layout::left_to_right(Align::TOP)
    };

    ui.vertical(|ui| {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0; // eliminate default vertical gaps inside a message

            let mut bubble_w_for_copy: f32 = 0.0;
            ui.with_layout(layout, |ui| {
                let avail = content_width;
                let avatar_w = 28.0;
                let gap = 8.0;
                // Assistant column max width (left side), cap for readability
                let assist_max_width = (avail - avatar_w - gap).min(800.0).max(160.0);
                // User bubbles expand leftward only: cap their maximum so they stop a bit
                // to the right of the assistant's left edge.
                let user_left_offset = 8.0; // "少しだけ右" のマージン
                // Add a right-side gutter to avoid overlap with the vertical scrollbar and clipping.
                let right_gutter = 20.0;
                let avail_user = (avail - right_gutter).max(0.0);
                let user_max_width = (assist_max_width - (avatar_w + gap) - user_left_offset)
                    .min((avail_user - avatar_w - gap).max(160.0))
                    .max(160.0);
                // Let bubbles shrink based on content for a more natural width (user only)
                let bubble_width = if align_right {
                    preferred_bubble_width(ui, &msg.content, user_max_width, scale)
                } else {
                    assist_max_width
                };

                // Build row: avatar + bubble (or reversed for right alignment)
                if align_right {
                    // Avatar at the far right, then bubble to its left
                    // Move avatar further right: smaller pre-gutter inside the row.
                    ui.add_space(8.0);
                    draw_avatar(ui, &avatar_initial, avatar_bg, avatar_fg);
                    ui.add_space(gap);
                    let role_label = match &msg.role {
                        Role::User => "User".to_string(),
                        Role::Assistant => "Assistant".to_string(),
                        Role::System => "System".to_string(),
                        Role::Other(r) => title_case(r),
                    };
                    bubble_w_for_copy = bubble_width;
                    let key = format!("msg-{}", index);
                    // Constrain bubble and copy bar to the same fixed-width column sized to bubble.
                    // Align RIGHT inside the column so the bubble's右端 is constant next to the avatar.
                    ui.allocate_ui_with_layout(egui::vec2(bubble_width, 0.0), Layout::top_down(Align::RIGHT), |col| {
                        render_bubble(col, bg, bubble_width, role_badge.as_ref(), &msg.content, cache, scale, &role_label, false, &key);
                        col.add_space(2.0);
                        render_copy_bar(col, bubble_width, &role_label, &msg.content, true);
                    });
                } else {
                    // Avatar left, then bubble
                    draw_avatar(ui, &avatar_initial, avatar_bg, avatar_fg);
                    ui.add_space(gap);
                    let role_label = match &msg.role {
                        Role::User => "User".to_string(),
                        Role::Assistant => "Assistant".to_string(),
                        Role::System => "System".to_string(),
                        Role::Other(r) => title_case(r),
                    };
                    // Assistant: bubble and copy bar in the same fixed-width column
                    bubble_w_for_copy = assist_max_width;
                    let key = format!("msg-{}", index);
                    ui.allocate_ui_with_layout(egui::vec2(assist_max_width, 0.0), Layout::top_down(Align::LEFT), |col| {
                        render_bubble(col, bg, assist_max_width, role_badge.as_ref(), &msg.content, cache, scale, &role_label, false, &key);
                        col.add_space(2.0);
                        render_copy_bar(col, assist_max_width, &role_label, &msg.content, false);
                    });
                }
            });

        });

        // Leave inter-message spacing to the outer loop for consistency
    });
}

fn draw_avatar(ui: &mut egui::Ui, initial: &str, bg: Color32, fg: Color32) {
    let size = egui::vec2(28.0, 28.0);
    let (rect, _resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let radius = size.x.min(size.y) * 0.5;
    let painter = ui.painter();
    painter.circle_filled(rect.center(), radius, bg);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
        fg,
    );
}

fn render_bubble(
    ui: &mut egui::Ui,
    bg: Color32,
    max_width: f32,
    role_badge: Option<&String>,
    content: &str,
    cache: &mut CommonMarkCache,
    scale: f32,
    role_label: &str,
    copy_inside_left: bool,
    viewer_key: &str,
) {
    Frame::none()
        .fill(bg)
        .rounding(Rounding::same(14.0))
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.set_max_width(max_width);
            // Optional role badge at the top for non-user/assistant roles
            if let Some(badge) = role_badge {
                let badge = format!("{}", title_case(badge));
                ui.add(Label::new(RichText::new(badge).small().italics()).wrap(true));
                ui.add_space(4.0);
            }

            // Main content
            render_markdown_with_width(ui, content, max_width, cache, Some(scale), viewer_key);
            if copy_inside_left {
                ui.add_space(6.0);
                // Bottom-right inside bubble for assistant
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Copy").on_hover_text("Copy this message").clicked() {
                        let md = format!("**{}**  \n{}\n", role_label, content);
                        ui.output_mut(|o| o.copied_text = md);
                    }
                });
            }
        });
}

fn render_copy_bar(ui: &mut egui::Ui, max_width: f32, role_label: &str, content: &str, align_right: bool) {
    // Subtle bar under the bubble with configurable alignment
    Frame::none()
        .show(ui, |ui| {
            ui.set_min_width(max_width);
            ui.set_max_width(max_width);
            if align_right {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Copy").on_hover_text("Copy this message").clicked() {
                        let md = format!("**{}**  \n{}\n", role_label, content);
                        ui.output_mut(|o| o.copied_text = md);
                    }
                });
            } else {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if ui.small_button("Copy").on_hover_text("Copy this message").clicked() {
                        let md = format!("**{}**  \n{}\n", role_label, content);
                        ui.output_mut(|o| o.copied_text = md);
                    }
                });
            }
        });
}

fn render_markdown_with_width(
    ui: &mut egui::Ui,
    text: &str,
    content_width: f32,
    cache: &mut CommonMarkCache,
    scale_override: Option<f32>,
    viewer_key: &str,
) {
    ui.set_max_width(content_width);
    // Use a stable-but-unique viewer id per text to avoid layout/cache collisions
    let id = format!("{}:{}", viewer_key, short_hash(text));
    let mut viewer = CommonMarkViewer::new(&id);
    // Sanitize common chat artifacts that look like code fences
    let sanitized = sanitize_chat_markdown(text);
    // Apply chat-only text scaling by temporarily adjusting text styles
    let content_scale = scale_override.unwrap_or_else(|| {
        // Read from a global-like hint stored via Ui memory? We don't have it here,
        // so caller should pass explicit scale. Fallback to 1.0.
        1.0
    });

    if (content_scale - 1.0).abs() < f32::EPSILON {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0; // minimize intra-markdown vertical gaps
            viewer.show(ui, cache, &sanitized);
        });
        return;
    }

    let prev_style_arc = ui.style().clone();
    let prev_style: egui::Style = (*prev_style_arc).clone();
    let mut style = prev_style.clone();
    for ts in [egui::TextStyle::Body, egui::TextStyle::Monospace, egui::TextStyle::Heading] {
        if let Some(font) = style.text_styles.get_mut(&ts) {
            font.size *= content_scale;
        }
    }
    ui.set_style(style);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        viewer.show(ui, cache, &sanitized);
    });
    ui.set_style(prev_style);
}

// Markdown parsing is delegated to egui_commonmark.

fn to_markdown(state: &AppState) -> String {
    let mut out = String::new();
    if let Some(sys) = &state.system {
        out.push_str("# System\n");
        out.push_str(sys);
        out.push_str("\n\n---\n\n");
    }
    for msg in &state.messages {
        let role_label = match &msg.role {
            Role::User => "User".to_string(),
            Role::Assistant => "Assistant".to_string(),
            Role::System => "System".to_string(),
            Role::Other(r) => title_case(r),
        };
        out.push_str(&format!("**{}**  \n{}\n\n", role_label, msg.content));
    }
    out
}

fn to_html(state: &AppState) -> String {
    let mut out = String::new();
    let dark = state.theme_dark;
    let (bg_body, fg_body, bg_assist, bg_user, avatar_user_bg, avatar_assist_bg, avatar_user_fg, avatar_assist_fg) = if dark {
        (
            "#121212", "#eaeaea",
            "#2d2d2d", "#14503c",
            "#30c878", "#646464",
            "#ffffff", "#ffffff",
        )
    } else {
        (
            "#ffffff", "#222222",
            "#f6f6f6", "#dbf7e6",
            "#10a37f", "#c8c8c8",
            "#ffffff", "#000000",
        )
    };

    out.push_str("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str("<title>Chat Export</title>\n<style>\n");
    out.push_str(&format!(
        "body {{ background:{}; color:{}; font: 14px/1.5 -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Noto Sans', 'Hiragino Sans', 'Yu Gothic UI', Arial, sans-serif; margin:0; }}\n",
        bg_body, fg_body
    ));
    out.push_str(".container{ max-width: 940px; margin:24px auto; padding:0 16px;}\n");
    out.push_str(".system{ border:1px solid rgba(127,127,127,0.25); border-radius:8px; padding:10px 12px; margin-bottom:10px;}\n");
    out.push_str(&format!(
        ".row{{ display:flex; align-items:flex-start; gap:8px; margin:10px 0; }}\n.bubble{{ border-radius:14px; padding:10px 12px; max-width:800px; display:inline-block; overflow-wrap:anywhere; word-break:break-word; white-space:pre-wrap; box-sizing:border-box; }}\n.assist .bubble{{ background:{}; }}\n.user .bubble{{ background:{}; }}\n",
        bg_assist, bg_user
    ));
    out.push_str(&format!(
        ".avatar{{ width:28px; height:28px; border-radius:50%; display:flex; align-items:center; justify-content:center; font-weight:600; font-size:14px; }}\n.user .avatar{{ background:{}; color:{}; }}\n.assist .avatar{{ background:{}; color:{}; }}\n",
        avatar_user_bg, avatar_user_fg, avatar_assist_bg, avatar_assist_fg
    ));
    out.push_str(".assist{ justify-content:flex-start;}\n");
    out.push_str(".user{ justify-content:flex-end;}\n");
    out.push_str(".content{ }\n");
    out.push_str(".role{ font-weight:600; margin-bottom:6px; opacity:0.8;}\n");
    out.push_str(".bubble pre, .system pre{ background: rgba(127,127,127,0.15); border:1px solid rgba(127,127,127,0.25); border-radius:8px; padding:10px; overflow:auto; white-space:pre; margin:8px 0 0 0; }\n");
    out.push_str(".bubble code{ font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", \"Courier New\", monospace; font-size: 0.95em; }\n");
    out.push_str("</style></head><body><div class=\"container\">\n");

    if let Some(sys) = &state.system {
        out.push_str("<div class=\"system\">\n<div class=\"role\">System</div>\n");
        let sanitized = sanitize_chat_markdown(sys);
        out.push_str(&format!("<div class=\"content\">{}</div>\n", text_to_html_with_fences(&sanitized)));
        out.push_str("</div>\n");
    }

    for msg in &state.messages {
        let (cls, role, initial, show_role_badge) = match &msg.role {
            Role::User => ("user", "User", "U", false),
            Role::Assistant => ("assist", "Assistant", "A", false),
            Role::System => ("assist", "System", "S", true),
            Role::Other(r) => ("assist", &*title_case(r), "?", true),
        };
        out.push_str(&format!("<div class=\"row {}\">\n", cls));
        if matches!(&msg.role, Role::User) {
            // User: bubble first (right側に気泡、その右にアバター)
            out.push_str("<div class=\"bubble\">\n");
            if show_role_badge {
                out.push_str(&format!("<div class=\"role\">{}</div>\n", html_escape(role)));
            }
            let sanitized = sanitize_chat_markdown(&msg.content);
            out.push_str(&format!("<div class=\"content\">{}</div>\n", text_to_html_with_fences(&sanitized)));
            out.push_str("</div>\n");
            out.push_str(&format!("<div class=\"avatar\">{}</div>\n", html_escape(initial)));
        } else {
            // Assistant/Other: avatar first, then bubble
            out.push_str(&format!("<div class=\"avatar\">{}</div>\n", html_escape(initial)));
            out.push_str("<div class=\"bubble\">\n");
            if show_role_badge {
                out.push_str(&format!("<div class=\"role\">{}</div>\n", html_escape(role)));
            }
            let sanitized = sanitize_chat_markdown(&msg.content);
            out.push_str(&format!("<div class=\"content\">{}</div>\n", text_to_html_with_fences(&sanitized)));
            out.push_str("</div>\n");
        }
        out.push_str("</div>\n");
    }

    out.push_str("</div></body></html>\n");
    out
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\\' => out.push_str("&#92;"),
            _ => out.push(c),
        }
    }
    out
}

fn text_to_html_with_fences(s: &str) -> String {
    // Convert a subset of Markdown-like fences ```lang ... ``` into <pre><code> blocks.
    // Outside code blocks, escape HTML and keep newlines (white-space: pre-wrap in CSS handles them).
    let mut out = String::new();
    let mut in_fence = false;
    let mut fence_lang: Option<String> = None;
    for line in s.lines() {
        let trimmed = line.trim_start();
        if !in_fence {
            if let Some(rest) = trimmed.strip_prefix("```") {
                // Start of fence
                let lang = rest.trim();
                let lang_safe = if lang.is_empty() { None } else { Some(lang.to_string()) };
                fence_lang = lang_safe;
                if let Some(lang) = &fence_lang {
                    out.push_str(&format!("<pre><code class=\"language-{}\">", html_escape(lang)));
                } else {
                    out.push_str("<pre><code>");
                }
                in_fence = true;
            } else {
                out.push_str(&html_escape(line));
                out.push('\n');
            }
        } else {
            // In fence: check for closing fence
            if trimmed.starts_with("```") && trimmed.trim() == "```" {
                out.push_str("</code></pre>\n");
                in_fence = false;
                fence_lang = None;
            } else {
                out.push_str(&html_escape(line));
                out.push('\n');
            }
        }
    }
    if in_fence {
        out.push_str("</code></pre>\n");
    }
    out
}

fn title_case(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &it.as_str().to_lowercase(),
    }
}

// Best-effort sanitizer to prevent accidental fenced code blocks from
// stray triple backticks that appear in natural language, especially at EOL.
fn sanitize_chat_markdown(input: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_fence = false;
    for line in input.lines() {
        let trimmed_start = line.trim_start();
        if !in_fence {
            if let Some(rest) = trimmed_start.strip_prefix("```") {
                // Determine if this looks like a legit fence opener (``` or ```lang)
                let after = rest.trim_end();
                let only_spaces_after = rest.chars().all(char::is_whitespace);
                let looks_like_lang = {
                    let lang = after.trim();
                    !lang.is_empty() && lang.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '+' | '-' | '.' | '#'))
                };
                if only_spaces_after || looks_like_lang {
                    in_fence = true;
                    out.push(line.to_string());
                } else {
                    // Likely narrative text like "``` code block." — break the fence
                    let broken = line.replacen("```", "``\u{200B}`", 1);
                    out.push(broken);
                }
            } else {
                // If a line ends with stray triple backticks, break them to avoid starting a fence
                if line.ends_with("```") {
                    let mut s = line.to_string();
                    let _ = s.split_off(s.len() - 3);
                    out.push(format!("{}{}", s, "``\u{200B}`"));
                } else {
                    out.push(line.to_string());
                }
            }
        } else {
            // Inside a fence; detect a proper closing fence (``` on its own or with only spaces)
            if trimmed_start.starts_with("```") && trimmed_start.trim() == "```" {
                in_fence = false;
            }
            out.push(line.to_string());
        }
    }

    // If a fence was left open unintentionally, close it to avoid swallowing following UI
    if in_fence {
        out.push("```".to_string());
    }
    out.join("\n")
}

fn short_hash(s: &str) -> u64 {
    // Simple FNV-1a 64-bit
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// Trim excess whitespace typical in logs: leading blank lines and trailing whitespace/newlines
fn trim_chat_whitespace(input: &str) -> String {
    let mut s = input.to_string();
    // Trim trailing spaces/tabs/newlines
    while s.ends_with([' ', '\t', '\n', '\r']) {
        s.pop();
    }
    // Trim leading newlines (but keep regular leading spaces for indentation in code)
    while s.starts_with(['\n', '\r']) {
        s.remove(0);
    }
    s
}

fn preferred_bubble_width(ui: &egui::Ui, text: &str, max_width: f32, scale: f32) -> f32 {
    // Heuristic: estimate width from first non-empty line length and body font size
    let first_line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let style = ui.style().clone();
    let body_size = style.text_styles.get(&egui::TextStyle::Body).map(|f| f.size).unwrap_or(14.0);
    // Approximate average character width as ~0.55 of font size
    let char_w = body_size * 0.55 * scale;
    let len = first_line.chars().take(80).count() as f32; // cap to avoid overly wide single lines
    let padding = 24.0; // inner margins in bubble frame roughly
    let desired = padding + len * char_w;
    desired.clamp(160.0, max_width)
}
