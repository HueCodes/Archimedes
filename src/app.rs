use eframe::egui::{self, Align, Layout, RichText, Vec2};

use crate::demos::convex_hull::ConvexHullDemo;
use crate::demos::critical_area::CriticalAreaDemo;
use crate::demos::delaunay_voronoi::DelaunayVoronoiDemo;
use crate::demos::polygon_ops::{EditMode, PolygonOpsDemo, Preset};
use crate::demos::robustness::RobustnessDemo;
use crate::theme;
use i_overlay::core::overlay_rule::OverlayRule;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    ConvexHull,
    DelaunayVoronoi,
    PolygonOps,
    CriticalArea,
    Robustness,
}

impl Tab {
    fn title(&self) -> &'static str {
        match self {
            Tab::ConvexHull => "Convex Hull",
            Tab::DelaunayVoronoi => "Delaunay / Voronoi",
            Tab::PolygonOps => "Polygon Ops",
            Tab::CriticalArea => "Critical Area",
            Tab::Robustness => "Robustness",
        }
    }

    fn status(&self) -> TabStatus {
        TabStatus::Live
    }
}

#[derive(Clone, Copy)]
enum TabStatus {
    Live,
}

pub struct App {
    tab: Tab,
    convex_hull: ConvexHullDemo,
    voronoi: DelaunayVoronoiDemo,
    polygon_ops: PolygonOpsDemo,
    critical_area: CriticalAreaDemo,
    robustness: RobustnessDemo,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);
        Self {
            tab: Tab::ConvexHull,
            convex_hull: ConvexHullDemo::default(),
            voronoi: DelaunayVoronoiDemo::default(),
            polygon_ops: PolygonOpsDemo::default(),
            critical_area: CriticalAreaDemo::default(),
            robustness: RobustnessDemo::default(),
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
                self.tab = Tab::CriticalArea;
            }
            if i.key_pressed(egui::Key::Num5) {
                self.tab = Tab::Robustness;
            }
            if i.key_pressed(egui::Key::C) {
                match self.tab {
                    Tab::ConvexHull => self.convex_hull.clear(),
                    Tab::DelaunayVoronoi => self.voronoi.clear(),
                    Tab::PolygonOps => self.polygon_ops.clear(),
                    Tab::CriticalArea => self.critical_area.reset(),
                    Tab::Robustness => self.robustness.reset(),
                }
            }
            if i.key_pressed(egui::Key::R) {
                match self.tab {
                    Tab::ConvexHull => self.convex_hull.random_into_last_rect(100),
                    Tab::DelaunayVoronoi => self.voronoi.random_into_last_rect(100),
                    _ => {}
                }
            }
            if i.key_pressed(egui::Key::Space) && self.tab == Tab::ConvexHull {
                self.convex_hull.toggle_play();
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        top_bar(ctx);
        bottom_bar(
            ctx,
            &self.convex_hull,
            &self.voronoi,
            &self.polygon_ops,
            &self.critical_area,
            &self.robustness,
            self.tab,
        );
        left_panel(
            ctx,
            &mut self.tab,
            &mut self.convex_hull,
            &mut self.voronoi,
            &mut self.polygon_ops,
            &mut self.critical_area,
            &mut self.robustness,
        );
        right_panel(
            ctx,
            self.tab,
            &mut self.convex_hull,
            &mut self.voronoi,
            &mut self.polygon_ops,
            &mut self.critical_area,
            &mut self.robustness,
        );

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG)
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| match self.tab {
                Tab::ConvexHull => self.convex_hull.ui(ui),
                Tab::DelaunayVoronoi => self.voronoi.ui(ui),
                Tab::PolygonOps => self.polygon_ops.ui(ui),
                Tab::CriticalArea => self.critical_area.ui(ui),
                Tab::Robustness => self.robustness.ui(ui),
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
                    RichText::new("Archimedes")
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

fn bottom_bar(
    ctx: &egui::Context,
    hull: &ConvexHullDemo,
    voronoi: &DelaunayVoronoiDemo,
    polygons: &PolygonOpsDemo,
    critical: &CriticalAreaDemo,
    robustness: &RobustnessDemo,
    tab: Tab,
) {
    egui::TopBottomPanel::bottom("status")
        .exact_height(26.0)
        .frame(
            egui::Frame::none()
                .fill(theme::PANEL)
                .inner_margin(egui::Margin::symmetric(14.0, 4.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let (left, seed) = match tab {
                    Tab::ConvexHull => {
                        let (n, hull_n, tests, ms) = hull.metrics();
                        (
                            format!(
                                "{n} points · {hull_n} on hull · {tests} orient tests · {ms:.2} ms"
                            ),
                            hull.seed(),
                        )
                    }
                    Tab::DelaunayVoronoi => {
                        let (n, tri, ms) = voronoi.metrics();
                        (
                            format!("{n} sites · {tri} triangles · build {ms:.2} ms"),
                            voronoi.seed(),
                        )
                    }
                    Tab::PolygonOps => {
                        let (na, nb, vcount, area, ms) = polygons.metrics();
                        (
                            format!(
                                "A {na} · B {nb} · result {vcount} verts · area {area:.0} · {ms:.2} ms"
                            ),
                            0,
                        )
                    }
                    Tab::CriticalArea => {
                        let (r, area, ms) = critical.metrics();
                        (
                            format!(
                                "defect radius r = {r:.1} px · critical area {area:.0} · {ms:.2} ms"
                            ),
                            0,
                        )
                    }
                    Tab::Robustness => {
                        let r = robustness.readout();
                        let flag = if r.agree { "agree" } else { "DISAGREE" };
                        (
                            format!(
                                "naive {:+.4e} · robust {:+.4e} · signs {flag} · total disagreements {}",
                                r.naive,
                                r.robust,
                                robustness.disagreements()
                            ),
                            0,
                        )
                    }
                };
                let text = if seed != 0 {
                    format!("{left} · seed 0x{:08X}", seed as u32)
                } else {
                    left
                };
                ui.label(
                    RichText::new(text)
                        .monospace()
                        .size(11.0)
                        .color(theme::FG_DIM),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (txt, color) = match (tab, hull.anim_progress()) {
                        (Tab::ConvexHull, Some((_, _, true))) => ("animating", theme::ACCENT),
                        _ => ("ready", theme::OK),
                    };
                    ui.label(
                        RichText::new(txt)
                            .monospace()
                            .size(11.0)
                            .color(color),
                    );
                });
            });
        });
}

fn left_panel(
    ctx: &egui::Context,
    tab: &mut Tab,
    hull: &mut ConvexHullDemo,
    voronoi: &mut DelaunayVoronoiDemo,
    polygons: &mut PolygonOpsDemo,
    critical: &mut CriticalAreaDemo,
    robustness: &mut RobustnessDemo,
) {
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
                Tab::CriticalArea,
                Tab::Robustness,
            ] {
                tree_item(ui, tab, t);
            }

            ui.add_space(18.0);
            section_header(ui, "ACTIONS");
            let clear_label = match *tab {
                Tab::ConvexHull | Tab::DelaunayVoronoi => "Clear",
                Tab::PolygonOps | Tab::CriticalArea | Tab::Robustness => "Reset",
            };
            let random_label = match *tab {
                Tab::ConvexHull => Some("Random 100"),
                Tab::DelaunayVoronoi => Some("Random 100"),
                _ => None,
            };
            ui.horizontal(|ui| {
                if ui.button(clear_label).clicked() {
                    match *tab {
                        Tab::ConvexHull => hull.clear(),
                        Tab::DelaunayVoronoi => voronoi.clear(),
                        Tab::PolygonOps => polygons.clear(),
                        Tab::CriticalArea => critical.reset(),
                        Tab::Robustness => robustness.reset(),
                    }
                }
                if let Some(label) = random_label {
                    if ui.button(label).clicked() {
                        match *tab {
                            Tab::ConvexHull => hull.random_into_last_rect(100),
                            Tab::DelaunayVoronoi => voronoi.random_into_last_rect(100),
                            _ => {}
                        }
                    }
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
            shortcut_line(ui, "1-5", "switch demo");
            shortcut_line(ui, "Space", "play · pause");
        });
}

fn tree_item(ui: &mut egui::Ui, current: &mut Tab, t: Tab) {
    let selected = *current == t;
    let (glyph, glyph_color) = match t.status() {
        TabStatus::Live => ("*", theme::OK),
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

fn right_panel(
    ctx: &egui::Context,
    tab: Tab,
    hull: &mut ConvexHullDemo,
    voronoi: &mut DelaunayVoronoiDemo,
    polygons: &mut PolygonOpsDemo,
    critical: &mut CriticalAreaDemo,
    robustness: &mut RobustnessDemo,
) {
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
                Tab::DelaunayVoronoi => voronoi_sidebar(ui, voronoi),
                Tab::PolygonOps => polygon_ops_sidebar(ui, polygons),
                Tab::CriticalArea => critical_area_sidebar(ui, critical),
                Tab::Robustness => robustness_sidebar(ui, robustness),
            }
        });
}

fn hull_sidebar(ui: &mut egui::Ui, hull: &mut ConvexHullDemo) {
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
    let nlogn = if n >= 2 {
        (n as f32) * (n as f32).log2()
    } else {
        0.0
    };
    metric_line(ui, "n log n", &format!("{nlogn:>6.0}"));
    metric_line(ui, "last frame", &format!("{ms:.2} ms"));

    ui.add_space(14.0);
    section_header(ui, "ANIMATION");
    let progress = hull.anim_progress();
    let (step, total, playing) = progress.unwrap_or((0, 0, false));
    let enabled = n >= 3;
    ui.horizontal(|ui| {
        let play_label = if playing { "Pause" } else { "Play" };
        if ui
            .add_enabled(enabled, egui::Button::new(play_label))
            .clicked()
        {
            hull.toggle_play();
        }
        if ui
            .add_enabled(enabled && total > 0, egui::Button::new("Reset"))
            .clicked()
        {
            hull.reset_anim();
        }
    });
    metric_line(
        ui,
        "step",
        &if total > 0 {
            format!("{step} / {total}")
        } else {
            "—".to_string()
        },
    );
    metric_line(ui, "interval", "120 ms");

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

fn voronoi_sidebar(ui: &mut egui::Ui, demo: &mut DelaunayVoronoiDemo) {
    section_header(ui, "ALGORITHM");
    ui.label(RichText::new("Bowyer-Watson (spade)").size(15.0).color(theme::FG));
    ui.label(
        RichText::new("O(n log n) expected")
            .monospace()
            .size(12.0)
            .color(theme::ACCENT),
    );

    ui.add_space(14.0);
    section_header(ui, "INVARIANT");
    ui.label(
        RichText::new(
            "Voronoi cell of site p is the set of points closest to p. Delaunay is its dual: two sites share a Voronoi edge iff they share a Delaunay edge.",
        )
        .size(12.5)
        .color(theme::FG.linear_multiply(0.85)),
    );

    ui.add_space(14.0);
    section_header(ui, "LIVE");
    let (n, tri, ms) = demo.metrics();
    metric_line(ui, "sites", &format!("{n}"));
    metric_line(ui, "triangles", &format!("{tri}"));
    metric_line(ui, "build", &format!("{ms:.2} ms"));

    ui.add_space(14.0);
    section_header(ui, "EULER");
    let euler = demo.euler();
    metric_line(ui, "V", &format!("{}", euler.v));
    metric_line(ui, "E", &format!("{}", euler.e));
    metric_line(ui, "F", &format!("{}", euler.f));
    let chi = euler.characteristic();
    let (label, color) = if euler.v == 0 {
        ("—", theme::FG_DIM)
    } else if chi == 2 {
        ("V − E + F = 2", theme::OK)
    } else {
        ("invariant broken", theme::WARN)
    };
    ui.label(
        RichText::new(label)
            .monospace()
            .size(11.0)
            .color(color),
    );

    ui.add_space(14.0);
    section_header(ui, "LAYERS");
    ui.checkbox(demo.show_voronoi_mut(), "Voronoi cells");
    ui.checkbox(demo.show_delaunay_mut(), "Delaunay edges");
    ui.checkbox(demo.show_circumcircle_mut(), "Circumcircles on hover");

    ui.add_space(14.0);
    section_header(ui, "FOCUS");
    if let Some(focus) = demo.focus() {
        metric_line(ui, "degree", &format!("{}", focus.degree));
        metric_line(ui, "cell area", &format!("{:.0} px²", focus.cell_area));
        metric_line(ui, "nearest nbr", &format!("{:.1} px", focus.nearest_dist));
        if focus.is_hull {
            ui.label(
                RichText::new("hull site · unbounded cell")
                    .monospace()
                    .size(11.0)
                    .color(theme::ORANGE),
            );
        }
    } else {
        ui.label(
            RichText::new("hover a site to inspect")
                .size(11.5)
                .color(theme::FG_DIM),
        );
    }

    ui.add_space(14.0);
    section_header(ui, "REFERENCES");
    ui.label(RichText::new("Bowyer (1981) · Watson (1981)").size(12.0).color(theme::FG_DIM));
    ui.label(RichText::new("de Berg et al., §7 / §9").size(12.0).color(theme::FG_DIM));
}

fn polygon_ops_sidebar(ui: &mut egui::Ui, demo: &mut PolygonOpsDemo) {
    section_header(ui, "ALGORITHM");
    ui.label(RichText::new("Overlay (i_overlay)").size(15.0).color(theme::FG));
    ui.label(
        RichText::new("O((n+k) log n)")
            .monospace()
            .size(12.0)
            .color(theme::ACCENT),
    );

    ui.add_space(14.0);
    section_header(ui, "INVARIANT");
    ui.label(
        RichText::new(
            "Robust union / intersection / difference / xor on simple polygons, handling shared edges, coincident vertices, and degenerate touchings.",
        )
        .size(12.5)
        .color(theme::FG.linear_multiply(0.85)),
    );

    ui.add_space(14.0);
    section_header(ui, "OPERATION");
    let op = *demo.op_mut();
    for (rule, label) in [
        (OverlayRule::Union, "A ∪ B"),
        (OverlayRule::Intersect, "A ∩ B"),
        (OverlayRule::Difference, "A \\ B"),
        (OverlayRule::InverseDifference, "B \\ A"),
        (OverlayRule::Xor, "A ⊕ B"),
    ] {
        let selected = op == rule;
        if ui
            .selectable_label(selected, RichText::new(label).monospace().size(12.5))
            .clicked()
        {
            *demo.op_mut() = rule;
        }
    }

    ui.add_space(14.0);
    section_header(ui, "EDIT MODE");
    for (mode, label) in [
        (EditMode::DragOnly, "Drag only"),
        (EditMode::EditA, "Edit A (click to add)"),
        (EditMode::EditB, "Edit B (click to add)"),
    ] {
        let selected = *demo.mode_mut() == mode;
        if ui.selectable_label(selected, label).clicked() {
            *demo.mode_mut() = mode;
        }
    }

    ui.add_space(14.0);
    section_header(ui, "PRESETS");
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("A:")
                .monospace()
                .size(11.0)
                .color(theme::FG_DIM),
        );
        for (label, p) in [
            ("pentagon", Preset::Pentagon),
            ("star", Preset::Star),
            ("L", Preset::LShape),
            ("rect", Preset::Rectangle),
        ] {
            if ui.small_button(label).clicked() {
                demo.preset_a(p);
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("B:")
                .monospace()
                .size(11.0)
                .color(theme::FG_DIM),
        );
        for (label, p) in [
            ("pentagon", Preset::Pentagon),
            ("star", Preset::Star),
            ("L", Preset::LShape),
            ("rect", Preset::Rectangle),
        ] {
            if ui.small_button(label).clicked() {
                demo.preset_b(p);
            }
        }
    });

    ui.add_space(14.0);
    section_header(ui, "LIVE");
    let (na, nb, vcount, area, ms) = demo.metrics();
    metric_line(ui, "A vertices", &format!("{na}"));
    metric_line(ui, "B vertices", &format!("{nb}"));
    metric_line(ui, "result", &format!("{vcount} verts"));
    metric_line(ui, "area", &format!("{area:.0}"));
    metric_line(ui, "build", &format!("{ms:.2} ms"));

    ui.add_space(14.0);
    section_header(ui, "REFERENCES");
    ui.label(RichText::new("Vatti (1992) · Greiner-Hormann (1998)").size(12.0).color(theme::FG_DIM));
    ui.label(RichText::new("i_overlay crate").size(12.0).color(theme::FG_DIM));
}

fn critical_area_sidebar(ui: &mut egui::Ui, demo: &mut CriticalAreaDemo) {
    section_header(ui, "ALGORITHM");
    ui.label(RichText::new("Critical-area via Minkowski dilation").size(15.0).color(theme::FG));
    ui.label(
        RichText::new("dilate(A, r/2) ∩ dilate(B, r/2)")
            .monospace()
            .size(12.0)
            .color(theme::ACCENT),
    );

    ui.add_space(14.0);
    section_header(ui, "INVARIANT");
    ui.label(
        RichText::new(
            "A disk of radius r centered at point p bridges features A and B iff p lies in dilate(A, r/2) ∩ dilate(B, r/2).",
        )
        .size(12.5)
        .color(theme::FG.linear_multiply(0.85)),
    );

    ui.add_space(14.0);
    section_header(ui, "DEFECT RADIUS");
    let r = *demo.radius_mut();
    ui.add(egui::Slider::new(demo.radius_mut(), 0.0..=80.0).text("r (px)"));
    let _ = r;

    ui.add_space(14.0);
    section_header(ui, "LIVE");
    let (r, area, ms) = demo.metrics();
    metric_line(ui, "r", &format!("{r:.1} px"));
    metric_line(ui, "critical area", &format!("{area:.0} px²"));
    metric_line(ui, "build", &format!("{ms:.2} ms"));

    ui.add_space(14.0);
    section_header(ui, "WHY IT MATTERS");
    ui.label(
        RichText::new(
            "Yield modeling on a semiconductor mask: the critical area is the integral over defect radii of the region where a defect would short neighboring features. Minimum spacing shrinks \u{2192} critical area explodes.",
        )
        .size(12.0)
        .color(theme::FG_DIM),
    );

    ui.add_space(14.0);
    section_header(ui, "REFERENCES");
    ui.label(RichText::new("Stapper (1983) · VLSI yield").size(12.0).color(theme::FG_DIM));
    ui.label(RichText::new("Papadopoulou & Lee (1999)").size(12.0).color(theme::FG_DIM));
}

fn robustness_sidebar(ui: &mut egui::Ui, demo: &mut RobustnessDemo) {
    section_header(ui, "ALGORITHM");
    ui.label(RichText::new("orient2d · naive f32 vs Shewchuk adaptive").size(15.0).color(theme::FG));
    ui.label(
        RichText::new("robust: ~5x avg, exact sign")
            .monospace()
            .size(12.0)
            .color(theme::ACCENT),
    );

    ui.add_space(14.0);
    section_header(ui, "INVARIANT");
    ui.label(
        RichText::new(
            "Near-collinear inputs make (b-a) × (c-a) subtract two almost-equal terms. Under f32 the sign is dominated by rounding — downstream hull turns, Delaunay flips, and boolean ops branch on this sign and silently produce inconsistent output. Shewchuk's adaptive predicates fall back to extended precision only when the error interval crosses zero.",
        )
        .size(12.5)
        .color(theme::FG.linear_multiply(0.85)),
    );

    ui.add_space(14.0);
    section_header(ui, "READOUT");
    let r = demo.readout();
    metric_line(ui, "naive (f32)", &format!("{:+.4e}", r.naive));
    metric_line(ui, "robust (f64)", &format!("{:+.4e}", r.robust));
    let sign_label = |s: i8| match s {
        1 => "LEFT",
        -1 => "RIGHT",
        _ => "ZERO",
    };
    metric_line(ui, "naive sign", sign_label(r.sign_naive));
    metric_line(ui, "robust sign", sign_label(r.sign_robust));
    let (label, color) = if r.agree {
        ("AGREE", theme::OK)
    } else {
        ("DISAGREE", theme::WARN)
    };
    ui.label(
        RichText::new(label)
            .monospace()
            .size(13.0)
            .color(color),
    );
    metric_line(
        ui,
        "disagreements",
        &format!("{}", demo.disagreements()),
    );

    ui.add_space(14.0);
    section_header(ui, "VIEW");
    ui.checkbox(demo.show_diff_field_mut(), "Shade disagreement field");
    if ui.button("Reload degenerate preset").clicked() {
        demo.preset_nearly_collinear();
    }

    ui.add_space(14.0);
    section_header(ui, "WHY IT MATTERS");
    ui.label(
        RichText::new(
            "Mask layout and alignment math branch on orientation tests at every edge. A silent sign flip in a CAD tool produces a subtly broken polygon no-one notices until the wafer is ruined.",
        )
        .size(12.0)
        .color(theme::FG_DIM),
    );

    ui.add_space(14.0);
    section_header(ui, "REFERENCES");
    ui.label(
        RichText::new("Shewchuk (1997) Adaptive Precision FP Predicates")
            .size(12.0)
            .color(theme::FG_DIM),
    );
    ui.label(
        RichText::new("Yap · Exact Geometric Computation paradigm")
            .size(12.0)
            .color(theme::FG_DIM),
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

struct Color32Ext;
impl Color32Ext {
    const TRANSPARENT: egui::Color32 = egui::Color32::TRANSPARENT;
}
