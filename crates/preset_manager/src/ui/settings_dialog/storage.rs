//! Speicherorte der App (Sicherungen, Logdateien, Einstellungen) und
//! „Über“.

use std::path::Path;

use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::safe_fs::{default_backup_root, BACKUPS_TO_KEEP};

use crate::app::App;
use crate::logging;
use crate::texts::{self, tr, tr_format};
use crate::ui::{theme, widgets};

use super::Action;

pub(super) fn storage_section(ui: &mut Ui, action: &mut Option<Action>) {
    widgets::section_heading(ui, tr("SICHERUNGEN UND PROTOKOLL", "BACKUPS AND LOG"));
    ui.add_space(4.0);
    ui.label(
        RichText::new(tr_format!(
            "Bevor die App eine Datei ersetzt oder ändert, sichert sie das Original. \
             Die Sicherungen der letzten {BACKUPS_TO_KEEP} Aktionen bleiben erhalten. \
             Gelöschte Einträge liegen im Windows-Papierkorb.",
            "Before the app replaces or changes a file, it backs up the original. \
             Backups of the last {BACKUPS_TO_KEEP} actions are kept. \
             Deleted items are in the Windows Recycle Bin."
        ))
        .small()
        .color(theme::MUTED),
    );
    ui.add_space(4.0);
    folder_row(
        ui,
        tr("Sicherungen", "Backups"),
        default_backup_root().as_deref(),
        action,
    );
    folder_row(
        ui,
        tr("Logdateien", "Log files"),
        logging::log_dir().as_deref(),
        action,
    );
}

fn folder_row(ui: &mut Ui, label: &str, path: Option<&Path>, action: &mut Option<Action>) {
    ui.horizontal(|ui| {
        ui.add_sized([90.0, 20.0], egui::Label::new(label));
        match path {
            Some(path) => {
                if widgets::icon_button(ui, icon::FOLDER_OPEN, tr("Ordner öffnen", "Open folder"))
                    .on_hover_text(path.display().to_string())
                    .clicked()
                {
                    *action = Some(Action::Open(path.to_path_buf()));
                }
            }
            None => {
                ui.label(
                    RichText::new(tr(
                        "%LOCALAPPDATA% ist nicht gesetzt",
                        "%LOCALAPPDATA% is not set",
                    ))
                    .color(theme::MUTED),
                );
            }
        }
    });
}

pub(super) fn about_section(ui: &mut Ui, app: &App) {
    widgets::section_heading(ui, tr("ÜBER", "ABOUT"));
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!(
            "{} {}",
            texts::APP_TITLE,
            app.updater.current_version
        ))
        .strong(),
    );
    ui.label(
        RichText::new(tr(
            "MIT-Lizenz · © 2026 Steffen Schwabe",
            "MIT License · © 2026 Steffen Schwabe",
        ))
        .small()
        .color(theme::MUTED),
    );
    ui.label(
        RichText::new(texts::trademark_note())
            .small()
            .color(theme::MUTED),
    );
}

/// Ein Pfad in einer Zeile, bei Platzmangel gekürzt, voll im Tooltip.
pub(super) fn path_line(ui: &mut Ui, path: Option<&Path>) {
    let text = path.map_or_else(|| "–".to_owned(), |path| path.display().to_string());
    widgets::truncated(ui, RichText::new(&text).monospace()).on_hover_text(text);
}
