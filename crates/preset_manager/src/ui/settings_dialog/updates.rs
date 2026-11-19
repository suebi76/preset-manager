//! Updates: Stand, Suche, automatisch installieren, Testversionen.

use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as icon;

use crate::app::App;
use crate::settings::UpdateMode;
use crate::texts::{self, tr};
use crate::ui::{theme, widgets};
use crate::updates::{self, Status};

use super::Action;

pub(super) fn update_section(ui: &mut Ui, app: &App, action: &mut Option<Action>) {
    widgets::section_heading(ui, "UPDATES");
    ui.add_space(4.0);
    let updater = &app.updater;
    ui.label(updater.status_text());
    if let Status::Downloading { percent, .. } = updater.status {
        ui.add(egui::ProgressBar::new(f32::from(percent) / 100.0).desired_width(300.0));
    }
    if !updater.is_installed() {
        if updates::STORE {
            return;
        }
        ui.label(
            RichText::new(tr(
                "Die installierte Fassung gibt es als PresetManager-Setup.exe unter Releases auf GitHub.",
                "The installable version is PresetManager-Setup.exe under Releases on GitHub.",
            ))
                .small()
                .color(theme::MUTED),
        );
        return;
    }
    ui.add_space(4.0);
    for mode in UpdateMode::ALL {
        let (label, hint) = texts::update_mode(mode);
        if ui
            .radio(app.settings.updates == mode, label)
            .on_hover_text(hint)
            .clicked()
            && app.settings.updates != mode
        {
            *action = Some(Action::UpdateMode(mode));
        }
    }
    ui.label(
        RichText::new(texts::update_mode(app.settings.updates).1)
            .small()
            .color(theme::MUTED),
    );
    let mut test_versions = app.settings.test_versions;
    if ui
        .checkbox(
            &mut test_versions,
            tr("Testversionen erhalten", "Receive test versions"),
        )
        .on_hover_text(tr(
            "Vorabversionen zum Ausprobieren. Sie können noch Fehler enthalten.",
            "Pre-releases for trying out. They may still contain bugs.",
        ))
        .changed()
    {
        *action = Some(Action::TestVersions(test_versions));
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !updater.busy(),
                egui::Button::new(format!(
                    "{}  {}",
                    icon::ARROWS_CLOCKWISE,
                    texts::update_check()
                )),
            )
            .clicked()
        {
            *action = Some(Action::CheckUpdates);
        }
        let label = match updater.status {
            Status::Available(_) => Some(texts::update_install()),
            Status::Ready(_) => Some(texts::update_restart()),
            _ => None,
        };
        if let Some(label) = label {
            if widgets::primary_button(ui, label, !app.restart_pending).clicked() {
                *action = Some(Action::InstallUpdate);
            }
        }
    });
}
