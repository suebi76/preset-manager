//! Farben, Schrift und Abstände (`Planung/07-Oberflaeche.md`, „Gestaltung“).
//!
//! Alle Farben stehen hier und nur hier. Die übrigen Module verwenden die
//! Konstanten, nie eigene Farbwerte – so bleibt das Aussehen einheitlich und
//! ein helles Thema lässt sich später an einer Stelle ergänzen.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
    Theme, ThemePreference,
};

pub const BG: Color32 = Color32::from_rgb(0x1d, 0x1d, 0x1d);
pub const PANEL: Color32 = Color32::from_rgb(0x25, 0x25, 0x25);
pub const RAISED: Color32 = Color32::from_rgb(0x32, 0x32, 0x32);
pub const HOVER: Color32 = Color32::from_rgb(0x2e, 0x2e, 0x2e);
pub const LINE: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xe8);
pub const MUTED: Color32 = Color32::from_rgb(0x9a, 0x9a, 0x9a);
pub const FAINT: Color32 = Color32::from_rgb(0x6f, 0x6f, 0x6f);
pub const ACCENT: Color32 = Color32::from_rgb(0x26, 0x80, 0xeb);
/// Akzent mit 25 % Deckkraft für ausgewählte Zeilen.
pub const SELECTION: Color32 = Color32::from_rgba_premultiplied(0x0a, 0x20, 0x3b, 0x40);
pub const PRESET: Color32 = ACCENT;
pub const PROFILE: Color32 = Color32::from_rgb(0x92, 0x56, 0xd9);
pub const WARN: Color32 = Color32::from_rgb(0xe6, 0x86, 0x19);
pub const WARN_TEXT: Color32 = Color32::from_rgb(0xf2, 0xb4, 0x6c);
pub const WARN_BG: Color32 = Color32::from_rgb(0x3a, 0x2c, 0x1b);
pub const ERROR: Color32 = Color32::from_rgb(0xe3, 0x48, 0x50);
pub const OK: Color32 = Color32::from_rgb(0x2d, 0x9d, 0x78);
pub const OK_TEXT: Color32 = Color32::from_rgb(0x7f, 0xd6, 0xb6);

/// Höhe einer Listenzeile in Punkten.
pub const ROW_HEIGHT: f32 = 28.0;
/// Größe der Symbole in Listen.
pub const ICON_SIZE: f32 = 15.0;

/// Richtet Schrift, Farben und Abstände ein. Einmal beim Start aufrufen.
pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_visuals_of(Theme::Dark, visuals());
    ctx.set_theme(ThemePreference::Dark);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        // Bildlaufleisten immer sichtbar (nicht erst beim Darüberfahren) und
        // der Inhalt am Rand leicht ausgeblendet, wo es weitergeht – so sieht
        // man jedem Bereich an, dass er rollt.
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 8.0;
        scroll.handle_min_length = 24.0;
        scroll.bar_inner_margin = 3.0;
        scroll.fade = egui::style::ScrollFadeStyle {
            strength: 0.7,
            size: 28.0,
        };
        style.spacing.scroll = scroll;
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(16.0)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(13.0)),
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Monospace, FontId::monospace(11.5)),
        ]
        .into();
    });
}

/// Windows-Systemschrift Segoe UI Variable, ersatzweise Segoe UI; dahinter
/// die Phosphor-Symbole und egui's eigene Schriften als Rückfall.
fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    if let Some(bytes) = system_font() {
        fonts
            .font_data
            .insert("segoe".to_owned(), FontData::from_owned(bytes).into());
        if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
            family.insert(0, "segoe".to_owned());
        }
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts
}

fn system_font() -> Option<Vec<u8>> {
    let windows = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
    let folder = std::path::Path::new(&windows).join("Fonts");
    ["SegUIVar.ttf", "segoeui.ttf"]
        .iter()
        .find_map(|name| std::fs::read(folder.join(name)).ok())
}

fn visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = RAISED;
    visuals.text_edit_bg_color = Some(BG);
    visuals.window_stroke = Stroke::new(1.0, LINE);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(6);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = WARN;
    visuals.error_fg_color = ERROR;
    visuals.selection.bg_fill = SELECTION;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    let radius = CornerRadius::same(4);
    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = PANEL;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    widgets.noninteractive.corner_radius = radius;
    for (state, fill, stroke) in [
        (&mut widgets.inactive, RAISED, LINE),
        (
            &mut widgets.hovered,
            Color32::from_rgb(0x3d, 0x3d, 0x3d),
            MUTED,
        ),
        (
            &mut widgets.active,
            Color32::from_rgb(0x47, 0x47, 0x47),
            ACCENT,
        ),
        (&mut widgets.open, RAISED, LINE),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, stroke);
        state.fg_stroke = Stroke::new(1.0, TEXT);
        state.corner_radius = radius;
        state.expansion = 0.0;
    }
    visuals
}
