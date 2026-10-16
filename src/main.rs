#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, text::LayoutJob, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, TextFormat, Vec2};
use std::path::PathBuf;
use std::process::Command;

// Familiar post-it colours: (paper, ink)
const NOTES: [(u32, u32); 6] = [
    (0xFFEB3B, 0x4A3F00), // canary yellow
    (0xFF6EB4, 0x4D0A2C), // neon pink
    (0x4FD8FF, 0x00394D), // aqua blue
    (0x8EE53F, 0x1E4000), // lime green
    (0xFFA52E, 0x4D2600), // orange
    (0xB78CFF, 0x2E0F66), // purple
];

fn rgb(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

fn seed() -> usize {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0)
}

struct App {
    text: String,
    path: Option<PathBuf>,
    color: usize,
    menu_open: bool,
    prompt: Option<(bool, String)>, // fallback path prompt (save?, text)
    grip_off: Vec2,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        let mut s = egui::Style::default();
        s.visuals = egui::Visuals::light();
        s.visuals.selection.bg_fill = Color32::from_black_alpha(45);
        s.visuals.selection.stroke = Stroke::NONE;
        cc.egui_ctx.set_style(s);
        let mut color = seed() % NOTES.len();
        let mut file = None;
        for a in std::env::args().skip(1) {
            if let Some(n) = a.strip_prefix("--color=") {
                color = n.parse::<usize>().unwrap_or(0) % NOTES.len();
            } else {
                file = Some(PathBuf::from(a));
            }
        }
        let mut app = App { text: String::new(), path: None, color, menu_open: false, prompt: None, grip_off: Vec2::ZERO };
        if let Some(p) = file {
            app.open(p);
        }
        app
    }

    fn open(&mut self, p: PathBuf) {
        if let Ok(t) = std::fs::read_to_string(&p) {
            self.text = t;
            self.path = Some(p);
        }
    }

    fn new_note(&self) {
        if let Ok(exe) = std::env::current_exe() {
            let _ = Command::new(exe).arg(format!("--color={}", self.color + 1)).spawn();
        }
    }

    fn save_as(&mut self) {
        match pick(true) {
            Pick::Path(mut p) => {
                if p.extension().is_none() {
                    p.set_extension("md");
                }
                if std::fs::write(&p, &self.text).is_ok() {
                    self.path = Some(p);
                }
            }
            Pick::NoTool => self.prompt = Some((true, "note.md".into())),
            Pick::Cancel => {}
        }
    }

    fn save(&mut self) {
        match &self.path {
            Some(p) => {
                let _ = std::fs::write(p, &self.text);
            }
            None => self.save_as(),
        }
    }

    fn load(&mut self) {
        match pick(false) {
            Pick::Path(p) => self.open(p),
            Pick::NoTool => self.prompt = Some((false, String::new())),
            Pick::Cancel => {}
        }
    }
}

enum Pick {
    Path(PathBuf),
    NoTool,
    Cancel,
}

#[cfg(not(target_os = "linux"))]
fn pick(save: bool) -> Pick {
    let d = rfd::FileDialog::new().add_filter("Markdown", &["md", "markdown", "txt"]);
    let r = if save { d.set_file_name("note.md").save_file() } else { d.pick_file() };
    r.map_or(Pick::Cancel, Pick::Path)
}

// On Linux we use the desktop's own dialog tool: no extra libraries linked in.
#[cfg(target_os = "linux")]
fn pick(save: bool) -> Pick {
    let tries: [(&str, Vec<&str>); 2] = [
        ("zenity", if save { vec!["--file-selection", "--save", "--confirm-overwrite", "--filename=note.md"] } else { vec!["--file-selection"] }),
        ("kdialog", if save { vec!["--getsavefilename", "note.md"] } else { vec!["--getopenfilename", "."] }),
    ];
    for (bin, args) in tries {
        if let Ok(o) = Command::new(bin).args(args).output() {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            return if o.status.success() && !s.is_empty() { Pick::Path(PathBuf::from(s)) } else { Pick::Cancel };
        }
    }
    Pick::NoTool
}

// Light markdown styling: bigger headings, dimmed quotes and code.
fn layout_md(ui: &egui::Ui, s: &str, wrap: f32, ink: Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let mut in_code = false;
    for line in s.split_inclusive('\n') {
        let t = line.trim_start();
        let (size, color, mono) = if t.starts_with("```") {
            in_code = !in_code;
            (14.0, ink.gamma_multiply(0.55), true)
        } else if in_code {
            (14.0, ink.gamma_multiply(0.8), true)
        } else if t.starts_with("# ") {
            (24.0, ink, false)
        } else if t.starts_with("## ") {
            (20.0, ink, false)
        } else if t.starts_with('#') {
            (17.0, ink, false)
        } else if t.starts_with('>') {
            (15.0, ink.gamma_multiply(0.7), false)
        } else {
            (15.0, ink, false)
        };
        let font = if mono { FontId::monospace(size) } else { FontId::proportional(size) };
        job.append(line, 0.0, TextFormat { font_id: font, color, ..Default::default() });
    }
    ui.fonts(|f| f.layout_job(job))
}

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let (paper, ink) = (rgb(NOTES[self.color].0), rgb(NOTES[self.color].1));

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save();
        }

        let frame = egui::Frame::none().fill(paper).rounding(Rounding::same(8.0)).stroke(Stroke::new(1.0, Color32::from_black_alpha(40)));
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let full = ui.max_rect();

            // Header strip: drag to move, double-click to change colour
            let head = Rect::from_min_size(full.min, Vec2::new(full.width(), 34.0));
            let hr = ui.interact(head, ui.id().with("head"), Sense::click_and_drag());
            if hr.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if hr.double_clicked() {
                self.color = (self.color + 1) % NOTES.len();
            }

            // Hamburger (top-left)
            let hb = Rect::from_min_size(full.min + Vec2::new(6.0, 4.0), Vec2::splat(26.0));
            let hbr = ui.interact(hb, ui.id().with("menu"), Sense::click());
            if hbr.hovered() || self.menu_open {
                ui.painter().rect_filled(hb, Rounding::same(13.0), Color32::from_black_alpha(25));
            }
            for k in [-4.0, 0.0, 4.0] {
                let y = hb.center().y + k;
                ui.painter().line_segment([Pos2::new(hb.center().x - 6.0, y), Pos2::new(hb.center().x + 6.0, y)], Stroke::new(1.6, ink));
            }
            if hbr.clicked() {
                self.menu_open = !self.menu_open;
            }

            // Close (top-right)
            let cb = Rect::from_min_size(Pos2::new(full.max.x - 32.0, full.min.y + 4.0), Vec2::splat(26.0));
            let cbr = ui.interact(cb, ui.id().with("close"), Sense::click());
            if cbr.hovered() {
                ui.painter().rect_filled(cb, Rounding::same(13.0), Color32::from_black_alpha(25));
            }
            let c = cb.center();
            for (a, b) in [((-4.5, -4.5), (4.5, 4.5)), ((-4.5, 4.5), (4.5, -4.5))] {
                ui.painter().line_segment([c + Vec2::new(a.0, a.1), c + Vec2::new(b.0, b.1)], Stroke::new(1.6, ink));
            }
            if cbr.clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Editor
            let body = Rect::from_min_max(full.min + Vec2::new(14.0, 36.0), full.max - Vec2::new(12.0, 18.0));
            let mut bui = ui.child_ui(body, egui::Layout::top_down(egui::Align::LEFT));
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(&mut bui, |ui| {
                let mut layouter = |ui: &egui::Ui, s: &str, w: f32| layout_md(ui, s, w, ink);
                ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut self.text)
                        .frame(false)
                        .text_color(ink)
                        .layouter(&mut layouter)
                        .desired_width(f32::INFINITY),
                );
            });

            // Resize grip (bottom-right)
            let g = Rect::from_min_size(full.max - Vec2::splat(20.0), Vec2::splat(20.0));
            let gr = ui.interact(g, ui.id().with("grip"), Sense::drag());
            if gr.hovered() || gr.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            }
            for k in [5.0, 9.0, 13.0] {
                ui.painter().line_segment(
                    [Pos2::new(full.max.x - 3.0, full.max.y - k), Pos2::new(full.max.x - k, full.max.y - 3.0)],
                    Stroke::new(1.2, ink.gamma_multiply(0.5)),
                );
            }
            if gr.drag_started() {
                let p = ctx.pointer_interact_pos().unwrap_or(full.max);
                self.grip_off = full.max - p;
            }
            if gr.dragged() {
                if let Some(p) = ctx.pointer_latest_pos() {
                    let s = (p + self.grip_off) - full.min;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(s.x.max(200.0), s.y.max(160.0))));
                }
            }
        });

        // Menu popover (GNOME style)
        if self.menu_open {
            let mut act = 0;
            let resp = egui::Area::new(egui::Id::new("popover"))
                .fixed_pos(Pos2::new(8.0, 34.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none().rounding(Rounding::same(8.0)).fill(Color32::from_rgb((paper.r() as f32 * 0.9) as u8, (paper.g() as f32 * 0.9) as u8, (paper.b() as f32 * 0.9) as u8)).stroke(Stroke::new(1.0, ink.gamma_multiply(0.35))).inner_margin(4.0).show(ui, |ui| {
                        ui.set_min_width(130.0);
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::from_black_alpha(30);
                        for (i, label) in ["New file", "Load file", "Save as"].iter().enumerate() {
                            let b = egui::Button::new(egui::RichText::new(*label).color(ink))
                                .frame(false)
                                .min_size(Vec2::new(130.0, 26.0));
                            if ui.add(b).clicked() {
                                act = i + 1;
                            }
                        }
                    });
                });
            let clicked_out = ctx.input(|i| i.pointer.any_pressed())
                && !resp.response.hovered()
                && ctx.pointer_interact_pos().map_or(false, |p| p.x > 40.0 || p.y > 40.0);
            if act != 0 || clicked_out || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.menu_open = false;
            }
            match act {
                1 => self.new_note(),
                2 => self.load(),
                3 => self.save_as(),
                _ => {}
            }
        }

        // Fallback when no dialog tool exists (Linux without zenity/kdialog)
        if let Some((save, mut txt)) = self.prompt.take() {
            let mut keep = true;
            egui::Area::new(egui::Id::new("prompt")).anchor(egui::Align2::CENTER_BOTTOM, [0.0, -22.0]).show(ctx, |ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut txt).hint_text(if save { "save to path, Enter" } else { "open path, Enter" }).desired_width(240.0).text_color(ink));
                r.request_focus();
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    keep = false;
                } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    keep = false;
                    let p = PathBuf::from(txt.trim());
                    if save {
                        if std::fs::write(&p, &self.text).is_ok() {
                            self.path = Some(p);
                        }
                    } else {
                        self.open(p);
                    }
                }
            });
            if keep {
                self.prompt = Some((save, txt));
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 320.0])
            .with_min_inner_size([200.0, 160.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_title("StickyMD"),
        ..Default::default()
    };
    eframe::run_native("StickyMD", opts, Box::new(|cc| Box::new(App::new(cc))))
}
