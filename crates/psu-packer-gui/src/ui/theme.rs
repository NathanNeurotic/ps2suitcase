use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, RichText, Style,
    TextStyle, Vec2,
};

pub const DISPLAY_FONT_NAME: &str = "ps2_display";

#[derive(Clone)]
pub struct Palette {
    pub background: Color32,
    pub panel: Color32,
    pub input_background: Color32,
    pub header_top: Color32,
    pub header_bottom: Color32,
    pub footer_top: Color32,
    pub footer_bottom: Color32,
    pub neon_accent: Color32,
    pub soft_accent: Color32,
    pub separator: Color32,
    pub text_primary: Color32,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: Color32::from_rgb(2, 0, 14),
            panel: Color32::from_rgb(1, 0, 7),
            input_background: Color32::from_rgb(28, 28, 48),
            header_top: Color32::from_rgb(2, 0, 14),
            header_bottom: Color32::from_rgb(10, 0, 54),
            footer_top: Color32::from_rgb(2, 0, 14),
            footer_bottom: Color32::from_rgb(10, 0, 54),
            neon_accent: Color32::from_rgb(110, 0, 255),
            soft_accent: Color32::from_rgb(72, 64, 128),
            separator: Color32::from_rgb(110, 0, 255),
            text_primary: Color32::from_rgb(214, 220, 240),
        }
    }
}

pub fn install(ctx: &egui::Context, palette: &Palette) {
    install_fonts(ctx);
    apply_visuals(ctx, palette);
    ctx.style_mut(|style| {
        apply_text_styles(style);
        apply_spacing(style);
    });
}

pub fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY_FONT_NAME.into()))
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        DISPLAY_FONT_NAME.to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/Orbitron-Regular.ttf")).into(),
    );

    fonts
        .families
        .entry(FontFamily::Name(DISPLAY_FONT_NAME.into()))
        .or_default()
        .insert(0, DISPLAY_FONT_NAME.to_owned());

    ctx.set_fonts(fonts);
}

pub fn display_heading_text(ui: &egui::Ui, text: impl Into<String>) -> RichText {
    let size = ui.style().text_styles[&TextStyle::Heading].size;
    RichText::new(text).font(display_font(size))
}

fn apply_visuals(ctx: &egui::Context, palette: &Palette) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(palette.text_primary);
    visuals.widgets.noninteractive.bg_fill = palette.input_background;
    visuals.widgets.noninteractive.fg_stroke.color = palette.text_primary;
    visuals.widgets.inactive.bg_fill = palette.input_background;
    visuals.widgets.inactive.fg_stroke.color = palette.text_primary;
    visuals.widgets.hovered.bg_fill = palette.soft_accent;
    visuals.widgets.active.bg_fill = palette.neon_accent.gamma_multiply(0.7);
    visuals.widgets.open.bg_fill = palette.input_background;
    visuals.extreme_bg_color = palette.background;
    visuals.faint_bg_color = palette.background;
    visuals.panel_fill = palette.panel;
    visuals.window_fill = palette.panel;
    visuals.window_stroke.color = palette.neon_accent;
    visuals.window_shadow.color = palette.neon_accent;
    visuals.popup_shadow.color = palette.neon_accent;

    ctx.set_visuals(visuals);
}

fn apply_spacing(style: &mut Style) {
    style.spacing.item_spacing = Vec2::new(12.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.window_margin = Margin::same(14);
    style.spacing.menu_margin = Margin::same(10);
    style.spacing.indent = 20.0;
}

fn apply_text_styles(style: &mut Style) {
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(28.0));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(18.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(18.0));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(15.0));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::monospace(16.0));
}

pub fn draw_vertical_gradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    top: Color32,
    bottom: Color32,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }

    let mut mesh = egui::epaint::Mesh::default();

    let top_left = rect.left_top();
    let top_right = rect.right_top();
    let bottom_left = rect.left_bottom();
    let bottom_right = rect.right_bottom();

    let base_index = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex {
        pos: top_left,
        uv: egui::pos2(0.0, 0.0),
        color: top,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: top_right,
        uv: egui::pos2(1.0, 0.0),
        color: top,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: bottom_left,
        uv: egui::pos2(0.0, 1.0),
        color: bottom,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: bottom_right,
        uv: egui::pos2(1.0, 1.0),
        color: bottom,
    });

    mesh.indices.extend_from_slice(&[
        base_index,
        base_index + 1,
        base_index + 2,
        base_index + 2,
        base_index + 1,
        base_index + 3,
    ]);

    painter.add(egui::Shape::mesh(mesh));
}

pub fn draw_separator(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
    painter.rect_filled(rect, 0.0, color);
}
