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
//!
//! | Datei | Inhalt |
//! |---|---|
//! | `panels` | die Bereiche (Profil, Grundeinstellungen …) |
//! | `sliders` | Regler und Textzeilen |
//! | `curve` | Gradationskurve |
//! | `mixer` | Farbmischer |
//! | `grading` | Color-Grading-Räder |
//! | `wording` | Zahlen, Namen und Sätze |

mod curve;
mod grading;
mod mixer;
mod panels;
mod sliders;
mod wording;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::preset::adjustments::{Fact, Panel};
use preset_core::preset::{read_settings, PresetSettings};

use crate::texts::{self, tr};
use crate::ui::theme;
use panels::basic_panel;
use panels::curve_panel;
use panels::grading_panel;
use panels::local_panel;
use panels::mixer_panel;
use panels::profile_panel;
use sliders::slider_row;
use sliders::Track;
use wording::capitalize;
use wording::panel_title;
use wording::phrase;
use wording::slider_label;

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
