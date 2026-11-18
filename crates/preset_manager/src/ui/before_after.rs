//! Vorher/Nachher in den Details (F-39, Teil 2 – **Testversion**).
//!
//! Ein Foto (oder eine Testtafel) wird mit den Werten des Presets
//! nachgerechnet ([`preset_core::preview`]); ein Schieber teilt das Bild in
//! Vorher (links) und Nachher (rechts). Gerechnet wird im Hintergrund, damit
//! die Oberfläche flüssig bleibt. Die Vorschau ist eine Annäherung – was sie
//! nicht nachbildet, steht darunter.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::SystemTime;

use eframe::egui::{self, Color32, ColorImage, Rect, RichText, Sense, TextureHandle, Ui, Vec2};
use egui_phosphor::regular as icon;
use preset_core::preset::color_table::ColorTable;
use preset_core::preset::{read_settings, Preset, PresetSettings, PresetType};
use preset_core::preview::{self, Image, LookTable, Missing};

use crate::app::{App, ToastKind};
use crate::settings;
use crate::texts::{tr, tr_format};
use crate::ui::theme;

/// Längste Kante des Vorschaubilds in Pixeln.
const MAX_EDGE: u32 = 1100;

/// Was zur gerade gezeigten Vorschau gehört.
#[derive(Default)]
pub struct PreviewState {
    source: Option<Source>,
    after: Option<(Key, TextureHandle)>,
    pending: Option<(Key, Receiver<Image>)>,
    /// Lage des Schiebers, 0 (alles Nachher) … 1 (alles Vorher).
    split: f32,
    /// Farbtabellen der Profile, nach Datei.
    tables: HashMap<PathBuf, Arc<Vec<ColorTable>>>,
    /// Die große Ansicht ist offen.
    pub large: bool,
}

struct Source {
    photo: Option<PathBuf>,
    image: Arc<Image>,
    texture: TextureHandle,
}

/// Wann ein Ergebnis neu gerechnet werden muss.
#[derive(Clone, PartialEq)]
struct Key {
    preset: PathBuf,
    modified: Option<SystemTime>,
    photo: Option<PathBuf>,
}

/// Zeigt die Vorschau für ein Preset in der verfügbaren Breite.
pub fn show(ui: &mut Ui, app: &mut App, preset: &Preset, settings: &PresetSettings, now: f64) {
    let ctx = ui.ctx().clone();
    ensure_source(&ctx, app, now);
    let look = look_table(app, settings);
    request(&ctx, app, preset, settings, look.clone());

    let width = ui.available_width();
    draw(ui, &mut app.preview, width, 240.0);

    let missing = preview::missing(
        settings,
        look.as_ref().map(|(tables, amount)| LookTable {
            tables,
            amount: *amount,
        }),
    );
    let mut note = tr(
        "Testversion · Annäherung: Camera Raw rechnet auf Rohdaten anders. Die Richtung stimmt, Feinheiten nicht.",
        "Test version · approximation: Camera Raw works differently on raw files. The direction is right, details are not.",
    )
    .to_owned();
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|m| missing_name(*m)).collect();
        note.push_str(
            "

",
        );
        note.push_str(&tr_format!(
            "Nicht nachgebildet: {}",
            "Not reproduced: {}",
            names.join(", ")
        ));
    }

    // Eine ruhige Zeile unter dem Bild: Hinweis links, Werkzeuge rechts.
    ui.horizontal(|ui| {
        let marker = if missing.is_empty() { "" } else { " *" };
        ui.label(
            RichText::new(format!(
                "{}  {}{marker}",
                icon::INFO,
                tr("Annäherung", "Approximation")
            ))
            .small()
            .color(theme::FAINT),
        )
        .on_hover_text(note);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let tool = |ui: &mut Ui, glyph: &str, tip: &str| {
                ui.add(egui::Button::new(RichText::new(glyph).color(theme::MUTED)).frame(false))
                    .on_hover_text(tip)
                    .clicked()
            };
            if tool(ui, icon::ARROWS_OUT, tr("Groß anzeigen", "Show large")) {
                app.preview.large = true;
            }
            if app.settings.preview_photo.is_some()
                && tool(
                    ui,
                    icon::SQUARES_FOUR,
                    tr(
                        "Testtafel statt Foto (Verläufe und typische Farben)",
                        "Test chart instead of photo (gradients and typical colors)",
                    ),
                )
            {
                set_photo(app, None, now);
            }
            if tool(
                ui,
                icon::IMAGE,
                tr("Eigenes Foto wählen …", "Choose your own photo …"),
            ) {
                choose_photo(app, now);
            }
        });
    });
    ui.add_space(6.0);

    if app.preview.large {
        large_view(&ctx, app);
    }
}

/// Die große Ansicht als eigenes Fenster über allem.
fn large_view(ctx: &egui::Context, app: &mut App) {
    let screen = ctx.content_rect();
    let mut open = true;
    egui::Modal::new(egui::Id::new("before-after-large")).show(ctx, |ui| {
        let width = (screen.width() - 120.0).max(300.0);
        let height = (screen.height() - 180.0).max(200.0);
        ui.set_width(width);
        draw(ui, &mut app.preview, width, height);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr(
                    "Ziehen oder klicken verschiebt die Trennlinie: links vorher, rechts nachher.",
                    "Drag or click to move the divider: before on the left, after on the right.",
                ))
                .small()
                .color(theme::MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr("Schließen", "Close")).clicked() {
                    open = false;
                }
            });
        });
    });
    if !open || ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        app.preview.large = false;
    }
}

/// Zeichnet Vorher/Nachher mit Schieber, höchstens `max_height` hoch.
fn draw(ui: &mut Ui, state: &mut PreviewState, width: f32, max_height: f32) {
    let Some(source) = &state.source else {
        return;
    };
    let aspect = source.image.height as f32 / source.image.width.max(1) as f32;
    let mut size = Vec2::new(width, width * aspect);
    if size.y > max_height {
        size = Vec2::new(max_height / aspect, max_height);
    }
    let (outer, response) =
        ui.allocate_exact_size(Vec2::new(width, size.y), Sense::click_and_drag());
    let rect = Rect::from_center_size(outer.center(), size);
    if state.split <= 0.0 {
        state.split = 0.5;
    }
    if let Some(pointer) = response.interact_pointer_pos() {
        state.split = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    }

    let painter = ui.painter_at(outer);
    let full_uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let after = state
        .after
        .as_ref()
        .map_or(source.texture.id(), |(_, texture)| texture.id());
    painter.image(after, rect, full_uv, Color32::WHITE);

    let split_x = rect.left() + rect.width() * state.split;
    let before_rect = Rect::from_min_max(rect.min, egui::pos2(split_x, rect.bottom()));
    let before_uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(state.split, 1.0));
    painter.image(source.texture.id(), before_rect, before_uv, Color32::WHITE);

    painter.line_segment(
        [
            egui::pos2(split_x, rect.top()),
            egui::pos2(split_x, rect.bottom()),
        ],
        egui::Stroke::new(2.0, Color32::WHITE),
    );
    painter.circle_filled(egui::pos2(split_x, rect.center().y), 9.0, Color32::WHITE);
    painter.text(
        egui::pos2(split_x, rect.center().y),
        egui::Align2::CENTER_CENTER,
        icon::ARROWS_LEFT_RIGHT,
        egui::FontId::proportional(12.0),
        Color32::BLACK,
    );
    let label = |text: &str, pos: egui::Pos2, align: egui::Align2| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );
        let label_rect = align.anchor_size(pos, galley.size()).expand(4.0);
        painter.rect_filled(label_rect, 3.0, Color32::from_black_alpha(150));
        painter.galley(label_rect.min + Vec2::splat(4.0), galley, Color32::WHITE);
    };
    label(
        tr("Vorher", "Before"),
        rect.left_top() + Vec2::new(8.0, 8.0),
        egui::Align2::LEFT_TOP,
    );
    label(
        tr("Nachher", "After"),
        rect.right_top() + Vec2::new(-8.0, 8.0),
        egui::Align2::RIGHT_TOP,
    );
    if state.pending.is_some() {
        painter.text(
            rect.right_bottom() - Vec2::new(8.0, 8.0),
            egui::Align2::RIGHT_BOTTOM,
            tr("rechnet …", "working …"),
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
}

/// Lädt das gewählte Foto (oder die Testtafel), wenn es sich geändert hat.
fn ensure_source(ctx: &egui::Context, app: &mut App, now: f64) {
    let wanted = app.settings.preview_photo.clone();
    if app
        .preview
        .source
        .as_ref()
        .is_some_and(|source| source.photo == wanted)
    {
        return;
    }
    let image = match &wanted {
        Some(path) => match load_photo(path) {
            Ok(image) => image,
            Err(message) => {
                app.toast(message, ToastKind::Error, now);
                app.settings.preview_photo = None;
                let _ = settings::save_settings(&app.settings);
                preview::test_chart(900, 600)
            }
        },
        None => preview::test_chart(900, 600),
    };
    let texture = ctx.load_texture("preview-before", to_color_image(&image), Default::default());
    app.preview.source = Some(Source {
        photo: app.settings.preview_photo.clone(),
        image: Arc::new(image),
        texture,
    });
    app.preview.after = None;
    app.preview.pending = None;
}

fn load_photo(path: &Path) -> Result<Image, String> {
    let photo = image::open(path).map_err(|error| {
        tr_format!(
            "Das Foto konnte nicht geöffnet werden ({error}).",
            "The photo could not be opened ({error})."
        )
    })?;
    let photo = photo.thumbnail(MAX_EDGE, MAX_EDGE).to_rgb8();
    let (width, height) = (photo.width() as usize, photo.height() as usize);
    let pixels = photo
        .pixels()
        .map(|pixel| pixel.0.map(|value| f32::from(value) / 255.0))
        .collect();
    Ok(Image {
        width,
        height,
        pixels,
    })
}

fn to_color_image(image: &Image) -> ColorImage {
    let bytes: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|pixel| pixel.map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8))
        .collect();
    ColorImage::from_rgb([image.width, image.height], &bytes)
}

/// Startet die Berechnung im Hintergrund oder holt ein fertiges Ergebnis ab.
fn request(
    ctx: &egui::Context,
    app: &mut App,
    preset: &Preset,
    settings: &PresetSettings,
    look: Option<(Arc<Vec<ColorTable>>, f32)>,
) {
    let state = &mut app.preview;
    let Some(source) = &state.source else {
        return;
    };
    let key = Key {
        preset: preset.path.clone(),
        modified: std::fs::metadata(&preset.path)
            .and_then(|metadata| metadata.modified())
            .ok(),
        photo: source.photo.clone(),
    };
    if let Some((pending_key, receiver)) = &state.pending {
        if let Ok(image) = receiver.try_recv() {
            let texture =
                ctx.load_texture("preview-after", to_color_image(&image), Default::default());
            state.after = Some((pending_key.clone(), texture));
            state.pending = None;
        } else if *pending_key == key {
            return;
        }
    }
    if state.after.as_ref().is_some_and(|(done, _)| *done == key) {
        return;
    }
    let (sender, receiver) = mpsc::channel();
    let image = Arc::clone(&source.image);
    let settings = settings.clone();
    let repaint = ctx.clone();
    std::thread::spawn(move || {
        let look = look.as_ref().map(|(tables, amount)| LookTable {
            tables,
            amount: *amount,
        });
        let result = preview::develop(&image, &settings, look);
        let _ = sender.send(result);
        repaint.request_repaint();
    });
    state.pending = Some((key, receiver));
}

/// Die Farbtabelle des Profils: aus der Datei selbst oder – wenn das Preset
/// nur auf ein Profil verweist – aus der Profildatei mit derselben Kennung.
fn look_table(app: &mut App, settings: &PresetSettings) -> Option<(Arc<Vec<ColorTable>>, f32)> {
    let amount = settings
        .look
        .as_ref()
        .and_then(|look| look.amount)
        .or_else(|| settings.number("Amount"))
        .unwrap_or(1.0);
    if !settings.color_tables.is_empty() {
        let tables = decode_all(&settings.color_tables);
        return (!tables.is_empty()).then(|| (Arc::new(tables), amount));
    }
    let uuid = settings.look.as_ref()?.uuid.clone()?;
    let profile = app
        .all_presets()
        .find(|preset| {
            preset.preset_type == PresetType::Profile && preset.uuid.as_deref() == Some(&uuid)
        })?
        .path
        .clone();
    let table = app
        .preview
        .tables
        .entry(profile.clone())
        .or_insert_with(|| {
            Arc::new(
                read_settings(&profile)
                    .map(|profile| decode_all(&profile.color_tables))
                    .unwrap_or_default(),
            )
        })
        .clone();
    (!table.is_empty()).then_some((table, amount))
}

fn decode_all(texts: &[String]) -> Vec<ColorTable> {
    texts
        .iter()
        .filter_map(|text| ColorTable::decode(text))
        .collect()
}

fn choose_photo(app: &mut App, now: f64) {
    if let Some(path) = rfd::FileDialog::new()
        .set_title(tr("Foto für die Vorschau", "Photo for the preview"))
        .add_filter(tr("Bilder", "Images"), &["jpg", "jpeg", "png"])
        .pick_file()
    {
        set_photo(app, Some(path), now);
    }
}

fn set_photo(app: &mut App, photo: Option<PathBuf>, now: f64) {
    app.settings.preview_photo = photo;
    if let Err(error) = settings::save_settings(&app.settings) {
        app.toast(
            tr_format!(
                "Einstellung konnte nicht gespeichert werden ({error}).",
                "The setting could not be saved ({error})."
            ),
            ToastKind::Error,
            now,
        );
    }
}

fn missing_name(missing: Missing) -> &'static str {
    match missing {
        Missing::Masks => tr("Masken", "masks"),
        Missing::Calibration => tr("Kalibrierung", "calibration"),
        Missing::Detail => tr(
            "Schärfen und Rauschreduzierung",
            "sharpening and noise reduction",
        ),
        Missing::Optics => tr("Objektivkorrekturen", "lens corrections"),
        Missing::CameraProfile => tr("Kameraprofil", "camera profile"),
        Missing::ProfileTable => tr(
            "Farbtabelle des Profils (Profil nicht gefunden)",
            "profile color table (profile not found)",
        ),
        Missing::FixedWhiteBalance => tr(
            "fester Weißabgleich (nur ungefähr)",
            "fixed white balance (approximate only)",
        ),
    }
}
