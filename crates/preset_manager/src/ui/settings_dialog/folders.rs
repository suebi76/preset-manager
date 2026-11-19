//! Der Camera-Raw-Ordner: automatisch, gefundene Ordner (auch neben
//! Lightroom-Katalogen) oder ein eigener; Warnung, wenn Lightroom Classic
//! Presets mit dem Katalog speichert.

use eframe::egui::{self, Frame, Margin, RichText, Ui};
use egui_phosphor::regular as icon;
use preset_core::adobe::{locations::same_path, FolderKind, FoundFolder};

use crate::app::{App, FolderSource, CUSTOM_DIR_VARIABLE};
use crate::texts::{self, tr, tr_format};
use crate::ui::{theme, widgets};

use super::storage::path_line;
use super::Action;

pub(super) fn folder_section(ui: &mut Ui, app: &App, action: &mut Option<Action>) {
    widgets::section_heading(ui, tr("CAMERA-RAW-ORDNER", "CAMERA RAW FOLDER"));
    ui.add_space(4.0);
    path_line(ui, app.camera_raw_dir.as_deref());
    let lightroom_elsewhere = lightroom_reads_catalog(app);
    let source = match app.folder_source {
        FolderSource::Automatic if lightroom_elsewhere => {
            tr("Automatisch gefunden.", "Found automatically.").to_owned()
        }
        FolderSource::Automatic => tr(
            "Automatisch gefunden – derselbe Ordner für Photoshop, Photoshop Beta und Lightroom Classic.",
            "Found automatically – the same folder for Photoshop, Photoshop Beta and Lightroom Classic.",
        )
        .to_owned(),
        FolderSource::Chosen => tr("Selbst gewählt.", "Chosen by you.").to_owned(),
        FolderSource::Environment => tr_format!(
            "Vorgegeben über die Umgebungsvariable {CUSTOM_DIR_VARIABLE}; solange sie gesetzt ist, lässt sich der Ordner hier nicht ändern.",
            "Set by the environment variable {CUSTOM_DIR_VARIABLE}; while it is set, the folder cannot be changed here."
        ),
    };
    ui.label(RichText::new(source).small().color(theme::MUTED));
    ui.add_space(6.0);

    let can_change = app.folder_source != FolderSource::Environment && !app.busy();
    if lightroom_elsewhere {
        lightroom_warning(ui);
    }
    found_folders(ui, app, can_change, action);
    ui.horizontal(|ui| {
        let pick = ui
            .add_enabled(
                can_change,
                egui::Button::new(format!(
                    "{}  {}",
                    icon::FOLDER_OPEN,
                    texts::choose_camera_raw()
                )),
            )
            .on_disabled_hover_text(texts::wait_for_action());
        if pick.clicked() {
            *action = Some(Action::PickFolder);
        }
        if app.folder_source == FolderSource::Chosen
            && ui
                .add_enabled(
                    can_change,
                    egui::Button::new(format!(
                        "{}  {}",
                        icon::MAGIC_WAND,
                        tr("Automatisch suchen", "Search automatically")
                    )),
                )
                .clicked()
        {
            *action = Some(Action::AutomaticFolder);
        }
        if let Some(dir) = app.camera_raw_dir.as_deref().filter(|dir| dir.is_dir()) {
            if widgets::icon_button(ui, icon::ARROW_SQUARE_OUT, tr("Öffnen", "Open")).clicked() {
                *action = Some(Action::Open(dir.to_path_buf()));
            }
        }
    });
}

/// Lightroom Classic liest Presets zurzeit neben dem Katalog, verwaltet wird
/// aber der Benutzerordner: Was hier installiert wird, sieht nur Photoshop.
fn lightroom_reads_catalog(app: &App) -> bool {
    let current = app.camera_raw_dir.as_deref();
    let uses_user_folder = app.found_folders.iter().any(|folder| {
        folder.kind == FolderKind::User
            && current.is_some_and(|dir| same_path(dir, &folder.camera_raw_dir))
    });
    let catalog_in_use = app.found_folders.iter().any(|folder| {
        matches!(
            folder.kind,
            FolderKind::LightroomCatalog { in_use: true, .. }
        )
    });
    uses_user_folder && catalog_in_use
}

fn lightroom_warning(ui: &mut Ui) {
    Frame::new()
        .fill(theme::WARN_BG)
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!(
                    "{}  {}",
                    icon::WARNING,
                    tr(
                        "In Lightroom Classic ist „Presets mit diesem Katalog speichern“ eingeschaltet. Was hier installiert wird, sieht dann nur Photoshop – für Lightroom unten den Katalogordner wählen.",
                        "Lightroom Classic has “Store presets with this catalog” turned on. What you install here then shows up only in Photoshop – for Lightroom, choose the catalog folder below.",
                    )
                ))
                .small()
                .color(theme::WARN_TEXT),
            );
        });
    ui.add_space(6.0);
}

/// Die gefundenen Camera-Raw-Ordner zur Auswahl – nur, wenn es mehr als den
/// Benutzerordner gibt.
fn found_folders(ui: &mut Ui, app: &App, can_change: bool, action: &mut Option<Action>) {
    if app.found_folders.len() < 2 {
        return;
    }
    ui.label(
        RichText::new(tr(
            "Gefunden auf diesem Rechner:",
            "Found on this computer:",
        ))
        .small()
        .color(theme::MUTED),
    );
    for folder in &app.found_folders {
        let current = app
            .camera_raw_dir
            .as_deref()
            .is_some_and(|dir| same_path(dir, &folder.camera_raw_dir));
        ui.add_enabled_ui(can_change, |ui| {
            let response = ui
                .radio(current, found_title(folder))
                .on_hover_text(folder.camera_raw_dir.display().to_string())
                .on_disabled_hover_text(texts::wait_for_action());
            if response.clicked() && !current {
                *action = Some(match folder.kind {
                    FolderKind::User => Action::AutomaticFolder,
                    FolderKind::LightroomCatalog { .. } => {
                        Action::UseFolder(folder.camera_raw_dir.clone())
                    }
                });
            }
        });
        ui.indent(folder.camera_raw_dir.as_os_str(), |ui| {
            ui.label(
                RichText::new(found_note(folder))
                    .small()
                    .color(theme::FAINT),
            );
        });
    }
    ui.add_space(6.0);
}

fn found_title(folder: &FoundFolder) -> String {
    match &folder.kind {
        FolderKind::User => tr("Benutzerordner", "User folder").to_owned(),
        FolderKind::LightroomCatalog { catalog, .. } => {
            let name = catalog
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            tr_format!("Lightroom-Katalog „{name}“", "Lightroom catalog “{name}”")
        }
    }
}

fn found_note(folder: &FoundFolder) -> String {
    match &folder.kind {
        FolderKind::User => tr(
            "Für Photoshop, Photoshop Beta und Lightroom Classic – der Standard.",
            "For Photoshop, Photoshop Beta and Lightroom Classic – the default.",
        )
        .to_owned(),
        FolderKind::LightroomCatalog { in_use: true, .. } => tr(
            "Nur Lightroom Classic, „Presets mit diesem Katalog speichern“ ist an.",
            "Lightroom Classic only, “Store presets with this catalog” is on.",
        )
        .to_owned(),
        FolderKind::LightroomCatalog { in_use: false, .. } => tr(
            "Nur Lightroom Classic, und nur wenn dort „Presets mit diesem Katalog speichern“ an ist – zurzeit aus.",
            "Lightroom Classic only, and only while “Store presets with this catalog” is on there – currently off.",
        )
        .to_owned(),
    }
}
