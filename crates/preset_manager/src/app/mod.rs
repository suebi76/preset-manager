//! Zustand der Anwendung und der Ablauf eines Bildes.
//!
//! [`App`] hält, was die Oberfläche gerade zeigt (gewählte Gruppe, Auswahl,
//! Suche, offene Dialoge …). Die eigentliche Arbeit macht `preset_core` im
//! Hintergrund-Thread ([`crate::worker`]); die Module unter [`crate::ui`]
//! zeichnen nur. Was der Nutzer auslösen kann, steht in [`crate::actions`].

use std::collections::BTreeSet;
use std::path::PathBuf;

use eframe::egui;
use preset_core::adobe::{AdobeApp, AdobeLocations, FoundFolder};
use preset_core::library::Library;
use preset_core::preset::Section;

use crate::settings;
use crate::texts::{self, tr_format};
use crate::ui;
use crate::updates::{UpdateEvent, Updater};
use crate::worker::{Command, Worker};

/// Wie oft nachgesehen wird, ob Photoshop oder Lightroom läuft.
const ADOBE_CHECK_SECONDS: f64 = 5.0;
/// Wie lange eine Einblendung stehen bleibt.
const TOAST_SECONDS: f64 = 5.0;
const ERROR_TOAST_SECONDS: f64 = 12.0;
/// Eine Einblendung mit „Rückgängig“ bleibt länger – man soll den Knopf
/// erreichen können.
const UNDO_TOAST_SECONDS: f64 = 10.0;
/// Umgebungsvariable für einen anderen Camera-Raw-Ordner (F-43). Sie geht
/// der Einstellung vor – gedacht für Tests mit einer Kopie.
pub const CUSTOM_DIR_VARIABLE: &str = "APM_CAMERA_RAW_DIR";
/// So lange wird nach einer Zoom-Änderung gewartet, bevor sie gespeichert
/// wird – beim Ziehen am Regler nicht bei jedem Bild.
const ZOOM_SAVE_DELAY: f64 = 0.6;
mod events;
mod files;
mod package_paths;
mod queries;
mod types;

pub use files::{file_name, is_adobe_origin, open_folder, show_in_explorer};
pub use types::{
    Confirm, DragPayload, Filter, FolderSource, ImportDialog, Place, Rename, RenameTarget, Toast,
    ToastKind,
};

use files::check_camera_raw_dir;

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
    /// Was „Rückgängig“ zurücknehmen würde (F-29) – `None`: nichts.
    pub undo_label: Option<String>,
    /// Was „Wiederholen“ wiederholen würde.
    pub redo_label: Option<String>,
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
    /// übergeben wurden (z. B. auf die .exe gezogen).
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
            undo_label: None,
            redo_label: None,
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
        // Der neue Hintergrund-Thread beginnt mit leerer Geschichte.
        self.undo_label = None;
        self.redo_label = None;
        for toast in &mut self.toasts {
            toast.undo = false;
        }
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
            undo: false,
        });
    }

    /// Erfolgsmeldung einer Aktion, die sich zurücknehmen lässt: mit
    /// „Rückgängig“ in der Einblendung. Nur die jüngste behält den Knopf –
    /// er nimmt immer die letzte Aktion zurück.
    pub fn toast_undoable(&mut self, text: String, now: f64) {
        for toast in &mut self.toasts {
            toast.undo = false;
        }
        self.toast(text, ToastKind::Success, now);
        if let Some(toast) = self.toasts.last_mut() {
            toast.undo = true;
            toast.until = now + UNDO_TOAST_SECONDS;
        }
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
