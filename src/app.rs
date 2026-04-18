use eframe::egui::{self, Align, Layout, RichText, Vec2};

use crate::demos::convex_hull::ConvexHullDemo;
use crate::theme;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    ConvexHull,
    DelaunayVoronoi,
    PolygonOps,
    Robustness,
}

impl Tab {
    fn title(&self) -> &'static str {
        match self {
            Tab::ConvexHull => "Convex Hull",
            Tab::DelaunayVoronoi => "Delaunay / Voronoi",
            Tab::PolygonOps => "Polygon Ops",
            Tab::Robustness => "Robustness",
        }
    }

    fn status(&self) -> TabStatus {
        match self {
            Tab::ConvexHull => TabStatus::Live,
            _ => TabStatus::Planned,
        }
    }
}

#[derive(Clone, Copy)]
enum TabStatus {
    Live,
    Planned,
}

pub struct App {
    tab: Tab,
    convex_hull: ConvexHullDemo,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);
        Self {
            tab: Tab::ConvexHull,
            convex_hull: ConvexHullDemo::default(),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Num1) {
                self.tab = Tab::ConvexHull;
            }
            if i.key_pressed(egui::Key::Num2) {
                self.tab = Tab::DelaunayVoronoi;
            }
            if i.key_pressed(egui::Key::Num3) {
                self.tab = Tab::PolygonOps;
            }
            if i.key_pressed(egui::Key::Num4) {
                self.tab = Tab::Robustness;
            }
            if i.key_pressed(egui::Key::C) {
                self.convex_hull.clear();
            }
            if i.key_pressed(egui::Key::R) {
                self.convex_hull.random_into_last_rect(100);
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        top_bar(ctx);
        bottom_bar(ctx, &self.convex_hull, self.tab);
        left_panel(ctx, &mut self.tab, &mut self.convex_hull);
        right_panel(ctx, self.tab, &self.convex_hull);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG)
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| match self.tab {
                Tab::ConvexHull => self.convex_hull.ui(ui),
                Tab::DelaunayVoronoi => placeholder(ui, "Delaunay + Voronoi", "planned · Saturday evening"),
                Tab::PolygonOps => placeholder(ui, "Polygon boolean ops", "planned · Sunday morning"),
                Tab::Robustness => placeholder(ui, "Robustness demo", "planned · Sunday afternoon"),
            });
    }
}

fn top_bar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("chrome")
        .exact_height(44.0)
        .frame(
            egui::Frame::none()
                .fill(theme::PANEL)
                .inner_margin(egui::Margin::symmetric(14.0, 10.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new("archimedes")
                        .size(17.0)
                        .color(theme::FG),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("computational geometry playground")
                        .size(12.0)
                        .color(theme::FG_DIM),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("v0.1")
                            .monospace()
                            .size(11.0)
                            .color(theme::FG_DIM),
                    );
                });
            });
        });
}

fn bottom_bar(ctx: &egui::Context, hull: &ConvexHullDemo, tab: Tab) {
    egui::TopBottomPanel::bottom("status")
        .exact_height(26.0)
        .frame(
            egui::Frame::none()
                .fill(theme::PANEL)
                .inner_margin(egui::Margin::symmetric(14.0, 4.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let (n, hull_n, tests, ms) = match tab {
                    Tab::ConvexHull => hull.metrics(),
                    _ => (0, 0, 0, 0.0),
                };
                let left = format!(
                    "{n} points · {h} on hull · {t} orient tests · {ms:.2} ms",
                    n = n,
                    h = hull_n,
                    t = tests,
                    ms = ms,
                );
                ui.label(
                    RichText::new(left)
                        .monospace()
                        .size(11.0)
                        .color(theme::FG_DIM),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("ready")
                            .monospace()
                            .size(11.0)
                            .color(theme::OK),
                    );
                });
            });
        });
}

fn left_panel(ctx: &egui::Context, tab: &mut Tab, hull: &mut ConvexHullDemo) {
    egui::SidePanel::left("tree")
        .exact_width(236.0)
        .resizable(false)
        .frame(
            egui::Frame::none()
                .fill(theme::PANEL)
                .inner_margin(egui::Margin::symmetric(12.0, 14.0)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 6.0);
            section_header(ui, "DEMOS");
            for t in [
                Tab::ConvexHull,
                Tab::DelaunayVoronoi,
                Tab::PolygonOps,
                Tab::Robustness,
            ] {
                tree_item(ui, tab, t);
            }

            ui.add_space(18.0);
            section_header(ui, "ACTIONS");
            ui.horizontal(|ui| {
                if ui.button("Clear").clicked() {
                    hull.clear();
                }
                if ui.button("Random 100").clicked() {
                    hull.random_into_last_rect(100);
                }
            });

            ui.add_space(18.0);
            section_header(ui, "STACK");
            stack_line(ui, "runtime", "wgpu · wasm32");
            stack_line(ui, "ui", "egui 0.30 / eframe");
            stack_line(ui, "geom", "spade · i_overlay · robust");

            ui.add_space(18.0);
            section_header(ui, "SHORTCUTS");
            shortcut_line(ui, "C", "clear");
            shortcut_line(ui, "R", "random");
            shortcut_line(ui, "1-4", "switch demo");
            shortcut_line(ui, "Space", "play · pause");
        });
}

fn tree_item(ui: &mut egui::Ui, current: &mut Tab, t: Tab) {
    let selected = *current == t;
    let (glyph, glyph_color) = match t.status() {
        TabStatus::Live => ("*", theme::OK),
        TabStatus::Planned => ("·", theme::FG_DIM),
    };
    let title_color = if selected { theme::FG } else { theme::FG_DIM };
    let bg = if selected {
        theme::BG
    } else {
        Color32Ext::TRANSPARENT
    };

    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 26.0),
        egui::Sense::click(),
    );
    ui.painter().rect_filled(rect, 4.0, bg);
    if selected {
        let bar = egui::Rect::from_min_size(rect.min, Vec2::new(2.5, rect.height()));
        ui.painter().rect_filled(bar, 1.0, theme::ACCENT);
    }
    let mut text_x = rect.min.x + 10.0;
    ui.painter().text(
        egui::pos2(text_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        glyph,
        egui::FontId::monospace(13.0),
        glyph_color,
    );
    text_x += 16.0;
    ui.painter().text(
        egui::pos2(text_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        t.title(),
        egui::FontId::proportional(13.0),
        title_color,
    );

    if resp.clicked() {
        *current = t;
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

fn right_panel(ctx: &egui::Context, tab: Tab, hull: &ConvexHullDemo) {
    egui::SidePanel::right("properties")
        .exact_width(308.0)
        .resizable(false)
        .frame(
            egui::Frame::none()
                .fill(theme::PANEL)
                .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 6.0);
            match tab {
                Tab::ConvexHull => hull_sidebar(ui, hull),
                Tab::DelaunayVoronoi => planned_sidebar(
                    ui,
                    "Bowyer-Watson (spade)",
                    "O(n log n) expected",
                    "Delaunay triangulation maximizes the minimum angle; its dual is the Voronoi diagram.",
                ),
                Tab::PolygonOps => planned_sidebar(
                    ui,
                    "Vatti overlay (i_overlay)",
                    "O((n+k) log n)",
                    "Robust polygon union / intersection / difference / xor, handling shared edges and degeneracies.",
                ),
                Tab::Robustness => planned_sidebar(
                    ui,
                    "Shewchuk adaptive orient2d",
                    "~5x avg over naive",
                    "Floating-point orientation tests flip sign near collinearity. Adaptive predicates fall back to exact arithmetic only when the error interval crosses zero.",
                ),
            }
        });
}

fn hull_sidebar(ui: &mut egui::Ui, hull: &ConvexHullDemo) {
    section_header(ui, "ALGORITHM");
    ui.label(RichText::new("Andrew's monotone chain").size(15.0).color(theme::FG));
    ui.label(
        RichText::new("O(n log n)")
            .monospace()
            .size(12.0)
            .color(theme::ACCENT),
    );

    ui.add_space(14.0);
    section_header(ui, "INVARIANT");
    ui.label(
        RichText::new(
            "After each step the stack holds a counter-clockwise lower hull of all points processed so far.",
        )
        .size(12.5)
        .color(theme::FG.linear_multiply(0.85)),
    );

    ui.add_space(14.0);
    section_header(ui, "LIVE");
    let (n, hull_n, tests, ms) = hull.metrics();
    metric_line(ui, "points", &format!("{n}"));
    metric_line(ui, "on hull", &format!("{hull_n}"));
    metric_line(ui, "orient tests", &format!("{tests}"));
    metric_line(ui, "last frame", &format!("{ms:.2} ms"));

    ui.add_space(14.0);
    section_header(ui, "REFERENCES");
    ui.label(
        RichText::new("Andrew (1979)")
            .size(12.0)
            .color(theme::FG_DIM),
    );
    ui.label(
        RichText::new("de Berg et al., §1.1")
            .size(12.0)
            .color(theme::FG_DIM),
    );
}

fn planned_sidebar(ui: &mut egui::Ui, name: &str, complexity: &str, invariant: &str) {
    section_header(ui, "ALGORITHM");
    ui.label(RichText::new(name).size(15.0).color(theme::FG));
    ui.label(
        RichText::new(complexity)
            .monospace()
            .size(12.0)
            .color(theme::ACCENT),
    );

    ui.add_space(14.0);
    section_header(ui, "INVARIANT");
    ui.label(
        RichText::new(invariant)
            .size(12.5)
            .color(theme::FG.linear_multiply(0.85)),
    );

    ui.add_space(14.0);
    section_header(ui, "STATUS");
    ui.label(
        RichText::new("planned — stub in place")
            .monospace()
            .size(12.0)
            .color(theme::ORANGE),
    );
}

fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .monospace()
            .size(10.5)
            .color(theme::FG_DIM),
    );
}

fn metric_line(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .monospace()
                .size(11.5)
                .color(theme::FG_DIM),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .monospace()
                    .size(12.5)
                    .color(theme::FG),
            );
        });
    });
}

fn stack_line(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .monospace()
                .size(11.0)
                .color(theme::FG_DIM),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .monospace()
                    .size(11.0)
                    .color(theme::FG.linear_multiply(0.8)),
            );
        });
    });
}

fn shortcut_line(ui: &mut egui::Ui, key: &str, action: &str) {
    ui.horizontal(|ui| {
        let key_text = RichText::new(key)
            .monospace()
            .size(11.0)
            .color(theme::FG)
            .background_color(theme::BG);
        ui.label(key_text);
        ui.label(
            RichText::new(action)
                .size(11.5)
                .color(theme::FG_DIM),
        );
    });
}

fn placeholder(ui: &mut egui::Ui, title: &str, sub: &str) {
    let available = ui.available_size();
    let (_, painter) = ui.allocate_painter(available, egui::Sense::hover());
    let rect = painter.clip_rect();
    crate::canvas::paint_grid(&painter, rect);
    crate::canvas::paint_empty_state(&painter, rect, title, sub);
}

struct Color32Ext;
impl Color32Ext {
    const TRANSPARENT: egui::Color32 = egui::Color32::TRANSPARENT;
}
