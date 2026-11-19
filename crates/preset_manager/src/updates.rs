//! Neue Versionen aus den GitHub-Releases – über Velopack, wie in
//! CloudDrive-Sync.
//!
//! - Nur das mit `PresetManager-Setup.exe` installierte Programm (oder die
//!   portable Fassung) aktualisiert sich selbst; eine mit `cargo run`
//!   gebaute Kopie nicht.
//! - Kurz nach dem Start und danach alle sechs Stunden wird nachgesehen
//!   (nicht bei „Nur wenn ich nachsehe“).
//! - „Hinweis zeigen“: Hinweisleiste, installiert wird mit einem Klick.
//!   „Automatisch“: im Hintergrund laden, beim Schließen installieren.
//! - Testversionen sind Vorabversionen auf GitHub. Wer eine Testversion
//!   benutzt, bekommt weiter Testversionen, bis eine neuere reguläre da ist.
//!
//! Alles, was das Netz braucht, läuft in eigenen Threads; die Oberfläche
//! fragt nur in jedem Bild nach, ob etwas angekommen ist.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;
use velopack::sources::{FileSource, GithubSource};
use velopack::{UpdateCheck, UpdateInfo, UpdateManager};

use crate::settings::{Settings, UpdateMode};
use crate::texts::{self, tr, tr_format};

/// Das GitHub-Projekt, dessen Releases die Updates liefern.
pub const REPOSITORY_URL: &str = "https://github.com/suebi76/preset-manager";
/// Zum Ausprobieren: ein Ordner mit selbst gepackten Versionen statt GitHub
/// (`scripts/New-Release.ps1 -OutputDir …`).
pub const LOCAL_FEED_VARIABLE: &str = "APM_UPDATE_DIR";
/// Erster Blick nach dem Start – nicht sofort, damit der Start nicht wartet.
const FIRST_CHECK_SECONDS: f64 = 3.0;
const PERIOD_SECONDS: f64 = 6.0 * 60.0 * 60.0;

/// Was der Einstellungsdialog und die Hinweisleiste anzeigen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Nicht mit dem Installationsprogramm eingerichtet.
    NotInstalled,
    Idle,
    Checking,
    UpToDate,
    Available(String),
    Downloading {
        version: String,
        percent: i16,
    },
    /// Geladen; wird beim nächsten Neustart bzw. Schließen installiert.
    Ready(String),
    Failed(String),
}

/// Was die App tun muss.
pub enum UpdateEvent {
    /// Das Update ist geladen und soll jetzt installiert werden: App beenden.
    RestartNow,
}

enum Message {
    Checked(Result<Option<Box<UpdateInfo>>, String>),
    Progress(i16),
    Downloaded(Result<(), String>),
}

pub struct Updater {
    manager: Option<UpdateManager>,
    test_versions: bool,
    pub status: Status,
    info: Option<Box<UpdateInfo>>,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    next_check: Option<f64>,
    /// Nach dem Laden gleich installieren („Jetzt installieren“).
    install_when_ready: bool,
    /// Der Updater wurde schon beauftragt; nicht ein zweites Mal.
    applying: bool,
    /// Version, deren Hinweis weggeklickt wurde.
    pub dismissed: Option<String>,
    /// Die laufende Version – beim installierten Programm die des Pakets.
    pub current_version: String,
}

impl Updater {
    pub fn new(settings: &Settings) -> Self {
        let (sender, receiver) = mpsc::channel();
        let manager = make_manager(settings.test_versions);
        let current_version = manager.as_ref().map_or_else(
            || env!("CARGO_PKG_VERSION").to_owned(),
            UpdateManager::get_current_version_as_string,
        );
        let installed = manager.is_some();
        tracing::info!(
            "Updates: Version {current_version}, {}",
            if installed {
                "installiert"
            } else {
                "nicht installiert – keine Selbstaktualisierung"
            }
        );
        Self {
            manager,
            test_versions: settings.test_versions,
            status: if installed {
                Status::Idle
            } else {
                Status::NotInstalled
            },
            info: None,
            sender,
            receiver,
            next_check: (installed && settings.updates != UpdateMode::Manual)
                .then_some(FIRST_CHECK_SECONDS),
            install_when_ready: false,
            applying: false,
            dismissed: None,
            current_version,
        }
    }

    pub fn is_installed(&self) -> bool {
        self.manager.is_some()
    }

    /// Läuft gerade eine Suche oder ein Download?
    pub fn busy(&self) -> bool {
        matches!(self.status, Status::Checking | Status::Downloading { .. })
    }

    /// Die gefundene Version, falls es eine gibt.
    pub fn found_version(&self) -> Option<&str> {
        match &self.status {
            Status::Available(version) | Status::Ready(version) => Some(version),
            Status::Downloading { version, .. } => Some(version),
            _ => None,
        }
    }

    /// Text für den Einstellungsdialog.
    pub fn status_text(&self) -> String {
        let current = &self.current_version;
        match &self.status {
            Status::NotInstalled => tr_format!(
                "Version {current} – diese Kopie wurde nicht mit dem Installationsprogramm \
                 eingerichtet und aktualisiert sich nicht selbst.",
                "Version {current} – this copy was not set up with the installer \
                 and does not update itself."
            ),
            Status::Idle => format!("Version {current}"),
            Status::Checking => tr(
                "Sucht nach einer neuen Version …",
                "Looking for a new version …",
            )
            .to_owned(),
            Status::UpToDate => tr_format!(
                "Version {current} – das ist die neueste.",
                "Version {current} – this is the latest."
            ),
            Status::Available(version) => tr_format!(
                "Version {version} ist verfügbar (installiert: {current}).",
                "Version {version} is available (installed: {current})."
            ),
            Status::Downloading { version, percent } => tr_format!(
                "Lädt Version {version} … {percent} %",
                "Downloading version {version} … {percent} %"
            ),
            Status::Ready(version) => tr_format!(
                "Version {version} ist geladen.",
                "Version {version} is downloaded."
            ),
            Status::Failed(reason) => reason.clone(),
        }
    }

    /// Einstellung geändert: vergessen, was gefunden wurde, und bald neu
    /// nachsehen. Andere Testversionen-Wahl heißt andere Quelle.
    pub fn settings_changed(&mut self, settings: &Settings, now: f64) {
        if self.busy() || self.applying {
            return;
        }
        if settings.test_versions != self.test_versions {
            self.test_versions = settings.test_versions;
            self.manager = make_manager(settings.test_versions);
        }
        if self.manager.is_none() {
            self.status = Status::NotInstalled;
            return;
        }
        self.info = None;
        self.install_when_ready = false;
        self.status = Status::Idle;
        self.next_check = (settings.updates != UpdateMode::Manual).then_some(now + 2.0);
    }

    /// In jedem Bild: Rückmeldungen abholen und zur rechten Zeit nachsehen.
    pub fn poll(&mut self, ctx: &egui::Context, now: f64, mode: UpdateMode) -> Option<UpdateEvent> {
        let mut event = None;
        while let Ok(message) = self.receiver.try_recv() {
            if let Some(found) = self.handle(ctx, message, mode) {
                event = Some(found);
            }
        }
        if let Some(at) = self.next_check {
            if mode == UpdateMode::Manual {
                self.next_check = None;
            } else if now >= at {
                self.next_check = Some(now + PERIOD_SECONDS);
                self.check(ctx);
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(at - now));
            }
        }
        event
    }

    /// „Nach Updates suchen“.
    pub fn check(&mut self, ctx: &egui::Context) {
        let Some(manager) = self.manager.clone() else {
            return;
        };
        if self.busy() || self.applying {
            return;
        }
        self.status = Status::Checking;
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        let spawned = thread::Builder::new()
            .name("update-check".to_owned())
            .spawn(move || {
                let result = match manager.check_for_updates() {
                    Ok(UpdateCheck::UpdateAvailable(info)) => Ok(Some(info)),
                    Ok(UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty) => Ok(None),
                    Err(error) => Err(error.to_string()),
                };
                let _ = sender.send(Message::Checked(result));
                ctx.request_repaint();
            });
        if spawned.is_err() {
            self.status = Status::Failed(texts::update_unreachable().to_owned());
        }
    }

    /// „Jetzt installieren“: laden, falls nötig, dann neu starten.
    pub fn install_now(&mut self, ctx: &egui::Context) -> Option<UpdateEvent> {
        match self.status {
            Status::Ready(_) => Some(UpdateEvent::RestartNow),
            Status::Available(_) => {
                self.install_when_ready = true;
                self.download(ctx);
                None
            }
            _ => None,
        }
    }

    /// Beauftragt den Velopack-Updater: Er wartet, bis die App beendet ist,
    /// installiert und startet sie neu. Danach muss die App sich schließen.
    pub fn apply_and_restart(&mut self) -> Result<(), String> {
        self.apply(false, true)
    }

    /// Beim Schließen der App: im Modus „Automatisch“ ein geladenes Update
    /// still installieren, ohne neu zu starten.
    pub fn on_exit(&mut self, mode: UpdateMode) {
        if mode == UpdateMode::Automatic && matches!(self.status, Status::Ready(_)) {
            if let Err(error) = self.apply(true, false) {
                tracing::warn!("Update beim Schließen nicht installiert: {error}");
            }
        }
    }

    fn apply(&mut self, silent: bool, restart: bool) -> Result<(), String> {
        if self.applying {
            return Ok(());
        }
        let (Some(manager), Some(info)) = (&self.manager, &self.info) else {
            return Err(tr(
                "Es ist kein geladenes Update da.",
                "No update has been downloaded.",
            )
            .to_owned());
        };
        manager
            .wait_exit_then_apply_updates(info.as_ref(), silent, restart, Vec::<String>::new())
            .map_err(|error| error.to_string())?;
        self.applying = true;
        tracing::info!(
            "Update {} wird nach dem Beenden installiert",
            info.TargetFullRelease.Version
        );
        Ok(())
    }

    fn handle(
        &mut self,
        ctx: &egui::Context,
        message: Message,
        mode: UpdateMode,
    ) -> Option<UpdateEvent> {
        match message {
            Message::Checked(Ok(Some(info))) => {
                let version = info.TargetFullRelease.Version.clone();
                tracing::info!("Update: Version {version} ist verfügbar");
                let same = self
                    .info
                    .as_ref()
                    .is_some_and(|known| known.TargetFullRelease.Version == version);
                if !(same && matches!(self.status, Status::Ready(_))) {
                    self.info = Some(info);
                    self.status = Status::Available(version);
                    if mode == UpdateMode::Automatic {
                        self.download(ctx);
                    }
                } else {
                    self.status = Status::Ready(version);
                }
                None
            }
            Message::Checked(Ok(None)) => {
                self.status = Status::UpToDate;
                None
            }
            Message::Checked(Err(error)) => {
                // GitHub nicht erreichbar oder Abfragegrenze erreicht: beim nächsten Mal.
                tracing::info!("Update-Suche gescheitert: {error}");
                self.status = Status::Failed(texts::update_unreachable().to_owned());
                None
            }
            Message::Progress(percent) => {
                if let Status::Downloading { percent: shown, .. } = &mut self.status {
                    *shown = percent;
                }
                None
            }
            Message::Downloaded(Ok(())) => {
                let version = self.found_version().unwrap_or_default().to_owned();
                tracing::info!("Update {version} geladen");
                self.status = Status::Ready(version);
                std::mem::take(&mut self.install_when_ready).then_some(UpdateEvent::RestartNow)
            }
            Message::Downloaded(Err(error)) => {
                let version = self.found_version().unwrap_or_default().to_owned();
                tracing::warn!("Update {version} nicht geladen: {error}");
                self.install_when_ready = false;
                self.status = Status::Failed(tr_format!(
                    "Version {version} konnte nicht geladen werden. Preset Manager versucht es später erneut.",
                    "Version {version} could not be downloaded. Preset Manager will try again later."
                ));
                None
            }
        }
    }

    fn download(&mut self, ctx: &egui::Context) {
        let (Some(manager), Some(info)) = (self.manager.clone(), self.info.clone()) else {
            return;
        };
        let version = info.TargetFullRelease.Version.clone();
        self.status = Status::Downloading {
            version,
            percent: 0,
        };
        let sender = self.sender.clone();
        let (progress_sender, progress) = mpsc::channel::<i16>();
        // Fortschritt weiterreichen, bis der Download fertig ist.
        let forward = sender.clone();
        let forward_ctx = ctx.clone();
        let _ = thread::Builder::new()
            .name("update-progress".to_owned())
            .spawn(move || {
                for percent in progress {
                    let _ = forward.send(Message::Progress(percent));
                    forward_ctx.request_repaint();
                }
            });
        let ctx = ctx.clone();
        let spawned = thread::Builder::new()
            .name("update-download".to_owned())
            .spawn(move || {
                let result = manager
                    .download_updates(&info, Some(progress_sender))
                    .map_err(|error| error.to_string());
                let _ = sender.send(Message::Downloaded(result));
                ctx.request_repaint();
            });
        if spawned.is_err() {
            self.status = Status::Failed(texts::update_unreachable().to_owned());
        }
    }
}

/// `None`, wenn das Programm nicht installiert ist.
fn make_manager(test_versions: bool) -> Option<UpdateManager> {
    if let Some(dir) = std::env::var_os(LOCAL_FEED_VARIABLE).filter(|value| !value.is_empty()) {
        return UpdateManager::new(FileSource::new(dir), None, None).ok();
    }
    let github = |prerelease: bool| {
        UpdateManager::new(
            GithubSource::new(REPOSITORY_URL, None, prerelease),
            None,
            None,
        )
        .ok()
    };
    let manager = github(test_versions)?;
    // Läuft eine Testversion (maßgeblich ist die installierte Paketversion),
    // bleiben Testversionen sichtbar, bis eine neuere reguläre da ist.
    if !test_versions && manager.get_current_version_as_string().contains('-') {
        return github(true);
    }
    Some(manager)
}
