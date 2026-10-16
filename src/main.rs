#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, text::LayoutJob, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, TextFormat, Vec2};
use std::path::PathBuf;

// Familiar post-it colours: (paper, ink)
const NOTES: [(u32, u32); 6] = [
    (0xFFF176, 0x4A4210), // canary yellow
    (0xFF9EC4, 0x4F1A33), // pink
    (0x9EE7F5, 0x103F4A), // aqua blue
    (0xC5F28B, 0x2A4410), // lime green
    (0xFFC27A, 0x4F2E0A), // orange
    (0xD7B8FF, 0x35195A), // lavender
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
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        let mut s = egui::Style::default();
        s.visuals = egui::Visuals::light();
        s.visuals.selection.bg_fill = Color32::from_black_alpha(45);
        s.visuals.selection.stroke = Stroke::NONE;
        cc.egui_ctx.set_style(s);
        let mut app = App { text: String::new(), path: None, color: seed() % NOTES.len(), menu_open: false };
        if let Some(p) = std::env::args().nth(1) {
            app.open(PathBuf::from(p));
        }
        app
    }

    fn open(&mut self, p: PathBuf) {
        if let Ok(t) = std::fs::read_to_string(&p) {
            self.text = t;
            self.path = Some(p);
        }
    }

    fn new_note(&mut self) {
        self.text.clear();
        self.path = None;
        self.color = (self.color + 1) % NOTES.len();
    }

    fn save_as(&mut self) {
        let d = rfd::FileDialog::new().add_filter("Markdown", &["md"]).set_file_name("note.md");
        if let Some(mut p) = d.save_file() {
            if p.extension().is_none() {
                p.set_extension("md");
            }
            if std::fs::write(&p, &self.text).is_ok() {
                self.path = Some(p);
            }
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
        if let Some(p) = rfd::FileDialog::new().add_filter("Markdown / text", &["md", "markdown", "txt"]).pick_file() {
            self.open(p);
        }
    }
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
            if cfg!(target_os = "macos") {
                // winit can't start a native resize on macOS, so resize manually
                if gr.dragged() {
                    let s = full.size() + gr.drag_delta();
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(s.x.max(200.0), s.y.max(160.0))));
                }
            } else if gr.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::viewport::ResizeDirection::SouthEast));
            }
        });

        // Menu popover (GNOME style)
        if self.menu_open {
            let mut act = 0;
            let resp = egui::Area::new(egui::Id::new("popover"))
                .fixed_pos(Pos2::new(8.0, 34.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).rounding(Rounding::same(10.0)).fill(Color32::from_rgb(0xFA, 0xFA, 0xFA)).show(ui, |ui| {
                        ui.set_min_width(130.0);
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::from_black_alpha(18);
                        for (i, label) in ["New file", "Load file", "Save as"].iter().enumerate() {
                            let b = egui::Button::new(egui::RichText::new(*label).color(Color32::from_rgb(0x24, 0x24, 0x24)))
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
