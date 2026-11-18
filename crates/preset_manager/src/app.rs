//! Zustand der Anwendung und der Ablauf eines Bildes.
//!
//! [`App`] hält, was die Oberfläche gerade zeigt (gewählte Gruppe, Auswahl,
//! Suche, offene Dialoge …). Die eigentliche Arbeit macht `preset_core` im
//! Hintergrund-Thread ([`crate::worker`]); die Module unter [`crate::ui`]
//! zeichnen nur. Was der Nutzer auslösen kann, steht in [`crate::actions`].

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use eframe::egui;
use preset_core::adobe::{AdobeApp, AdobeLocations, FoundFolder};
use preset_core::import::{ConflictPolicy, ImportPlan};
use preset_core::library::Library;
use preset_core::preset::{Preset, PresetOrigin, Section};
use preset_core::report::ItemOutcome;
use preset_core::CoreError;

use crate::settings;
use crate::texts::{self, tr, tr_format};
use crate::ui;
use crate::updates::{UpdateEvent, Updater};
use crate::worker::{Command, Event, Worker};

/// Wie oft nachgesehen wird, ob Photoshop oder Lightroom läuft.
const ADOBE_CHECK_SECONDS: f64 = 5.0;
/// Wie lange eine Einblendung stehen bleibt.
const TOAST_SECONDS: f64 = 5.0;
const ERROR_TOAST_SECONDS: f64 = 12.0;
/// Umgebungsvariable für einen anderen Camera-Raw-Ordner (F-43). Sie geht
/// der Einstellung vor – gedacht für Tests mit einer Kopie.
pub const CUSTOM_DIR_VARIABLE: &str = "APM_CAMERA_RAW_DIR";
/// So lange wird nach einer Zoom-Änderung gewartet, bevor sie gespeichert
/// wird – beim Ziehen am Regler nicht bei jedem Bild.
const ZOOM_SAVE_DELAY: f64 = 0.6;

/// Woher der Camera-Raw-Ordner kommt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderSource {
    /// Unter `%APPDATA%\Adobe\CameraRaw` gefunden.
    Automatic,
    /// Im Einstellungsdialog gewählt.
    Chosen,
    /// Über [`CUSTOM_DIR_VARIABLE`] vorgegeben.
    Environment,
}

/// Welche Bereiche angezeigt werden (Strg+1/2/3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Presets,
    Profiles,
}

impl Filter {
    pub fn shows(self, section: Section) -> bool {
        match self {
            Self::All => true,
            Self::Presets => section == Section::Presets,
            Self::Profiles => section == Section::Profiles,
        }
    }
}

/// Was links ausgewählt ist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Group {
        section: Section,
        name: String,
    },
    /// Die Liste der Dateien, die nicht gelesen werden konnten.
    Problems,
}

/// Der offene Importdialog.
pub struct ImportDialog {
    /// `None`, während der Plan im Hintergrund ausgeführt wird.
    pub plan: Option<Box<ImportPlan>>,
    /// Quellen, die abgelehnt wurden, mit Begründung.
    pub rejected: Vec<(PathBuf, String)>,
    pub policy: ConflictPolicy,
}

/// Was gerade umbenannt wird (F2 oder Doppelklick).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameTarget {
    Preset(PathBuf),
    Group { section: Section, name: String },
}

pub struct Rename {
    pub target: RenameTarget,
    /// Der Text im Eingabefeld.
    pub text: String,
    /// Beim ersten Bild Fokus setzen und Text markieren.
    pub focus: bool,
}

/// Rückfragen vor Aktionen, die sich nicht von selbst erklären.
pub enum Confirm {
    DeletePresets(Vec<Preset>),
    /// Die Einträge einer Gruppe – nur die des angezeigten Zustands (aktiv
    /// oder deaktiviert).
    DeleteGroup {
        name: String,
        count: usize,
        presets: Vec<Preset>,
    },
    Merge {
        section: Section,
        source: String,
        target: String,
        /// Einträge, die verschoben werden (ohne Namenskonflikte).
        presets: Vec<Preset>,
        /// Einträge, die wegen gleichen Namens stehen bleiben.
        clashes: usize,
    },
}

/// Was beim Ziehen innerhalb des Fensters getragen wird.
#[derive(Debug, Clone)]
pub enum DragPayload {
    Items(Vec<Preset>),
    Group { section: Section, name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Info,
    Error,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub until: f64,
}

pub struct App {
    pub worker: Option<Worker>,
    /// Fehler, ohne den die App nicht arbeiten kann (Camera Raw fehlt).
    pub fatal: Option<String>,
    pub library: Option<Library>,
    /// Dateien, die nicht gelesen werden konnten, mit Begründung.
    pub problems: Vec<(PathBuf, String)>,
    pub filter: Filter,
    pub search: String,
    pub place: Option<Place>,
    /// Ausgewählte Einträge (über ihre Pfade, damit die Auswahl ein neues
    /// Einlesen übersteht).
    pub selection: BTreeSet<PathBuf>,
    /// Ausgangspunkt für Umschalt+Klick.
    pub anchor: Option<PathBuf>,
    pub rename: Option<Rename>,
    pub confirm: Option<Confirm>,
    /// Neu angelegte Gruppen ohne Inhalt (F-33), in den Einstellungen gemerkt.
    pub empty_groups: Vec<(Section, String)>,
    pub import: Option<ImportDialog>,
    /// Eine Quelle wird gerade durchsucht.
    pub scanning: bool,
    /// Ein Verwalten-Auftrag läuft; bis zum neuen Einlesen keine weiteren.
    pub managing: bool,
    /// Gruppe, die nach dem nächsten Einlesen gezeigt werden soll.
    pub landing: Option<Place>,
    /// Pfade, die nach dem nächsten Einlesen ausgewählt sein sollen.
    pub reselect: Vec<PathBuf>,
    /// Gruppe, die nach dem nächsten Einlesen umbenannt werden soll
    /// („Neue Gruppe aus Auswahl“).
    pub rename_after_load: Option<(Section, String)>,
    pub toasts: Vec<Toast>,
    pub adobe_apps: Vec<AdobeApp>,
    pub hint_dismissed: bool,
    next_adobe_check: f64,
    /// Lage der Gruppen links im letzten Bild – um zu erkennen, auf welche
    /// Gruppe Dateien gezogen werden.
    pub group_rects: Vec<(Place, egui::Rect)>,
    /// Begründung, warum das aktuelle Ziehziel nicht geht (für den Tooltip).
    pub drag_hint: Option<String>,
    pub focus_search: bool,
    /// Der verwendete (oder vergeblich gesuchte) Camera-Raw-Ordner.
    pub camera_raw_dir: Option<PathBuf>,
    pub folder_source: FolderSource,
    /// Alle Camera-Raw-Ordner auf diesem Rechner (Benutzerordner,
    /// Lightroom-Kataloge) – zur Auswahl im Einstellungsdialog.
    pub found_folders: Vec<FoundFolder>,
    pub settings: settings::Settings,
    pub settings_open: bool,
    /// Neue Versionen (Velopack).
    pub updater: Updater,
    /// Ein Update soll installiert werden, sobald keine Aktion mehr läuft.
    pub restart_pending: bool,
    /// Zeitpunkt, zu dem eine geänderte Vergrößerung gespeichert wird.
    zoom_save_at: Option<f64>,
    /// Welcher Zustand der gewählten Gruppe angezeigt wird: die aktiven
    /// Einträge (`true`) oder die deaktivierten (F-48).
    pub place_active: bool,
    /// Seite der Details für ein einzelnes Preset („Wirkung“/„Verwalten“).
    pub details_tab: ui::details::DetailsTab,
    /// Die gelesenen Einstellungen des ausgewählten Presets.
    pub settings_cache: Option<ui::preset_view::SettingsCache>,
    /// Vorher/Nachher.
    pub preview: ui::before_after::PreviewState,
}

impl App {
    /// Startet die App. `start_paths` sind Ordner/ZIPs, die beim Aufruf
    /// übergeben wurden (z. B. auf `Start.bat` gezogen).
    pub fn new(cc: &eframe::CreationContext<'_>, start_paths: Vec<PathBuf>) -> Self {
        ui::theme::apply(&cc.egui_ctx);
        let settings = settings::load_settings();
        texts::set_language(settings.language.resolve());
        cc.egui_ctx.set_zoom_factor(settings.zoom);
        let updater = Updater::new(&settings);
        let mut app = Self {
            worker: None,
            fatal: None,
            library: None,
            problems: Vec::new(),
            filter: Filter::All,
            search: String::new(),
            place: None,
            selection: BTreeSet::new(),
            anchor: None,
            rename: None,
            confirm: None,
            empty_groups: settings::load_empty_groups(),
            import: None,
            scanning: false,
            managing: false,
            landing: None,
            reselect: Vec::new(),
            rename_after_load: None,
            toasts: Vec::new(),
            adobe_apps: Vec::new(),
            hint_dismissed: false,
            next_adobe_check: 0.0,
            group_rects: Vec::new(),
            drag_hint: None,
            focus_search: false,
            camera_raw_dir: None,
            folder_source: FolderSource::Automatic,
            found_folders: Vec::new(),
            settings,
            settings_open: false,
            updater,
            restart_pending: false,
            zoom_save_at: None,
            place_active: true,
            details_tab: ui::details::DetailsTab::default(),
            settings_cache: None,
            preview: ui::before_after::PreviewState::default(),
        };
        app.connect(&cc.egui_ctx);
        if app.worker.is_some() && !start_paths.is_empty() {
            app.start_import(start_paths, None, 0.0);
        }
        app
    }

    /// Sucht den Camera-Raw-Ordner und startet dafür den Hintergrund-Thread.
    ///
    /// Reihenfolge: Umgebungsvariable (F-43), dann der in den Einstellungen
    /// gewählte Ordner, sonst automatisch. Wird auch nach einem Ordnerwechsel
    /// aufgerufen; alles, was zum alten Ordner gehörte, wird verworfen.
    pub(crate) fn connect(&mut self, ctx: &egui::Context) {
        // Leerzeichen am Ende entstehen leicht mit `set X=... && ...` in cmd.
        // Nie stillschweigend auf den echten Ordner ausweichen, wenn die
        // Variable gesetzt ist – auch nicht bei ungewöhnlichen Zeichen.
        let from_environment = std::env::var_os(CUSTOM_DIR_VARIABLE)
            .map(|value| match value.to_str() {
                Some(text) => text.trim().into(),
                None => value,
            })
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let (wanted, source) = match (from_environment, &self.settings.camera_raw_dir) {
            (Some(dir), _) => (Some(dir), FolderSource::Environment),
            (None, Some(dir)) => (Some(dir.clone()), FolderSource::Chosen),
            (None, None) => (None, FolderSource::Automatic),
        };
        self.folder_source = source;
        self.reset_for_new_folder();
        // Mit der Umgebungsvariable (Tests, Bildschirmfotos) bleiben die
        // echten Ordner außen vor.
        self.found_folders = if source == FolderSource::Environment {
            Vec::new()
        } else {
            AdobeLocations::find_all()
        };

        match AdobeLocations::resolve(wanted.as_deref()) {
            Ok(locations) => {
                self.camera_raw_dir = Some(locations.camera_raw_dir().to_path_buf());
                // Der alte Thread endet, sobald seine Verbindung wegfällt.
                let worker = Worker::start(ctx.clone(), locations);
                worker.send(Command::LoadLibrary);
                self.worker = Some(worker);
            }
            Err(error) => {
                self.camera_raw_dir = wanted;
                self.fatal = Some(texts::describe(&error));
            }
        }
    }

    /// Vergisst alles, was zum bisherigen Camera-Raw-Ordner gehörte.
    fn reset_for_new_folder(&mut self) {
        self.worker = None;
        self.fatal = None;
        self.library = None;
        self.problems.clear();
        self.place = None;
        self.selection.clear();
        self.anchor = None;
        self.rename = None;
        self.confirm = None;
        self.import = None;
        self.scanning = false;
        self.managing = false;
        self.landing = None;
        self.reselect.clear();
        self.rename_after_load = None;
    }

    /// Läuft gerade etwas, das ein Ordnerwechsel abschneiden würde?
    pub fn busy(&self) -> bool {
        self.scanning || self.managing || self.import.is_some()
    }

    /// Übernimmt einen anderen Camera-Raw-Ordner (`None` = automatisch).
    ///
    /// Ein gewählter Ordner wird vorher geprüft; wer versehentlich den
    /// Unterordner `Settings` wählt, bekommt den Ordner darüber.
    pub(crate) fn choose_camera_raw_dir(
        &mut self,
        ctx: &egui::Context,
        dir: Option<PathBuf>,
        now: f64,
    ) {
        let dir = match dir {
            None => None,
            Some(dir) => match check_camera_raw_dir(&dir) {
                Ok(dir) => Some(dir),
                Err(message) => {
                    self.toast(message, ToastKind::Error, now);
                    return;
                }
            },
        };
        self.settings.camera_raw_dir = dir;
        if let Err(error) = settings::save_settings(&self.settings) {
            self.toast(
                tr_format!(
                    "Einstellung konnte nicht gespeichert werden ({error}).",
                    "The setting could not be saved ({error})."
                ),
                ToastKind::Error,
                now,
            );
        }
        self.connect(ctx);
        if self.fatal.is_none() {
            if let Some(dir) = &self.camera_raw_dir {
                let text = tr_format!(
                    "Camera-Raw-Ordner: {}",
                    "Camera Raw folder: {}",
                    dir.display()
                );
                self.toast(text, ToastKind::Success, now);
            }
        }
    }

    /// Neue Versionen: nachsehen, laden, zum Installieren beenden.
    fn track_updates(&mut self, ctx: &egui::Context, now: f64) {
        if let Some(UpdateEvent::RestartNow) = self.updater.poll(ctx, now, self.settings.updates) {
            self.restart_pending = true;
        }
        // Erst beenden, wenn keine Dateiaktion mehr läuft.
        if self.restart_pending && !self.busy() {
            self.restart_pending = false;
            match self.updater.apply_and_restart() {
                Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(error) => self.toast(
                    tr_format!(
                        "Das Update konnte nicht installiert werden ({error}).",
                        "The update could not be installed ({error})."
                    ),
                    ToastKind::Error,
                    now,
                ),
            }
        }
        if ctx.input(|input| input.viewport().close_requested()) {
            self.updater.on_exit(self.settings.updates);
        }
    }

    /// Einstellung für Updates geändert: speichern und neu nachsehen.
    pub(crate) fn update_settings_changed(&mut self, now: f64) {
        if let Err(error) = settings::save_settings(&self.settings) {
            self.toast(
                tr_format!(
                    "Einstellung konnte nicht gespeichert werden ({error}).",
                    "The setting could not be saved ({error})."
                ),
                ToastKind::Error,
                now,
            );
        }
        self.updater.settings_changed(&self.settings, now);
    }

    /// Merkt sich die Vergrößerung – auch wenn sie mit Strg+Plus/Minus
    /// geändert wurde – und speichert sie kurz danach.
    fn track_zoom(&mut self, ctx: &egui::Context, now: f64) {
        let zoom = ctx.zoom_factor();
        if (zoom - self.settings.zoom).abs() > 0.001 {
            self.settings.zoom = zoom;
            self.zoom_save_at = Some(now + ZOOM_SAVE_DELAY);
        }
        if let Some(at) = self.zoom_save_at {
            if now >= at {
                self.zoom_save_at = None;
                let _ = settings::save_settings(&self.settings);
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(at - now));
            }
        }
    }

    pub(crate) fn send(&self, command: Command) {
        if let Some(worker) = &self.worker {
            worker.send(command);
        }
    }

    // -----------------------------------------------------------------------
    // Ereignisse aus dem Hintergrund
    // -----------------------------------------------------------------------

    fn handle_events(&mut self, now: f64) {
        let events = self.worker.as_ref().map(Worker::poll).unwrap_or_default();
        for event in events {
            match event {
                Event::Library { library, report } => self.on_library(library, report),
                Event::PlanReady { plan, rejected } => {
                    self.scanning = false;
                    for (path, reason) in &rejected {
                        self.toast(
                            tr_format!(
                                "{} wurde abgelehnt: {reason}",
                                "{} was rejected: {reason}",
                                file_name(path)
                            ),
                            ToastKind::Error,
                            now,
                        );
                    }
                    self.import = Some(ImportDialog {
                        plan: Some(plan),
                        rejected,
                        policy: ConflictPolicy::Skip,
                    });
                }
                Event::ScanFailed { rejected } => {
                    self.scanning = false;
                    for (_path, reason) in rejected {
                        self.toast(reason, ToastKind::Error, now);
                    }
                }
                Event::ImportDone {
                    report,
                    backup_folder,
                } => {
                    self.import = None;
                    let (done, skipped, failed) = (
                        report.done_count(),
                        report.skipped_count(),
                        report.failed_count(),
                    );
                    let kind = if failed > 0 {
                        ToastKind::Error
                    } else {
                        ToastKind::Success
                    };
                    let mut text = texts::import_result(done, skipped, failed);
                    if done > 0 {
                        text.push_str(tr(
                            " – Photoshop/Lightroom neu starten, damit sie erscheinen",
                            " – restart Photoshop/Lightroom to see them",
                        ));
                        self.hint_dismissed = false;
                    }
                    self.toast(text, kind, now);
                    if let Some((_, ItemOutcome::Failed(error))) = report
                        .items()
                        .iter()
                        .find(|(_, outcome)| matches!(outcome, ItemOutcome::Failed(_)))
                    {
                        self.toast(texts::describe(error), ToastKind::Error, now);
                    }
                    if let Some(folder) = backup_folder {
                        self.toast(
                            tr_format!(
                                "Ersetzte Dateien gesichert in {}",
                                "Replaced files backed up in {}",
                                folder.display()
                            ),
                            ToastKind::Info,
                            now,
                        );
                    }
                }
                Event::Managed {
                    message,
                    failures,
                    reselect,
                } => {
                    if let Some(message) = message {
                        self.toast(message, ToastKind::Success, now);
                        self.hint_dismissed = false;
                    }
                    let failed = failures.len();
                    for failure in failures.into_iter().take(3) {
                        self.toast(failure, ToastKind::Error, now);
                    }
                    if failed > 3 {
                        self.toast(
                            tr_format!(
                                "… und {} weitere Fehler",
                                "… and {} more errors",
                                failed - 3
                            ),
                            ToastKind::Error,
                            now,
                        );
                    }
                    self.reselect = reselect;
                }
                Event::Error(message) => {
                    self.scanning = false;
                    self.managing = false;
                    if let Some(dialog) = &self.import {
                        if dialog.plan.is_none() {
                            self.import = None;
                        }
                    }
                    self.toast(message, ToastKind::Error, now);
                }
                Event::AdobeApps(apps) => self.adobe_apps = apps,
            }
        }
    }

    fn on_library(&mut self, library: Library, report: preset_core::report::OperationReport) {
        self.problems = report
            .into_items()
            .into_iter()
            .filter_map(|(path, outcome)| match outcome {
                ItemOutcome::Failed(error) => Some((path, texts::describe(&error))),
                _ => None,
            })
            .collect();

        // Leere Gruppen, in denen inzwischen etwas liegt, sind keine mehr.
        let before = self.empty_groups.len();
        self.empty_groups
            .retain(|(section, name)| library.group(*section, name).is_none());
        if self.empty_groups.len() != before {
            let _ = settings::save_empty_groups(&self.empty_groups);
        }

        self.library = Some(library);
        self.managing = false;

        if let Some(landing) = self.landing.take() {
            self.place = Some(landing);
            self.search.clear();
            if !self
                .filter
                .shows(self.place_section().unwrap_or(Section::Presets))
            {
                self.filter = Filter::All;
            }
        }
        if !self.reselect.is_empty() {
            self.selection = std::mem::take(&mut self.reselect).into_iter().collect();
            self.anchor = self.selection.iter().next().cloned();
        }
        if let Some((section, name)) = self.rename_after_load.take() {
            if self.group_exists(section, &name) {
                self.place = Some(Place::Group {
                    section,
                    name: name.clone(),
                });
                self.rename = Some(Rename {
                    text: name.clone(),
                    target: RenameTarget::Group { section, name },
                    focus: true,
                });
            }
        }
        self.ensure_valid_place();
    }

    // -----------------------------------------------------------------------
    // Abfragen für die Darstellung
    // -----------------------------------------------------------------------

    pub fn place_section(&self) -> Option<Section> {
        match &self.place {
            Some(Place::Group { section, .. }) => Some(*section),
            _ => None,
        }
    }

    pub fn searching(&self) -> bool {
        !self.search.trim().is_empty()
    }

    /// Die Einträge, die die Liste in der Mitte zeigt: Suchtreffer oder die
    /// Einträge der gewählten Gruppe.
    pub fn listed(&self) -> Vec<&Preset> {
        let Some(library) = &self.library else {
            return Vec::new();
        };
        let query = self.search.trim().to_lowercase();
        if !query.is_empty() {
            return library
                .groups()
                .iter()
                .filter(|group| self.filter.shows(group.section))
                .flat_map(|group| &group.presets)
                .filter(|preset| preset.name.to_lowercase().contains(&query))
                .collect();
        }
        match &self.place {
            Some(Place::Group { section, name }) => {
                self.group_presets(*section, name, self.place_active)
            }
            _ => Vec::new(),
        }
    }

    /// Die Einträge einer Gruppe in einem Zustand (aktiv/deaktiviert).
    pub fn group_presets(&self, section: Section, name: &str, active: bool) -> Vec<&Preset> {
        self.library
            .as_ref()
            .and_then(|library| library.group(section, name))
            .map(|group| {
                group
                    .presets
                    .iter()
                    .filter(|preset| preset.active == active)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Die Gruppen eines Bereichs, die Einträge in diesem Zustand haben –
    /// bei den aktiven auch die neuen, leeren Gruppen.
    pub fn group_names_in_state(&self, section: Section, active: bool) -> Vec<String> {
        self.group_names(section)
            .into_iter()
            .filter(|name| {
                (active && self.is_empty_group(section, name))
                    || !self.group_presets(section, name, active).is_empty()
            })
            .collect()
    }

    /// Gibt es deaktivierte Einträge (in den angezeigten Bereichen)?
    pub fn has_inactive(&self) -> bool {
        self.all_presets()
            .any(|preset| !preset.active && self.filter.shows(preset.section()))
    }

    /// Alle Einträge der Bibliothek.
    pub fn all_presets(&self) -> impl Iterator<Item = &Preset> {
        self.library
            .iter()
            .flat_map(|library| library.groups())
            .flat_map(|group| &group.presets)
    }

    /// Die ausgewählten Einträge in Listenreihenfolge.
    pub fn selected_presets(&self) -> Vec<Preset> {
        self.all_presets()
            .filter(|preset| self.selection.contains(&preset.path))
            .cloned()
            .collect()
    }

    /// Der einzige ausgewählte Eintrag, falls genau einer gewählt ist.
    pub fn single_selected(&self) -> Option<&Preset> {
        if self.selection.len() != 1 {
            return None;
        }
        let path = self.selection.iter().next()?;
        self.all_presets().find(|preset| &preset.path == path)
    }

    /// Gibt es diese Gruppe – in den Dateien oder als neue, leere Gruppe?
    pub fn group_exists(&self, section: Section, name: &str) -> bool {
        self.is_empty_group(section, name)
            || self
                .library
                .as_ref()
                .is_some_and(|library| library.group(section, name).is_some())
    }

    pub fn is_empty_group(&self, section: Section, name: &str) -> bool {
        self.empty_groups
            .iter()
            .any(|(s, n)| *s == section && n == name)
    }

    /// Alle Gruppennamen eines Bereichs, sortiert – für Auswahllisten.
    pub fn group_names(&self, section: Section) -> Vec<String> {
        let mut names: Vec<String> = self
            .library
            .iter()
            .flat_map(|library| library.groups_in(section))
            .map(|group| group.name.clone())
            .chain(
                self.empty_groups
                    .iter()
                    .filter(|(s, _)| *s == section)
                    .map(|(_, name)| name.clone()),
            )
            .collect();
        names.sort_by_key(|name| preset_core::library::scan::collation_key(name));
        names.dedup();
        names
    }

    /// Sind gerade Adobe-Programme offen und der Hinweis nicht weggeklickt?
    pub fn show_running_hint(&self) -> bool {
        !self.adobe_apps.is_empty() && !self.hint_dismissed
    }

    /// Wählt eine sichtbare Gruppe, wenn die bisherige fehlt oder ausgefiltert ist.
    pub(crate) fn ensure_valid_place(&mut self) {
        // Ist die Gruppe im angezeigten Zustand leer geworden (z. B. ganz
        // deaktiviert), aber im anderen Zustand noch da, wechselt die Ansicht.
        if let Some(Place::Group { section, name }) = &self.place {
            let here = self.place_active && self.is_empty_group(*section, name)
                || !self
                    .group_presets(*section, name, self.place_active)
                    .is_empty();
            let there = !self
                .group_presets(*section, name, !self.place_active)
                .is_empty();
            if !here && there {
                self.place_active = !self.place_active;
            }
        }
        let valid = match &self.place {
            Some(Place::Group { section, name }) => {
                self.filter.shows(*section)
                    && self
                        .group_names_in_state(*section, self.place_active)
                        .contains(name)
            }
            Some(Place::Problems) => !self.problems.is_empty(),
            None => false,
        };
        if !valid {
            let first = [true, false].into_iter().find_map(|active| {
                [Section::Presets, Section::Profiles]
                    .into_iter()
                    .filter(|section| self.filter.shows(*section))
                    .find_map(|section| {
                        self.group_names_in_state(section, active)
                            .into_iter()
                            .next()
                            .map(|name| (Place::Group { section, name }, active))
                    })
            });
            match first {
                Some((place, active)) => {
                    self.place = Some(place);
                    self.place_active = active;
                }
                None => {
                    self.place = None;
                    self.place_active = true;
                }
            }
        }
        // Auswahl auf Einträge beschränken, die es noch gibt.
        let existing: BTreeSet<PathBuf> = self
            .all_presets()
            .filter(|preset| self.selection.contains(&preset.path))
            .map(|preset| preset.path.clone())
            .collect();
        self.selection = existing;
    }

    pub fn toast(&mut self, text: String, kind: ToastKind, now: f64) {
        let seconds = if kind == ToastKind::Error {
            ERROR_TOAST_SECONDS
        } else {
            TOAST_SECONDS
        };
        self.toasts.push(Toast {
            text,
            kind,
            until: now + seconds,
        });
    }

    /// Ordner/ZIPs, die ins Fenster gezogen wurden (F-01, F-02). Landen sie
    /// auf einer Gruppe links, werden sie in diese Gruppe importiert (F-12).
    fn handle_dropped_files(&mut self, ctx: &egui::Context, now: f64) {
        let (paths, pointer) = ctx.input(|input| {
            let paths: Vec<PathBuf> = input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .filter(|path| !path.as_os_str().is_empty())
                .collect();
            (paths, input.pointer.latest_pos())
        });
        if paths.is_empty() {
            return;
        }
        let target = pointer.and_then(|pos| self.group_under(pos));
        self.start_import(paths, target, now);
    }

    /// Die Gruppe links, über der `pos` liegt.
    pub fn group_under(&self, pos: egui::Pos2) -> Option<Place> {
        self.group_rects
            .iter()
            .find(|(place, rect)| rect.contains(pos) && matches!(place, Place::Group { .. }))
            .map(|(place, _)| place.clone())
    }

    fn check_adobe(&mut self, ctx: &egui::Context, now: f64) {
        if now >= self.next_adobe_check {
            self.next_adobe_check = now + ADOBE_CHECK_SECONDS;
            self.send(Command::CheckAdobe);
        }
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(ADOBE_CHECK_SECONDS));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|input| input.time);

        self.track_zoom(&ctx, now);
        self.track_updates(&ctx, now);
        if let Some(message) = self.fatal.clone() {
            ui::overlays::fatal(ui, self, &message, now);
            ui::overlays::toasts(&ctx, self, now);
            return;
        }

        self.handle_events(now);
        self.check_adobe(&ctx, now);
        self.handle_dropped_files(&ctx, now);
        if self.import.is_none() && self.confirm.is_none() && !self.settings_open {
            self.handle_shortcuts(&ctx, now);
        }
        self.drag_hint = None;

        ui::topbar::show(ui, self, now);
        ui::overlays::update_bar(ui, self);
        ui::overlays::hint_bar(ui, self);
        ui::groups::show(ui, self, now);
        ui::details::show(ui, self, now);
        ui::list::show(ui, self, now);
        ui::import_dialog::show(&ctx, self);
        ui::confirm_dialog::show(&ctx, self, now);
        ui::settings_dialog::show(&ctx, self, now);
        ui::overlays::drag_feedback(&ctx, self);
        ui::overlays::drop_overlay(&ctx, self);
        ui::overlays::busy(&ctx, self);
        ui::overlays::toasts(&ctx, self, now);
    }
}

/// Öffnet den Explorer und markiert die Datei.
pub fn show_in_explorer(path: &Path) -> Result<(), String> {
    let mut command = std::process::Command::new("explorer");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Der Explorer erwartet `/select,"Pfad"` als ein einziges Argument
        // mit Anführungszeichen nur um den Pfad.
        command.raw_arg(format!("/select,\"{}\"", path.display()));
    }
    #[cfg(not(windows))]
    {
        command.arg(path.parent().unwrap_or(path));
    }
    command.spawn().map(|_| ()).map_err(|error| {
        tr_format!(
            "Explorer konnte nicht geöffnet werden ({error}).",
            "Explorer could not be opened ({error})."
        )
    })
}

/// Öffnet einen Ordner im Explorer; fehlt er, wird er angelegt.
pub fn open_folder(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|error| {
        tr_format!(
            "{} konnte nicht angelegt werden ({error}).",
            "{} could not be created ({error}).",
            path.display()
        )
    })?;
    std::process::Command::new("explorer")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            tr_format!(
                "Explorer konnte nicht geöffnet werden ({error}).",
                "Explorer could not be opened ({error})."
            )
        })
}

/// Prüft einen gewählten Camera-Raw-Ordner. Wurde der Unterordner `Settings`
/// gewählt, gilt der Ordner darüber.
fn check_camera_raw_dir(dir: &Path) -> Result<PathBuf, String> {
    let first = match AdobeLocations::resolve(Some(dir)) {
        Ok(_) => return Ok(dir.to_path_buf()),
        Err(error) => error,
    };
    let is_settings = dir
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("Settings"));
    if let (true, Some(parent)) = (is_settings, dir.parent()) {
        match AdobeLocations::resolve(Some(parent)) {
            Ok(_) => return Ok(parent.to_path_buf()),
            // Die genauere Meldung zuerst.
            Err(error @ CoreError::AdobeOwnFolder { .. }) => return Err(texts::describe(&error)),
            Err(_) => {}
        }
    }
    Err(texts::describe(&first))
}

/// Ob ein Preset über Adobes Importdialog kam.
pub fn is_adobe_origin(preset: &Preset) -> bool {
    preset.origin == PresetOrigin::ImportedSettings
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
