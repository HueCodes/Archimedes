use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Stroke, Vec2};

pub const BG: Color32 = Color32::from_rgb(0x1a, 0x1b, 0x26);
pub const PANEL: Color32 = Color32::from_rgb(0x24, 0x28, 0x3b);
pub const FG: Color32 = Color32::from_rgb(0xc0, 0xca, 0xf5);
pub const FG_DIM: Color32 = Color32::from_rgb(0x56, 0x5f, 0x89);
pub const ACCENT: Color32 = Color32::from_rgb(0x7a, 0xa2, 0xf7);
pub const WARN: Color32 = Color32::from_rgb(0xf7, 0x76, 0x8e);
pub const OK: Color32 = Color32::from_rgb(0x9e, 0xce, 0x6a);
pub const VIOLET: Color32 = Color32::from_rgb(0xbb, 0x9a, 0xf7);
pub const ORANGE: Color32 = Color32::from_rgb(0xe0, 0xaf, 0x68);

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "inter".into(),
        FontData::from_static(include_bytes!("../assets/Inter-Medium.otf")).into(),
    );
    fonts.font_data.insert(
        "jbmono".into(),
        FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf")).into(),
    );

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "inter".into());
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "jbmono".into());

    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = BG;
    visuals.override_text_color = Some(FG);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, FG_DIM);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, FG);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, FG);
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.3);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(10.0, 4.0);
        style.spacing.window_margin = egui::Margin::same(12.0);
    });
}
