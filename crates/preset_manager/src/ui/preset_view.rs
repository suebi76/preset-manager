//! „Was macht dieses Preset?“ – die Werte eines Presets, wie in Camera Raw
//! geordnet und gezeichnet (F-39).
//!
//! - ein Satz in Klartext ([`summary`])
//! - Bereiche wie in Camera Raw: Regler mit Mittelpunkt, die Gradationskurve
//!   als Zeichnung, der Farbmischer als farbige Balken, das Color Grading als
//!   Farbräder, Profil und Masken als Liste ([`panels`])
//!
//! Gezeigt wird nur, was vom Standard abweicht. Gelesen wird die Datei einmal
//! je Auswahl ([`SettingsCache`]).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use eframe::egui::{
    self, Color32, CornerRadius, Mesh, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, Vec2,
};
use egui_phosphor::regular as icon;
use preset_core::preset::adjustments::{
    curve_value, ColorChange, ColorGrading, CurveChannel, Fact, HslColor, Panel, Scale, Slider,
    ToneZone, Wheel, WhiteBalance,
};
use preset_core::preset::settings::{CurvePoint, LocalAdjustment, MaskKind};
use preset_core::preset::{read_settings, PresetSettings};

use crate::texts::{self, tr, tr_format, Language};
use crate::ui::theme;

/// Die gelesenen Einstellungen des ausgewählten Presets. Neu gelesen wird,
/// wenn sich Auswahl oder Datei ändern.
pub struct SettingsCache {
    path: PathBuf,
    modified: Option<SystemTime>,
    pub result: Result<PresetSettings, String>,
}

impl SettingsCache {
    /// Liefert die Einstellungen für `path` – aus dem Speicher, wenn die
    /// Datei seit dem letzten Lesen gleich geblieben ist.
    pub fn get<'a>(cache: &'a mut Option<Self>, path: &Path) -> &'a Result<PresetSettings, String> {
        let modified = std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok();
        let fresh = cache
            .as_ref()
            .is_some_and(|cached| cached.path == path && cached.modified == modified);
        if !fresh {
            *cache = None;
        }
        let cached = cache.get_or_insert_with(|| Self {
            path: path.to_path_buf(),
            modified,
            result: read_settings(path).map_err(|error| texts::describe(&error)),
        });
        &cached.result
    }
}

/// Der Satz in Klartext, z. B. „Heller, mehr Kontrast, wärmer, Filmkorn.“
pub fn summary(ui: &mut Ui, settings: &PresetSettings) {
    let facts = settings.facts();
    let (main, extra): (Vec<&Fact>, Vec<&Fact>) = facts.iter().partition(|fact| {
        !matches!(
            fact,
            Fact::Profile(_) | Fact::ColorTable | Fact::LocalAdjustments(_)
        )
    });

    let sentence = if main.is_empty() {
        tr(
            "Ändert kaum etwas an den Reglern.",
            "Barely changes any sliders.",
        )
        .to_owned()
    } else {
        let phrases: Vec<String> = main.iter().map(|fact| phrase(fact)).collect();
        capitalize(&format!("{}.", phrases.join(", ")))
    };
    ui.label(RichText::new(sentence).size(14.0).color(theme::TEXT));
    for fact in extra {
        ui.label(RichText::new(phrase(fact)).small().color(theme::MUTED));
    }
}

/// Alle Bereiche, die etwas enthalten.
pub fn panels(ui: &mut Ui, settings: &PresetSettings) {
    let force = expand_toolbar(ui);
    ui.data_mut(|data| data.insert_temp(force_id(), force));
    let mut shown = false;
    shown |= profile_panel(ui, settings);
    shown |= basic_panel(ui, settings);
    shown |= curve_panel(ui, settings);
    shown |= mixer_panel(ui, settings);
    shown |= grading_panel(ui, settings);
    for panel in [
        Panel::Detail,
        Panel::Effects,
        Panel::Calibration,
        Panel::Optics,
    ] {
        let sliders = settings.sliders(panel);
        if !sliders.is_empty() {
            shown = true;
            section(ui, panel_title(panel), |ui| {
                for slider in &sliders {
                    slider_row(ui, slider_label(slider.spec.key), slider, Track::Plain);
                }
            });
        }
    }
    shown |= local_panel(ui, settings);
    if !shown {
        ui.label(
            RichText::new(tr(
                "Keine Regler weichen vom Standard ab.",
                "No slider differs from the default.",
            ))
            .color(theme::MUTED),
        );
    }
}

/// Unter diesem Schlüssel steht im aktuellen Bild, ob alle Bereiche auf-
/// (`Some(true)`) oder zugeklappt (`Some(false)`) werden sollen.
fn force_id() -> egui::Id {
    egui::Id::new("preset-view-expand-all")
}

/// Zeile über den Bereichen: „Alle aufklappen“ und „Alle zuklappen“. Gibt
/// zurück, ob in diesem Bild eine der beiden geklickt wurde.
fn expand_toolbar(ui: &mut Ui) -> Option<bool> {
    let mut force = None;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(tr("EINSTELLUNGEN", "SETTINGS"))
                .small()
                .strong()
                .color(theme::FAINT),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = |ui: &mut Ui, glyph: &str, text: &str| {
                ui.add(
                    egui::Button::new(
                        RichText::new(format!("{glyph} {text}"))
                            .small()
                            .color(theme::MUTED),
                    )
                    .frame(false),
                )
                .clicked()
            };
            if button(
                ui,
                icon::CARET_DOUBLE_UP,
                tr("Alle zuklappen", "Collapse all"),
            ) {
                force = Some(false);
            }
            if button(
                ui,
                icon::CARET_DOUBLE_DOWN,
                tr("Alle aufklappen", "Expand all"),
            ) {
                force = Some(true);
            }
        });
    });
    ui.add_space(2.0);
    force
}

/// Ein Bereich wie in Camera Raw – Überschrift in ruhigen Großbuchstaben,
/// auf- und zuklappbar.
fn section(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    egui::CollapsingHeader::new(
        RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(theme::MUTED),
    )
    .id_salt(("preset-view", title))
    .default_open(true)
    .open(
        ui.data(|data| data.get_temp::<Option<bool>>(force_id()))
            .flatten(),
    )
    .show_unindented(ui, |ui| {
        ui.add_space(2.0);
        add(ui);
        ui.add_space(4.0);
    });
    ui.add_space(2.0);
}

// ---------------------------------------------------------------- Bereiche

fn profile_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let look = settings.look.as_ref().filter(|look| !look.name.is_empty());
    let profile = settings
        .text("CameraProfile")
        .filter(|name| !name.is_empty() && *name != "Embedded");
    let black_and_white = settings.is_black_and_white();
    if look.is_none() && profile.is_none() && !black_and_white && !settings.uses_color_table {
        return false;
    }
    section(ui, tr("Profil", "Profile"), |ui| {
        if let Some(look) = look {
            let amount = look
                .amount
                .filter(|amount| (amount - 1.0).abs() > 0.005)
                .map(|amount| format!(" · {} %", (amount * 100.0).round()))
                .unwrap_or_default();
            value_row(
                ui,
                tr("Profil", "Profile"),
                &format!("{}{amount}", look.name),
            );
        } else if let Some(profile) = profile {
            value_row(ui, tr("Kameraprofil", "Camera profile"), profile);
        }
        if black_and_white {
            value_row(
                ui,
                tr("Behandlung", "Treatment"),
                tr("Schwarzweiß", "Black & white"),
            );
        }
        if settings.uses_color_table {
            note(
                ui,
                tr(
                    "Enthält eine Farbtabelle (LUT). Ihr Look lässt sich nicht als Regler zeigen.",
                    "Contains a color table (LUT). Its look cannot be shown as sliders.",
                ),
            );
        }
    });
    true
}

fn basic_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let sliders = settings.sliders(Panel::Basic);
    let white_balance = settings.white_balance();
    if sliders.is_empty() && white_balance == WhiteBalance::AsShot {
        return false;
    }
    section(ui, panel_title(Panel::Basic), |ui| {
        match &white_balance {
            WhiteBalance::AsShot => {}
            WhiteBalance::Named(name) => {
                value_row(
                    ui,
                    tr("Weißabgleich", "White balance"),
                    white_balance_name(name),
                );
            }
            WhiteBalance::Custom { temperature, tint } => {
                let mut parts = Vec::new();
                if let Some(kelvin) = temperature {
                    parts.push(format!("{} K", kelvin.round()));
                }
                if let Some(tint) = tint {
                    parts.push(format!(
                        "{} {}",
                        tr("Tönung", "Tint"),
                        number(*tint, 0, true)
                    ));
                }
                value_row(ui, tr("Weißabgleich", "White balance"), &parts.join(" · "));
                note(
                    ui,
                    tr(
                        "Feste Werte – wirken so nur bei Rohdaten.",
                        "Fixed values – work like this on raw files only.",
                    ),
                );
            }
        }
        for slider in &sliders {
            let track = match slider.spec.key {
                "IncrementalTemperature" => Track::Temperature,
                "IncrementalTint" => Track::Tint,
                _ => Track::Plain,
            };
            slider_row(ui, slider_label(slider.spec.key), slider, track);
        }
    });
    true
}

fn curve_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let curves = settings.point_curves();
    let sliders = settings.sliders(Panel::Curve);
    if curves.is_empty() && sliders.is_empty() {
        return false;
    }
    section(ui, panel_title(Panel::Curve), |ui| {
        if !curves.is_empty() {
            curve_graph(ui, &curves);
            ui.add_space(6.0);
        }
        for slider in &sliders {
            slider_row(ui, slider_label(slider.spec.key), slider, Track::Plain);
        }
    });
    true
}

fn mixer_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let rows = settings.mixer();
    let gray = settings.gray_mixer();
    if rows.is_empty() && gray.is_empty() {
        return false;
    }
    if rows.is_empty() {
        section(ui, tr("Schwarzweiß-Mischer", "B&W mixer"), |ui| {
            for (color, value) in &gray {
                mixer_line(ui, *color, &[(*value, MixerPart::Luminance)]);
            }
        });
        return true;
    }
    section(ui, tr("Farbmischer", "Color mixer"), |ui| {
        mixer_header(ui);
        for row in &rows {
            mixer_line(
                ui,
                row.color,
                &[
                    (row.hue, MixerPart::Hue),
                    (row.saturation, MixerPart::Saturation),
                    (row.luminance, MixerPart::Luminance),
                ],
            );
        }
    });
    true
}

fn grading_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let Some(grading) = settings.color_grading() else {
        return false;
    };
    section(ui, "Color Grading", |ui| {
        grading_wheels(ui, &grading);
        if (grading.blending - 50.0).abs() > 0.5 {
            value_row(
                ui,
                tr("Überblendung", "Blending"),
                &number(grading.blending, 0, false),
            );
        }
        if grading.balance.abs() > 0.5 {
            value_row(
                ui,
                tr("Abgleich", "Balance"),
                &number(grading.balance, 0, true),
            );
        }
    });
    true
}

fn local_panel(ui: &mut Ui, settings: &PresetSettings) -> bool {
    let adjustments: Vec<&LocalAdjustment> = settings
        .local_adjustments
        .iter()
        .filter(|adjustment| adjustment.has_effect())
        .collect();
    if adjustments.is_empty() {
        return false;
    }
    section(ui, tr("Masken", "Masking"), |ui| {
        for (index, adjustment) in adjustments.iter().enumerate() {
            let name = if adjustment.name.is_empty() {
                tr_format!("Maske {}", "Mask {}", index + 1)
            } else {
                adjustment.name.clone()
            };
            let kinds: Vec<&str> = adjustment
                .masks
                .iter()
                .map(|kind| mask_kind(*kind))
                .collect();
            ui.label(RichText::new(name).strong());
            if !kinds.is_empty() {
                ui.label(RichText::new(kinds.join(" + ")).small().color(theme::MUTED));
            }
            let changes: Vec<String> = adjustment
                .changes
                .iter()
                .map(|(key, value)| {
                    let direction = if *value > 0.0 { "+" } else { "−" };
                    format!("{} {direction}", local_label(key))
                })
                .collect();
            ui.label(
                RichText::new(changes.join(" · "))
                    .small()
                    .color(theme::TEXT),
            );
            ui.add_space(4.0);
        }
        note(
            ui,
            tr(
                "Masken passen sich in Camera Raw dem Foto an; ihre Lage lässt sich hier nicht zeigen.",
                "Masks adapt to the photo in Camera Raw; their position cannot be shown here.",
            ),
        );
    });
    true
}

// ---------------------------------------------------------------- Regler

/// Wie die Spur eines Reglers gefärbt ist.
#[derive(Clone, Copy)]
enum Track {
    Plain,
    Temperature,
    Tint,
}

/// Eine Zeile: Bezeichnung links, Regler in der Mitte, Wert rechts.
fn slider_row(ui: &mut Ui, label: &str, slider: &Slider, track: Track) {
    let signed = matches!(slider.spec.scale, Scale::Bipolar { .. });
    let value_text = number(slider.value, slider.spec.decimals, signed);
    let (min, max) = match slider.spec.scale {
        Scale::Bipolar { min, max } => (min, max),
        Scale::Unipolar { max } => (0.0, max),
    };
    let width = ui.available_width();
    let (row, response) = ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::hover());
    let painter = ui.painter_at(row);
    let label_width = (width * 0.42).min(150.0);
    let value_width = 44.0;
    painter.galley(
        Pos2::new(row.left(), row.center().y - 8.0),
        one_line(ui, label, label_width - 6.0, theme::MUTED),
        theme::MUTED,
    );
    painter.text(
        Pos2::new(row.right(), row.center().y),
        egui::Align2::RIGHT_CENTER,
        &value_text,
        egui::FontId::monospace(12.0),
        theme::TEXT,
    );
    let rect = Rect::from_x_y_ranges(
        (row.left() + label_width)..=(row.right() - value_width),
        (row.center().y - 4.0)..=(row.center().y + 4.0),
    );
    response.on_hover_text(format!("{label}: {value_text}"));
    let bar = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 3.0));
    match track {
        Track::Plain => {
            painter.rect_filled(bar, CornerRadius::same(1), theme::LINE);
        }
        Track::Temperature => gradient_bar(
            &painter,
            bar,
            Color32::from_rgb(0x4a, 0x7b, 0xd8),
            Color32::from_rgb(0xe0, 0xc0, 0x3a),
        ),
        Track::Tint => gradient_bar(
            &painter,
            bar,
            Color32::from_rgb(0x4c, 0xb0, 0x5a),
            Color32::from_rgb(0xc8, 0x4c, 0xc0),
        ),
    }
    let position = |value: f32| {
        let t = ((value - min) / (max - min)).clamp(0.0, 1.0);
        bar.left() + t * bar.width()
    };
    let neutral = position(slider.spec.neutral.clamp(min, max));
    let value = position(slider.value);
    if matches!(track, Track::Plain) {
        let fill = Rect::from_x_y_ranges(neutral.min(value)..=neutral.max(value), bar.y_range());
        painter.rect_filled(fill, CornerRadius::same(1), theme::ACCENT);
    }
    if signed {
        painter.line_segment(
            [
                Pos2::new(neutral, rect.top()),
                Pos2::new(neutral, rect.bottom()),
            ],
            Stroke::new(1.0, theme::FAINT),
        );
    }
    painter.circle_filled(Pos2::new(value, bar.center().y), 3.5, theme::TEXT);
}

/// Text in einer Zeile, bei Platzmangel mit „…“ gekürzt.
fn one_line(ui: &Ui, text: &str, width: f32, color: Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        egui::TextStyle::Body.resolve(ui.style()),
        color,
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(10.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

fn gradient_bar(painter: &egui::Painter, rect: Rect, left: Color32, right: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// Bezeichnung links, Wert rechts.
fn value_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(RichText::new(value).color(theme::TEXT)).truncate());
        });
    });
}

fn note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::FAINT));
}

// ---------------------------------------------------------------- Kurve

fn curve_graph(ui: &mut Ui, curves: &[(CurveChannel, &[CurvePoint])]) {
    let size = ui.available_width().min(260.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), theme::BG);
    let inner = rect.shrink(6.0);
    let grid = Stroke::new(1.0, theme::LINE);
    for i in 1..4 {
        let t = i as f32 / 4.0;
        let x = inner.left() + t * inner.width();
        let y = inner.top() + t * inner.height();
        painter.line_segment(
            [Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())],
            grid,
        );
        painter.line_segment(
            [Pos2::new(inner.left(), y), Pos2::new(inner.right(), y)],
            grid,
        );
    }
    painter.line_segment(
        [inner.left_bottom(), inner.right_top()],
        Stroke::new(1.0, theme::FAINT),
    );
    let to_screen = |x: f32, y: f32| {
        Pos2::new(
            inner.left() + x / 255.0 * inner.width(),
            inner.bottom() - y / 255.0 * inner.height(),
        )
    };
    // Erst die Farbkanäle, dann die Helligkeit obenauf.
    let mut ordered: Vec<_> = curves.to_vec();
    ordered.sort_by_key(|(channel, _)| *channel == CurveChannel::Master);
    for (channel, points) in ordered {
        let color = curve_color(channel);
        let line: Vec<Pos2> = (0..=64)
            .map(|i| {
                let x = i as f32 * 255.0 / 64.0;
                to_screen(x, curve_value(points, x))
            })
            .collect();
        painter.add(Shape::line(line, Stroke::new(1.6, color)));
        for (x, y) in points {
            painter.circle_filled(to_screen(*x, *y), 2.5, color);
        }
    }
    if curves
        .iter()
        .any(|(channel, _)| *channel != CurveChannel::Master)
    {
        ui.horizontal_wrapped(|ui| {
            for (channel, _) in curves {
                ui.label(
                    RichText::new(format!("● {}", channel_name(*channel)))
                        .small()
                        .color(curve_color(*channel)),
                );
            }
        });
    }
}

fn curve_color(channel: CurveChannel) -> Color32 {
    match channel {
        CurveChannel::Master => theme::TEXT,
        CurveChannel::Red => Color32::from_rgb(0xe0, 0x55, 0x55),
        CurveChannel::Green => Color32::from_rgb(0x55, 0xc0, 0x6a),
        CurveChannel::Blue => Color32::from_rgb(0x5a, 0x8c, 0xe8),
    }
}

fn channel_name(channel: CurveChannel) -> &'static str {
    match channel {
        CurveChannel::Master => tr("Helligkeit", "Luminance"),
        CurveChannel::Red => tr("Rot", "Red"),
        CurveChannel::Green => tr("Grün", "Green"),
        CurveChannel::Blue => tr("Blau", "Blue"),
    }
}

// ---------------------------------------------------------------- Farbmischer

#[derive(Clone, Copy, PartialEq)]
enum MixerPart {
    Hue,
    Saturation,
    Luminance,
}

const MIXER_NAME_WIDTH: f32 = 74.0;

fn mixer_header(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.add_space(MIXER_NAME_WIDTH);
        let width = mixer_cell_width(ui);
        for title in [
            tr("Farbton", "Hue"),
            tr("Sättigung", "Saturation"),
            tr("Luminanz", "Luminance"),
        ] {
            ui.add_sized(
                [width, 14.0],
                egui::Label::new(RichText::new(title).small().color(theme::FAINT)),
            );
        }
    });
}

fn mixer_cell_width(ui: &Ui) -> f32 {
    ((ui.available_width() - MIXER_NAME_WIDTH) / 3.0 - ui.spacing().item_spacing.x).max(24.0)
}

fn mixer_line(ui: &mut Ui, color: HslColor, values: &[(f32, MixerPart)]) {
    ui.horizontal(|ui| {
        let (swatch, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
        ui.painter().circle_filled(
            swatch.center(),
            5.0,
            hue_color(color.hue_degrees(), 0.75, 0.9),
        );
        let name_width = MIXER_NAME_WIDTH - 10.0 - ui.spacing().item_spacing.x;
        let (name_rect, _) = ui.allocate_exact_size(Vec2::new(name_width, 16.0), Sense::hover());
        ui.painter_at(name_rect).text(
            name_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            color_name(color),
            egui::TextStyle::Body.resolve(ui.style()),
            theme::TEXT,
        );
        let columns = values.len().max(1) as f32;
        let width = (ui.available_width() / columns - ui.spacing().item_spacing.x).max(24.0);
        for (value, part) in values {
            mixer_cell(ui, color, *value, *part, width);
        }
    });
}

fn mixer_cell(ui: &mut Ui, color: HslColor, value: f32, part: MixerPart, width: f32) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 16.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let bar = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 4.0, 4.0));
    painter.rect_filled(bar, CornerRadius::same(1), theme::LINE);
    let center = bar.center().x;
    let end = center + (value / 100.0).clamp(-1.0, 1.0) * bar.width() / 2.0;
    let fill_color = match part {
        MixerPart::Hue => {
            let shifted = color.hue_degrees() + value.signum() * 30.0;
            hue_color(shifted, 0.75, 0.9)
        }
        MixerPart::Saturation => hue_color(color.hue_degrees(), 0.8, 0.9),
        MixerPart::Luminance => {
            if value >= 0.0 {
                hue_color(color.hue_degrees(), 0.35, 1.0)
            } else {
                hue_color(color.hue_degrees(), 0.8, 0.45)
            }
        }
    };
    if value.abs() >= 0.5 {
        let fill = Rect::from_x_y_ranges(center.min(end)..=center.max(end), bar.y_range());
        painter.rect_filled(fill, CornerRadius::same(1), fill_color);
    }
    painter.line_segment(
        [
            Pos2::new(center, rect.top() + 3.0),
            Pos2::new(center, rect.bottom() - 3.0),
        ],
        Stroke::new(1.0, theme::FAINT),
    );
    response.on_hover_text(number(value, 0, true));
}

// ---------------------------------------------------------------- Color Grading

fn grading_wheels(ui: &mut Ui, grading: &ColorGrading) {
    let wheels = [
        (tr("Tiefen", "Shadows"), grading.shadows),
        (tr("Mitteltöne", "Midtones"), grading.midtones),
        (tr("Lichter", "Highlights"), grading.highlights),
        (tr("Global", "Global"), grading.global),
    ];
    let spacing = ui.spacing().item_spacing.x;
    let size = ((ui.available_width() - 3.0 * spacing) / 4.0).clamp(36.0, 80.0);
    ui.horizontal(|ui| {
        for (title, wheel) in wheels {
            ui.vertical(|ui| {
                ui.set_width(size);
                color_wheel(ui, size, &wheel);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(title).small().color(theme::MUTED));
                    if wheel.luminance.abs() >= 0.5 {
                        ui.label(
                            RichText::new(number(wheel.luminance, 0, true))
                                .small()
                                .color(theme::FAINT),
                        )
                        .on_hover_text(tr("Luminanz", "Luminance"));
                    }
                });
            });
        }
    });
    ui.add_space(4.0);
}

fn color_wheel(ui: &mut Ui, size: f32, wheel: &Wheel) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = size / 2.0 - 2.0;

    // Farbkreis: außen volle Farbe, innen grau.
    let mut mesh = Mesh::default();
    let segments = 48;
    mesh.colored_vertex(center, Color32::from_gray(0x55));
    for i in 0..=segments {
        let degrees = i as f32 * 360.0 / segments as f32;
        mesh.colored_vertex(
            center + wheel_offset(degrees, radius),
            hue_color(degrees, 0.65, 0.75),
        );
    }
    for i in 1..=segments as u32 {
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(Shape::mesh(mesh));
    painter.circle_stroke(center, radius, Stroke::new(1.0, theme::LINE));

    let active = wheel.saturation >= 0.5;
    let dot = center
        + wheel_offset(
            wheel.hue,
            radius * (wheel.saturation / 100.0).clamp(0.0, 1.0),
        );
    painter.circle_filled(
        dot,
        4.0,
        if active {
            hue_color(wheel.hue, 0.8, 1.0)
        } else {
            Color32::from_gray(0x99)
        },
    );
    painter.circle_stroke(dot, 4.0, Stroke::new(1.5, Color32::WHITE));
    if active {
        response.on_hover_text(tr_format!(
            "Farbton {}°, Sättigung {}",
            "Hue {}°, saturation {}",
            wheel.hue.round(),
            wheel.saturation.round()
        ));
    }
}

/// Camera Raw zeichnet 0° (Rot) rechts, gegen den Uhrzeigersinn steigend.
fn wheel_offset(degrees: f32, radius: f32) -> Vec2 {
    let radians = degrees.to_radians();
    Vec2::new(radians.cos() * radius, -radians.sin() * radius)
}

fn hue_color(degrees: f32, saturation: f32, value: f32) -> Color32 {
    egui::ecolor::Hsva::new(degrees.rem_euclid(360.0) / 360.0, saturation, value, 1.0).into()
}

// ---------------------------------------------------------------- Texte

/// Eine Zahl wie in Camera Raw: mit Vorzeichen bei Reglern mit Mittelpunkt,
/// im Deutschen mit Komma.
fn number(value: f32, decimals: u8, signed: bool) -> String {
    let rounded = if decimals == 0 { value.round() } else { value };
    let mut text = format!("{:.*}", usize::from(decimals), rounded.abs());
    if texts::language() == Language::German {
        text = text.replace('.', ",");
    }
    let zero = rounded.abs() < 0.5 * 10f32.powi(-i32::from(decimals));
    match (signed, zero, rounded < 0.0) {
        (_, true, _) => text,
        (_, false, true) => format!("−{text}"),
        (true, false, false) => format!("+{text}"),
        (false, false, false) => text,
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn phrase(fact: &Fact) -> String {
    let fixed = match fact {
        Fact::BlackAndWhite => tr("Schwarzweiß", "black & white"),
        Fact::Brighter => tr("heller", "brighter"),
        Fact::Darker => tr("dunkler", "darker"),
        Fact::MoreContrast => tr("mehr Kontrast", "more contrast"),
        Fact::LessContrast => tr("weniger Kontrast", "less contrast"),
        Fact::FadedBlacks => tr("matte, angehobene Schwarztöne", "faded, lifted blacks"),
        Fact::MutedWhites => tr("gedämpftes Weiß", "muted whites"),
        Fact::BrighterShadows => tr("aufgehellte Tiefen", "lifted shadows"),
        Fact::DarkerShadows => tr("tiefere Schatten", "deeper shadows"),
        Fact::SofterHighlights => tr("zurückgenommene Lichter", "softened highlights"),
        Fact::Warmer => tr("wärmer", "warmer"),
        Fact::Cooler => tr("kühler", "cooler"),
        Fact::Magenta => tr("Stich ins Magenta", "a magenta tint"),
        Fact::Greener => tr("Stich ins Grüne", "a green tint"),
        Fact::MoreColorful => tr("kräftigere Farben", "richer colors"),
        Fact::LessColorful => tr("gedämpfte Farben", "muted colors"),
        Fact::MoreTexture => tr("mehr Struktur und Klarheit", "more texture and clarity"),
        Fact::Softer => tr("weicher", "softer"),
        Fact::LessHaze => tr("Dunst entfernt", "haze removed"),
        Fact::MoreHaze => tr("dunstiger, verträumter", "hazier, dreamier"),
        Fact::DarkVignette => tr("dunkle Ränder (Vignette)", "darkened edges (vignette)"),
        Fact::LightVignette => tr("aufgehellte Ränder", "lightened edges"),
        Fact::Grain => tr("Filmkorn", "film grain"),
        Fact::ColorTable => tr(
            "Der Look steckt in einer Farbtabelle (LUT).",
            "The look lives in a color table (LUT).",
        ),
        Fact::Color { color, change } => {
            let name = color_name(*color);
            return match change {
                ColorChange::Saturated => tr_format!("{name} kräftiger", "stronger {name}"),
                ColorChange::Desaturated => tr_format!("{name} entsättigt", "{name} desaturated"),
                ColorChange::Lighter => tr_format!("{name} heller", "lighter {name}"),
                ColorChange::Darker => tr_format!("{name} dunkler", "darker {name}"),
                ColorChange::Shifted => {
                    tr_format!("{name} im Farbton verschoben", "{name} hue shifted")
                }
            };
        }
        Fact::Toned { zone, hue } => {
            let color = hue_name(*hue);
            return match zone {
                ToneZone::Shadows => tr_format!("Tiefen {color} getönt", "shadows tinted {color}"),
                ToneZone::Midtones => {
                    tr_format!("Mitteltöne {color} getönt", "midtones tinted {color}")
                }
                ToneZone::Highlights => {
                    tr_format!("Lichter {color} getönt", "highlights tinted {color}")
                }
                ToneZone::Overall => {
                    tr_format!("insgesamt {color} getönt", "tinted {color} overall")
                }
            };
        }
        Fact::Profile(name) => {
            return tr_format!(
                "Baut auf dem Profil „{name}“ auf.",
                "Built on the profile “{name}”."
            )
        }
        Fact::LocalAdjustments(count) => {
            return if *count == 1 {
                tr(
                    "Eine lokale Anpassung mit Maske.",
                    "One local adjustment with a mask.",
                )
                .to_owned()
            } else {
                tr_format!(
                    "{count} lokale Anpassungen mit Masken.",
                    "{count} local adjustments with masks."
                )
            }
        }
    };
    fixed.to_owned()
}

fn color_name(color: HslColor) -> &'static str {
    match color {
        HslColor::Red => tr("Rot", "red"),
        HslColor::Orange => tr("Orange", "orange"),
        HslColor::Yellow => tr("Gelb", "yellow"),
        HslColor::Green => tr("Grün", "green"),
        HslColor::Aqua => tr("Aquamarin", "aqua"),
        HslColor::Blue => tr("Blau", "blue"),
        HslColor::Purple => tr("Lila", "purple"),
        HslColor::Magenta => tr("Magenta", "magenta"),
    }
}

/// Ein Wort für einen Farbton in Grad.
fn hue_name(degrees: f32) -> &'static str {
    match degrees.rem_euclid(360.0) {
        d if d < 15.0 => tr("rot", "red"),
        d if d < 45.0 => tr("orange", "orange"),
        d if d < 70.0 => tr("gelb", "yellow"),
        d if d < 160.0 => tr("grün", "green"),
        d if d < 200.0 => tr("türkis", "teal"),
        d if d < 250.0 => tr("blau", "blue"),
        d if d < 290.0 => tr("violett", "violet"),
        d if d < 340.0 => tr("magenta", "magenta"),
        _ => tr("rot", "red"),
    }
}

fn panel_title(panel: Panel) -> &'static str {
    match panel {
        Panel::Basic => tr("Grundeinstellungen", "Basic"),
        Panel::Curve => tr("Gradationskurve", "Tone curve"),
        Panel::Detail => tr("Details", "Detail"),
        Panel::Effects => tr("Effekte", "Effects"),
        Panel::Calibration => tr("Kalibrierung", "Calibration"),
        Panel::Optics => tr("Optik", "Optics"),
    }
}

fn white_balance_name(name: &str) -> &str {
    match name {
        "Auto" => tr("Automatisch", "Auto"),
        "Daylight" => tr("Tageslicht", "Daylight"),
        "Cloudy" => tr("Bewölkt", "Cloudy"),
        "Shade" => tr("Schatten", "Shade"),
        "Tungsten" => tr("Kunstlicht", "Tungsten"),
        "Fluorescent" => tr("Leuchtstoffröhre", "Fluorescent"),
        "Flash" => tr("Blitz", "Flash"),
        other => other,
    }
}

fn mask_kind(kind: MaskKind) -> &'static str {
    match kind {
        MaskKind::Linear => tr("Linearer Verlauf", "Linear gradient"),
        MaskKind::Radial => tr("Radialverlauf", "Radial gradient"),
        MaskKind::Brush => tr("Pinsel", "Brush"),
        MaskKind::Ai => tr(
            "KI-Maske (z. B. Motiv, Himmel)",
            "AI mask (e.g. subject, sky)",
        ),
        MaskKind::Range => tr("Bereich", "Range"),
        MaskKind::Other => tr("Maske", "Mask"),
    }
}

/// Bezeichnung eines Reglers in Camera Raw.
fn slider_label(key: &str) -> &'static str {
    match key {
        "Exposure2012" => tr("Belichtung", "Exposure"),
        "IncrementalTemperature" => tr("Temperatur", "Temperature"),
        "IncrementalTint" => tr("Tönung", "Tint"),
        "Contrast2012" => tr("Kontrast", "Contrast"),
        "Highlights2012" | "ParametricHighlights" => tr("Lichter", "Highlights"),
        "Shadows2012" | "ParametricShadows" => tr("Tiefen", "Shadows"),
        "Whites2012" => tr("Weiß", "Whites"),
        "Blacks2012" => tr("Schwarz", "Blacks"),
        "Texture" => tr("Struktur", "Texture"),
        "Clarity2012" => tr("Klarheit", "Clarity"),
        "Dehaze" => tr("Dunst entfernen", "Dehaze"),
        "Vibrance" => tr("Dynamik", "Vibrance"),
        "Saturation" => tr("Sättigung", "Saturation"),
        "ParametricLights" => tr("Helle Mitteltöne", "Lights"),
        "ParametricDarks" => tr("Dunkle Mitteltöne", "Darks"),
        "Sharpness" => tr("Schärfen", "Sharpening"),
        "SharpenRadius" => tr("Radius", "Radius"),
        "SharpenDetail" => tr("Details", "Detail"),
        "SharpenEdgeMasking" => tr("Maskieren", "Masking"),
        "LuminanceSmoothing" => tr("Rauschreduzierung", "Noise reduction"),
        "ColorNoiseReduction" => tr("Farbrauschen", "Color noise reduction"),
        "PostCropVignetteAmount" => tr("Vignette", "Vignette"),
        "PostCropVignetteMidpoint" => tr("Mittelpunkt", "Midpoint"),
        "PostCropVignetteRoundness" => tr("Rundheit", "Roundness"),
        "PostCropVignetteFeather" => tr("Weiche Kante", "Feather"),
        "PostCropVignetteHighlightContrast" => tr("Lichter", "Highlights"),
        "GrainAmount" => tr("Körnung", "Grain"),
        "GrainSize" => tr("Größe", "Size"),
        "GrainFrequency" => tr("Unregelmäßigkeit", "Roughness"),
        "ShadowTint" => tr("Tiefen – Tönung", "Shadows tint"),
        "RedHue" => tr("Rot – Farbton", "Red hue"),
        "RedSaturation" => tr("Rot – Sättigung", "Red saturation"),
        "GreenHue" => tr("Grün – Farbton", "Green hue"),
        "GreenSaturation" => tr("Grün – Sättigung", "Green saturation"),
        "BlueHue" => tr("Blau – Farbton", "Blue hue"),
        "BlueSaturation" => tr("Blau – Sättigung", "Blue saturation"),
        "VignetteAmount" => tr("Objektiv-Vignettierung", "Lens vignetting"),
        _ => tr("Regler", "Slider"),
    }
}

/// Bezeichnung eines lokalen Reglers (Schlüssel ohne `Local`).
fn local_label(key: &str) -> &'static str {
    match key {
        "Exposure2012" | "Exposure" => tr("Belichtung", "Exposure"),
        "Contrast2012" | "Contrast" => tr("Kontrast", "Contrast"),
        "Highlights2012" => tr("Lichter", "Highlights"),
        "Shadows2012" => tr("Tiefen", "Shadows"),
        "Whites2012" => tr("Weiß", "Whites"),
        "Blacks2012" => tr("Schwarz", "Blacks"),
        "Clarity2012" | "Clarity" => tr("Klarheit", "Clarity"),
        "Texture" => tr("Struktur", "Texture"),
        "Dehaze" => tr("Dunst", "Dehaze"),
        "Saturation" => tr("Sättigung", "Saturation"),
        "Temperature" => tr("Temperatur", "Temperature"),
        "Tint" => tr("Tönung", "Tint"),
        "Sharpness" => tr("Schärfe", "Sharpness"),
        "LuminanceNoise" => tr("Rauschen", "Noise"),
        "Moire" => tr("Moiré", "Moiré"),
        "Defringe" => tr("Rand entfernen", "Defringe"),
        "Brightness" => tr("Helligkeit", "Brightness"),
        "ToningSaturation" => tr("Farbe", "Color"),
        "Hue" => tr("Farbton", "Hue"),
        "Grain" => tr("Körnung", "Grain"),
        _ => tr("weitere Regler", "other sliders"),
    }
}
