//! Sprache und Größe der Oberfläche.

use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as icon;

use crate::app::App;
use crate::settings::{LanguageChoice, ZOOM_STEPS};
use crate::texts::{self, tr, tr_format, Language};
use crate::ui::{theme, widgets};

use super::Action;

/// Die Überschrift steht in beiden Sprachen – wer die falsche Sprache
/// erwischt hat, findet die Umstellung trotzdem.
pub(super) fn language_section(ui: &mut Ui, app: &App, action: &mut Option<Action>) {
    widgets::section_heading(ui, "SPRACHE · LANGUAGE");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for choice in LanguageChoice::ALL {
            let label = match choice {
                LanguageChoice::System => {
                    let system = match texts::system_language() {
                        Language::German => "Deutsch",
                        Language::English => "English",
                    };
                    tr_format!("Wie Windows ({system})", "Same as Windows ({system})")
                }
                LanguageChoice::German => "Deutsch".to_owned(),
                LanguageChoice::English => "English".to_owned(),
            };
            if ui.radio(app.settings.language == choice, label).clicked()
                && app.settings.language != choice
            {
                *action = Some(Action::Language(choice));
            }
        }
    });
}

pub(super) fn zoom_section(ui: &mut Ui, ctx: &egui::Context) {
    widgets::section_heading(ui, tr("DARSTELLUNG", "DISPLAY"));
    ui.add_space(4.0);
    // Feste Stufen statt Schieberegler: Ein Regler wächst beim Ziehen mit
    // der Oberfläche unter dem Mauszeiger mit und läuft davon.
    let zoom = ctx.zoom_factor();
    ui.horizontal(|ui| {
        ui.label(tr("Größe der Oberfläche", "Interface size"));
        ui.add_space(8.0);
        let smaller = ZOOM_STEPS.iter().rev().find(|step| **step < zoom - 0.001);
        let larger = ZOOM_STEPS.iter().find(|step| **step > zoom + 0.001);
        if ui
            .add_enabled(smaller.is_some(), egui::Button::new(icon::MINUS))
            .on_hover_text(tr("Kleiner", "Smaller"))
            .clicked()
        {
            if let Some(step) = smaller {
                ctx.set_zoom_factor(*step);
            }
        }
        ui.add_sized(
            [52.0, 20.0],
            egui::Label::new(RichText::new(format!("{:.0} %", zoom * 100.0)).strong()),
        );
        if ui
            .add_enabled(larger.is_some(), egui::Button::new(icon::PLUS))
            .on_hover_text(tr("Größer", "Larger"))
            .clicked()
        {
            if let Some(step) = larger {
                ctx.set_zoom_factor(*step);
            }
        }
        ui.add_space(8.0);
        if ui
            .add_enabled(
                (zoom - 1.0).abs() > 0.001,
                egui::Button::new(tr("Zurücksetzen", "Reset")),
            )
            .clicked()
        {
            ctx.set_zoom_factor(1.0);
        }
    });
    ui.label(
        RichText::new(tr(
            "Auch mit Strg + Plus / Minus, Strg + 0 setzt zurück.",
            "Also with Ctrl + Plus / Minus, Ctrl + 0 resets.",
        ))
        .small()
        .color(theme::MUTED),
    );
}
