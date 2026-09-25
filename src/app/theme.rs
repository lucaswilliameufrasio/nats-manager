use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, TextStyle, vec2};

pub(super) const CANVAS: Color32 = Color32::from_rgb(11, 18, 29);
pub(super) const SIDEBAR: Color32 = Color32::from_rgb(15, 25, 39);
pub(super) const SURFACE: Color32 = Color32::from_rgb(20, 33, 50);
pub(super) const SURFACE_RAISED: Color32 = Color32::from_rgb(27, 43, 62);
pub(super) const BORDER: Color32 = Color32::from_rgb(43, 61, 81);
pub(super) const TEXT: Color32 = Color32::from_rgb(232, 240, 247);
pub(super) const MUTED: Color32 = Color32::from_rgb(148, 166, 185);
pub(super) const ACCENT: Color32 = Color32::from_rgb(65, 205, 198);
pub(super) const ACCENT_DARK: Color32 = Color32::from_rgb(19, 57, 65);
pub(super) const SUCCESS: Color32 = Color32::from_rgb(95, 214, 169);
pub(super) const WARNING: Color32 = Color32::from_rgb(242, 190, 103);
pub(super) const DANGER: Color32 = Color32::from_rgb(242, 116, 124);
pub(super) const DANGER_DARK: Color32 = Color32::from_rgb(65, 31, 42);

pub(super) fn apply(context: &egui::Context) {
    context.set_theme(egui::Theme::Dark);
    context.style_mut_of(egui::Theme::Dark, |style| {
        let mut visuals = egui::Visuals::dark();
        let radius = egui::CornerRadius::same(9);

        visuals.dark_mode = true;
        visuals.panel_fill = CANVAS;
        visuals.window_fill = SURFACE;
        visuals.window_stroke = Stroke::new(1.0, BORDER);
        visuals.window_corner_radius = egui::CornerRadius::same(14);
        visuals.extreme_bg_color = Color32::from_rgb(9, 15, 24);
        visuals.faint_bg_color = SURFACE_RAISED;
        visuals.code_bg_color = Color32::from_rgb(10, 18, 29);
        visuals.override_text_color = Some(TEXT);
        visuals.weak_text_color = Some(MUTED);
        visuals.hyperlink_color = ACCENT;
        visuals.warn_fg_color = WARNING;
        visuals.error_fg_color = DANGER;
        visuals.selection.bg_fill = ACCENT_DARK;
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        visuals.button_frame = true;
        visuals.collapsing_header_frame = false;

        visuals.widgets.noninteractive.bg_fill = CANVAS;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        visuals.widgets.inactive.bg_fill = SURFACE_RAISED;
        visuals.widgets.inactive.weak_bg_fill = SURFACE_RAISED;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(37, 59, 79);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(37, 59, 79);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.active.bg_fill = ACCENT_DARK;
        visuals.widgets.active.weak_bg_fill = ACCENT_DARK;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.open.bg_fill = SURFACE_RAISED;
        visuals.widgets.open.weak_bg_fill = SURFACE_RAISED;
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, BORDER);

        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = radius;
        }

        style.visuals = visuals;
        style.spacing.item_spacing = vec2(10.0, 10.0);
        style.spacing.button_padding = vec2(14.0, 9.0);
        style.spacing.interact_size = vec2(42.0, 40.0);
        style.spacing.window_margin = egui::Margin::same(18);
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(26.0, FontFamily::Proportional),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        );
    });
}
