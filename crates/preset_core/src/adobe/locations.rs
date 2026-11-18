//! Die Ordner, in denen Adobe Presets ablegt.
//!
//! Photoshop, Photoshop (Beta) und Lightroom Classic teilen sich einen einzigen
//! Camera-Raw-Ordner. Hintergrund: `Planung/02-Adobe-Grundlagen.md`.
//!
//! | Ordner | Was dort liegt |
//! |---|---|
//! | `%APPDATA%\Adobe\CameraRaw` | Presets und Profile des Benutzers – der Standard |
//! | `<Katalogordner>\Lightroom Settings` (Name je nach Sprache, z. B. `Lightroom-Einstellungen`) | Presets von Lightroom Classic, wenn dort „Presets mit diesem Katalog speichern“ an ist; Photoshop liest sie nicht |
//! | `%ProgramData%\Adobe\CameraRaw` | Adobes mitgelieferte Presets und Profile – tabu |
//! | `<Camera-Raw-Ordner>\Preset Manager (inactive)` | Deaktivierte Presets der App ([`INACTIVE_FOLDER`]); Adobe liest den Ordner nicht |
//!
//! Dies ist die einzige Stelle im Projekt, die Adobe-Pfade kennt. Alle anderen
//! Module bekommen ein [`AdobeLocations`] übergeben. Dadurch können Tests mit
//! einem temporären Ordner arbeiten und fassen nie echte Presets an.

use std::path::{Component, Path, PathBuf, Prefix};

use super::lightroom;
use crate::error::CoreError;

/// Der Ordner im Camera-Raw-Ordner, in dem deaktivierte Presets liegen.
///
/// Camera Raw liest nur bekannte Unterordner (`Settings`, `ImportedSettings`
/// …), diesen also nicht. Er liegt im selben Camera-Raw-Ordner, damit
/// Deaktivieren und Aktivieren reines Umbenennen auf demselben Laufwerk ist
/// und die Presets beim Wechsel des Camera-Raw-Ordners dort bleiben, wo sie
/// hingehören. Darin spiegeln `Settings` und `ImportedSettings` die
/// ursprüngliche Lage – Versuch P-7 in `Planung/02-Adobe-Grundlagen.md`.
pub const INACTIVE_FOLDER: &str = "Preset Manager (inactive)";

/// Die Systemordner, von denen aus gesucht wird. Im Programm aus den
/// Umgebungsvariablen ([`SearchRoots::from_environment`]), in Tests ein
/// temporärer Ordner.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchRoots {
    /// `%APPDATA%`, z. B. `C:\Users\Anna\AppData\Roaming`.
    pub app_data: Option<PathBuf>,
    /// `%ProgramData%`, z. B. `C:\ProgramData`.
    pub program_data: Option<PathBuf>,
}

impl SearchRoots {
    /// Liest `%APPDATA%` und `%ProgramData%`; leere Werte zählen als fehlend.
    pub fn from_environment() -> Self {
        let variable = |name: &str| {
            std::env::var_os(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        Self {
            app_data: variable("APPDATA"),
            program_data: variable("ProgramData"),
        }
    }

    /// `%APPDATA%\Adobe\CameraRaw`.
    fn user_dir(&self) -> Option<PathBuf> {
        Some(self.app_data.as_ref()?.join("Adobe").join("CameraRaw"))
    }

    /// `%ProgramData%\Adobe\CameraRaw` – Adobes mitgelieferte Presets.
    fn adobe_own_dir(&self) -> Option<PathBuf> {
        Some(self.program_data.as_ref()?.join("Adobe").join("CameraRaw"))
    }

    /// Die zuletzt geänderte Einstellungsdatei von Lightroom Classic, z. B.
    /// `Lightroom Classic CC 7 Preferences.agprefs`. Die „Startup“-Datei
    /// enthält nur Fensterlagen und zählt nicht.
    fn lightroom_preferences_file(&self) -> Option<PathBuf> {
        let dir = self
            .app_data
            .as_ref()?
            .join("Adobe")
            .join("Lightroom")
            .join("Preferences");
        std::fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                name.ends_with("preferences.agprefs") && !name.contains("startup")
            })
            .filter_map(|entry| {
                let modified = entry.metadata().ok()?.modified().ok()?;
                Some((modified, entry.path()))
            })
            .max_by_key(|(modified, _)| *modified)
            .map(|(_, path)| path)
    }
}

/// Woher ein gefundener Camera-Raw-Ordner stammt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderKind {
    /// `%APPDATA%\Adobe\CameraRaw` – gilt für Photoshop, Photoshop (Beta) und
    /// Lightroom Classic.
    User,
    /// Der Ordner neben einem Lightroom-Classic-Katalog. Nur Lightroom Classic
    /// liest ihn, und nur bei „Presets mit diesem Katalog speichern“.
    LightroomCatalog {
        /// Der Katalog (`.lrcat`).
        catalog: PathBuf,
        /// Die Einstellung ist in Lightroom gerade eingeschaltet. Dann sieht
        /// Lightroom Classic die Presets im Benutzerordner nicht.
        in_use: bool,
    },
}

/// Ein Camera-Raw-Ordner, den [`AdobeLocations::find_all`] gefunden hat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundFolder {
    /// Woher er stammt.
    pub kind: FolderKind,
    /// Der Ordner, in dem `Settings` usw. liegen.
    pub camera_raw_dir: PathBuf,
}

/// Die Preset-Ordner einer Camera-Raw-Installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdobeLocations {
    /// Presets, die der Nutzer selbst angelegt oder hineinkopiert hat.
    /// Hierhin installiert die App.
    pub settings: PathBuf,
    /// Presets, die über Adobes eigenen Importdialog kamen.
    /// Die App liest und verwaltet sie, installiert aber nicht hierhin.
    pub imported_settings: PathBuf,
    /// Kameraprofile (`.dcp`).
    pub camera_profiles: PathBuf,
    /// Deaktivierte Presets aus `Settings` (gleicher Unterpfad).
    pub inactive_settings: PathBuf,
    /// Deaktivierte Presets aus `ImportedSettings` (gleicher Unterpfad).
    pub inactive_imported_settings: PathBuf,
}

impl AdobeLocations {
    /// Leitet die Unterordner aus einem Camera-Raw-Ordner ab.
    ///
    /// Prüft nicht, ob die Ordner existieren. Gedacht für Tests; im Programm
    /// [`AdobeLocations::resolve`] verwenden.
    pub fn from_camera_raw_dir(camera_raw_dir: &Path) -> Self {
        Self {
            settings: camera_raw_dir.join("Settings"),
            imported_settings: camera_raw_dir.join("ImportedSettings"),
            camera_profiles: camera_raw_dir.join("CameraProfiles"),
            inactive_settings: camera_raw_dir.join(INACTIVE_FOLDER).join("Settings"),
            inactive_imported_settings: camera_raw_dir
                .join(INACTIVE_FOLDER)
                .join("ImportedSettings"),
        }
    }

    /// Ermittelt die Ordner: den vom Nutzer gewählten Camera-Raw-Ordner, falls
    /// angegeben, sonst den automatisch gefundenen ([`AdobeLocations::detect`]).
    ///
    /// # Fehler
    ///
    /// - [`CoreError::AdobeFolderNotFound`], wenn der Ordner nicht existiert.
    /// - [`CoreError::AdobeOwnFolder`], wenn der Ordner Adobes mitgelieferte
    ///   Presets enthält (`%ProgramData%\Adobe\CameraRaw` oder darin).
    /// - [`CoreError::NotACameraRawFolder`], wenn der gewählte Ordner keinen
    ///   der Unterordner `Settings`, `ImportedSettings` oder `CameraProfiles`
    ///   enthält – etwa weil `CameraRaw\Settings` statt `CameraRaw` gewählt
    ///   wurde.
    pub fn resolve(custom_camera_raw_dir: Option<&Path>) -> Result<Self, CoreError> {
        Self::resolve_in(custom_camera_raw_dir, &SearchRoots::from_environment())
    }

    /// Wie [`Self::resolve`], aber mit vorgegebenen Systemordnern (für Tests).
    ///
    /// # Fehler
    ///
    /// Wie [`Self::resolve`].
    pub fn resolve_in(
        custom_camera_raw_dir: Option<&Path>,
        roots: &SearchRoots,
    ) -> Result<Self, CoreError> {
        let Some(dir) = custom_camera_raw_dir else {
            return Self::detect_in(roots);
        };

        // Adobes eigene Presets und Profile: umbenennen oder löschen würde
        // das Programm beschädigen, schreiben bräuchte Administratorrechte.
        if let Some(own) = roots.adobe_own_dir() {
            if is_at_or_below(dir, &own) {
                return Err(CoreError::AdobeOwnFolder {
                    path: dir.to_path_buf(),
                });
            }
        }

        if !dir.is_dir() {
            return Err(CoreError::AdobeFolderNotFound {
                searched: dir.to_path_buf(),
            });
        }

        let locations = Self::from_camera_raw_dir(dir);
        if locations
            .preset_folders()
            .iter()
            .any(|folder| folder.is_dir())
        {
            Ok(locations)
        } else {
            Err(CoreError::NotACameraRawFolder {
                path: dir.to_path_buf(),
            })
        }
    }

    /// Sucht den Camera-Raw-Ordner des angemeldeten Windows-Benutzers unter
    /// `%APPDATA%\Adobe\CameraRaw`.
    ///
    /// # Fehler
    ///
    /// [`CoreError::AdobeFolderNotFound`], wenn der Ordner nicht existiert –
    /// etwa weil Camera Raw noch nie gestartet wurde – oder `%APPDATA%` nicht
    /// gesetzt bzw. leer ist.
    pub fn detect() -> Result<Self, CoreError> {
        Self::detect_in(&SearchRoots::from_environment())
    }

    /// Wie [`Self::detect`], aber mit vorgegebenen Systemordnern (für Tests).
    ///
    /// # Fehler
    ///
    /// Wie [`Self::detect`].
    pub fn detect_in(roots: &SearchRoots) -> Result<Self, CoreError> {
        let Some(camera_raw_dir) = roots.user_dir() else {
            return Err(CoreError::AdobeFolderNotFound {
                searched: PathBuf::from(r"%APPDATA%\Adobe\CameraRaw"),
            });
        };

        if camera_raw_dir.is_dir() {
            Ok(Self::from_camera_raw_dir(&camera_raw_dir))
        } else {
            Err(CoreError::AdobeFolderNotFound {
                searched: camera_raw_dir,
            })
        }
    }

    /// Alle Camera-Raw-Ordner, die die App auf diesem Rechner findet: zuerst
    /// den Benutzerordner, dann die Ordner neben Lightroom-Classic-Katalogen.
    ///
    /// Nicht dabei ist `%ProgramData%\Adobe\CameraRaw`: Dort liegen nur Adobes
    /// mitgelieferte Presets und Profile (siehe [`CoreError::AdobeOwnFolder`]).
    pub fn find_all() -> Vec<FoundFolder> {
        Self::find_all_in(&SearchRoots::from_environment())
    }

    /// Wie [`Self::find_all`], aber mit vorgegebenen Systemordnern (für Tests).
    pub fn find_all_in(roots: &SearchRoots) -> Vec<FoundFolder> {
        let mut found = Vec::new();
        if let Some(dir) = roots.user_dir().filter(|dir| dir.is_dir()) {
            found.push(FoundFolder {
                kind: FolderKind::User,
                camera_raw_dir: dir,
            });
        }

        let prefs = roots
            .lightroom_preferences_file()
            .and_then(|file| std::fs::read(file).ok())
            .map(|bytes| lightroom::parse_preferences(&String::from_utf8_lossy(&bytes)))
            .unwrap_or_default();
        for catalog in prefs.catalogs {
            let Some(dir) = catalog_presets_dir(&catalog) else {
                continue;
            };
            if found
                .iter()
                .any(|known| same_path(&known.camera_raw_dir, &dir))
            {
                continue;
            }
            found.push(FoundFolder {
                kind: FolderKind::LightroomCatalog {
                    catalog,
                    in_use: prefs.presets_with_catalog,
                },
                camera_raw_dir: dir,
            });
        }
        found
    }

    /// Alle Preset-Ordner dieser Installation.
    pub fn preset_folders(&self) -> [&Path; 3] {
        [
            &self.settings,
            &self.imported_settings,
            &self.camera_profiles,
        ]
    }

    /// Alle Ordner, in denen die App ändern darf: die Preset-Ordner und die
    /// Ordner der deaktivierten Presets.
    pub fn managed_folders(&self) -> [&Path; 5] {
        [
            &self.settings,
            &self.imported_settings,
            &self.camera_profiles,
            &self.inactive_settings,
            &self.inactive_imported_settings,
        ]
    }

    /// Der Ordner, in dem `path` liegt, und sein Gegenstück: für ein aktives
    /// Preset der Ordner für deaktivierte, für ein deaktiviertes der
    /// Camera-Raw-Ordner. `None` für Pfade außerhalb.
    ///
    /// Gibt `(von, nach, ist_aktiv)` zurück.
    pub fn activation_pair(&self, path: &Path) -> Option<(&Path, &Path, bool)> {
        let pairs = [
            (&self.settings, &self.inactive_settings, true),
            (
                &self.imported_settings,
                &self.inactive_imported_settings,
                true,
            ),
            (&self.inactive_settings, &self.settings, false),
            (
                &self.inactive_imported_settings,
                &self.imported_settings,
                false,
            ),
        ];
        pairs.into_iter().find_map(|(from, to, active)| {
            is_inside(path, from).then_some((from.as_path(), to.as_path(), active))
        })
    }

    /// `true`, wenn `path` **innerhalb** eines der Preset-Ordner liegt – der
    /// Ordner selbst zählt nicht dazu.
    ///
    /// Schutz vor Datenverlust: verändernde Aktionen prüfen damit, dass sie
    /// nur Dateien und Unterordner in den Adobe-Ordnern anfassen, nie einen
    /// der Ordner selbst.
    ///
    /// Der Vergleich folgt den Windows-Regeln: Groß- und Kleinschreibung zählt
    /// nicht, und die Schreibweise `\\?\C:\…` (wie sie
    /// `std::fs::canonicalize` liefert) gilt als gleich mit `C:\…`. Pfade mit
    /// `..` werden immer abgelehnt.
    pub fn contains(&self, path: &Path) -> bool {
        let Some(candidate) = comparable_components(path) else {
            return false;
        };
        self.managed_folders().iter().any(|folder| {
            comparable_components(folder).is_some_and(|folder| {
                candidate.len() > folder.len() && candidate.starts_with(&folder)
            })
        })
    }

    /// Wie [`Self::contains`], aber als Fehler zum Weiterreichen.
    ///
    /// # Fehler
    ///
    /// [`CoreError::OutsideAdobeFolder`], wenn `path` nicht innerhalb der
    /// Preset-Ordner liegt.
    pub fn ensure_contains(&self, path: &Path) -> Result<(), CoreError> {
        if self.contains(path) {
            Ok(())
        } else {
            Err(CoreError::OutsideAdobeFolder {
                path: path.to_path_buf(),
            })
        }
    }

    /// Der Camera-Raw-Ordner selbst, in dem `Settings` usw. liegen.
    pub fn camera_raw_dir(&self) -> &Path {
        self.settings.parent().unwrap_or(&self.settings)
    }

    /// `path` relativ zum Camera-Raw-Ordner, z. B. `Settings\Paket\Warm.xmp` –
    /// so wird die Datei im Sicherungsordner abgelegt.
    ///
    /// # Fehler
    ///
    /// [`CoreError::OutsideAdobeFolder`], wenn `path` nicht darunter liegt.
    pub fn relative_path<'a>(&self, path: &'a Path) -> Result<&'a Path, CoreError> {
        path.strip_prefix(self.camera_raw_dir())
            .map_err(|_| CoreError::OutsideAdobeFolder {
                path: path.to_path_buf(),
            })
    }
}

/// Der Preset-Ordner neben einem Katalog, falls es ihn gibt.
///
/// Lightroom benennt ihn in der Sprache der Oberfläche („Lightroom Settings“,
/// „Lightroom-Einstellungen“ …). Erkannt wird er deshalb am Inhalt: ein
/// Unterordner des Katalogordners mit `Settings` und `Develop Presets`.
fn catalog_presets_dir(catalog: &Path) -> Option<PathBuf> {
    let catalog_dir = catalog.parent()?;
    std::fs::read_dir(catalog_dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|dir| dir.join("Settings").is_dir() && dir.join("Develop Presets").is_dir())
        .min()
}

/// `true`, wenn `path` **innerhalb** von `base` liegt (nicht `base` selbst).
fn is_inside(path: &Path, base: &Path) -> bool {
    match (comparable_components(path), comparable_components(base)) {
        (Some(path), Some(base)) => path.len() > base.len() && path.starts_with(&base),
        _ => false,
    }
}

/// `true`, wenn `path` gleich `base` ist oder darin liegt (Windows-Regeln wie
/// bei [`same_path`]).
fn is_at_or_below(path: &Path, base: &Path) -> bool {
    match (comparable_components(path), comparable_components(base)) {
        (Some(path), Some(base)) => path.starts_with(&base),
        _ => false,
    }
}

/// `true`, wenn zwei Pfade nach Windows-Regeln dieselbe Datei bezeichnen:
/// Groß- und Kleinschreibung zählt nicht, `\\?\C:\…` ist gleich `C:\…`.
pub fn same_path(a: &Path, b: &Path) -> bool {
    match (comparable_components(a), comparable_components(b)) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

/// Zerlegt einen Pfad in Bestandteile, die sich nach Windows-Regeln
/// vergleichen lassen. Gibt `None` zurück, wenn der Pfad `..` enthält.
fn comparable_components(path: &Path) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => parts.push(comparable_prefix(prefix.kind())),
            Component::RootDir => parts.push(String::from(r"\")),
            Component::CurDir => {}
            Component::ParentDir => return None,
            Component::Normal(part) => parts.push(part.to_string_lossy().to_lowercase()),
        }
    }
    Some(parts)
}

/// Bringt `C:` und `\\?\C:` (bzw. die UNC-Varianten) auf dieselbe Form.
fn comparable_prefix(prefix: Prefix) -> String {
    match prefix {
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
            format!("{}:", char::from(letter).to_ascii_lowercase())
        }
        Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => format!(
            r"\\{}\{}",
            server.to_string_lossy().to_lowercase(),
            share.to_string_lossy().to_lowercase()
        ),
        Prefix::Verbatim(part) | Prefix::DeviceNS(part) => part.to_string_lossy().to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leitet_unterordner_ab() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));

        assert_eq!(locations.settings, Path::new("X:/CameraRaw/Settings"));
        assert_eq!(
            locations.imported_settings,
            Path::new("X:/CameraRaw/ImportedSettings")
        );
        assert_eq!(
            locations.camera_profiles,
            Path::new("X:/CameraRaw/CameraProfiles")
        );
    }

    #[test]
    fn erkennt_pfade_innerhalb_und_ausserhalb() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));

        assert!(locations.contains(Path::new("X:/CameraRaw/Settings/Paket/a.xmp")));
        assert!(locations.contains(Path::new("X:/CameraRaw/Settings/Paket")));
        assert!(!locations.contains(Path::new("X:/CameraRaw/Logs/a.xmp")));
        assert!(!locations.contains(Path::new("X:/Dokumente/a.xmp")));
        assert!(!locations.contains(Path::new("X:/CameraRaw/SettingsAlt/a.xmp")));
    }

    #[test]
    fn deaktivierte_liegen_im_eigenen_ordner() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));
        let active = Path::new("X:/CameraRaw/Settings/Paket/a.xmp");
        let inactive = Path::new("X:/CameraRaw/Preset Manager (inactive)/Settings/Paket/a.xmp");

        assert!(locations.contains(inactive));
        assert_eq!(
            locations.activation_pair(active),
            Some((
                locations.settings.as_path(),
                locations.inactive_settings.as_path(),
                true
            ))
        );
        assert_eq!(
            locations.activation_pair(inactive),
            Some((
                locations.inactive_settings.as_path(),
                locations.settings.as_path(),
                false
            ))
        );
        assert_eq!(
            locations.activation_pair(Path::new("X:/Anderswo/a.xmp")),
            None
        );
        assert!(!locations.contains(Path::new("X:/CameraRaw/Preset Manager (inactive)/Settings")));
    }

    #[test]
    fn preset_ordner_selbst_gehoeren_nicht_dazu() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));

        assert!(!locations.contains(Path::new("X:/CameraRaw/Settings")));
        assert!(!locations.contains(Path::new("X:/CameraRaw/Settings/")));
        assert!(!locations.contains(Path::new("X:/CameraRaw/ImportedSettings")));
        assert!(!locations.contains(Path::new("X:/CameraRaw")));
    }

    #[test]
    fn lehnt_pfade_mit_zurueckspruengen_ab() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new("X:/CameraRaw"));

        assert!(!locations.contains(Path::new("X:/CameraRaw/Settings/../../Windows/a.xmp")));
    }

    #[cfg(windows)]
    #[test]
    fn vergleicht_nach_windows_regeln() {
        let locations = AdobeLocations::from_camera_raw_dir(Path::new(r"C:\Users\A\CameraRaw"));

        assert!(locations.contains(Path::new(r"c:\users\a\cameraraw\settings\a.xmp")));
        assert!(locations.contains(Path::new(r"\\?\C:\Users\A\CameraRaw\Settings\a.xmp")));
    }

    #[test]
    fn gewaehlter_ordner_wird_verwendet() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Settings")).unwrap();

        let locations = AdobeLocations::resolve(Some(dir.path())).unwrap();

        assert_eq!(locations.settings, dir.path().join("Settings"));
    }

    #[test]
    fn fehlender_gewaehlter_ordner_ist_ein_fehler() {
        let result = AdobeLocations::resolve(Some(Path::new("X:/gibt/es/nicht")));

        assert!(matches!(result, Err(CoreError::AdobeFolderNotFound { .. })));
    }

    /// Ein nachgebauter Rechner: Benutzerordner, Adobes eigener Ordner und
    /// zwei Kataloge, davon einer mit Presets daneben.
    fn fake_computer(root: &Path, presets_with_catalog: bool) -> SearchRoots {
        let app_data = root.join("Roaming");
        let program_data = root.join("ProgramData");
        std::fs::create_dir_all(app_data.join(r"Adobe\CameraRaw\Settings")).unwrap();
        std::fs::create_dir_all(program_data.join(r"Adobe\CameraRaw\Settings\Adobe")).unwrap();

        let with_presets = root.join(r"Kataloge\Reisen");
        std::fs::create_dir_all(with_presets.join(r"Lightroom-Einstellungen\Settings")).unwrap();
        std::fs::create_dir_all(with_presets.join(r"Lightroom-Einstellungen\Develop Presets"))
            .unwrap();
        // Ein Ordner mit `Settings`, aber ohne `Develop Presets` zählt nicht.
        std::fs::create_dir_all(with_presets.join(r"Exporte\Settings")).unwrap();
        let without_presets = root.join(r"Kataloge\Leer");
        std::fs::create_dir_all(&without_presets).unwrap();

        let escape = |path: &Path| path.display().to_string().replace('\\', r"\\");
        let prefs_dir = app_data.join(r"Adobe\Lightroom\Preferences");
        std::fs::create_dir_all(&prefs_dir).unwrap();
        let prefs = format!(
            "prefs = {{\n\tAgTemplateBrowser_storePresetsWithCatalog = {presets_with_catalog},\n\
             \tlibraryToLoad20 = \"{}\",\n\trecentLibraries20 = \"{}\",\n}}\n",
            escape(&with_presets.join("Reisen.lrcat")),
            escape(&without_presets.join("Leer.lrcat")),
        );
        std::fs::write(
            prefs_dir.join("Lightroom Classic CC 7 Preferences.agprefs"),
            prefs,
        )
        .unwrap();

        SearchRoots {
            app_data: Some(app_data),
            program_data: Some(program_data),
        }
    }

    #[test]
    fn findet_benutzerordner_und_katalogordner() {
        let dir = tempfile::tempdir().unwrap();
        let roots = fake_computer(dir.path(), true);

        let found = AdobeLocations::find_all_in(&roots);

        assert_eq!(
            found,
            vec![
                FoundFolder {
                    kind: FolderKind::User,
                    camera_raw_dir: dir.path().join(r"Roaming\Adobe\CameraRaw"),
                },
                FoundFolder {
                    kind: FolderKind::LightroomCatalog {
                        catalog: dir.path().join(r"Kataloge\Reisen\Reisen.lrcat"),
                        in_use: true,
                    },
                    camera_raw_dir: dir.path().join(r"Kataloge\Reisen\Lightroom-Einstellungen"),
                },
            ]
        );
    }

    #[test]
    fn katalogordner_wird_auch_ausgeschaltet_gefunden() {
        let dir = tempfile::tempdir().unwrap();
        let roots = fake_computer(dir.path(), false);

        let found = AdobeLocations::find_all_in(&roots);

        assert!(matches!(
            found.last().map(|folder| &folder.kind),
            Some(FolderKind::LightroomCatalog { in_use: false, .. })
        ));
    }

    #[test]
    fn standard_bleibt_der_benutzerordner() {
        let dir = tempfile::tempdir().unwrap();
        let roots = fake_computer(dir.path(), true);

        let locations = AdobeLocations::resolve_in(None, &roots).unwrap();

        assert_eq!(
            locations.camera_raw_dir(),
            dir.path().join(r"Roaming\Adobe\CameraRaw")
        );
    }

    #[test]
    fn ohne_lightroom_nur_der_benutzerordner() {
        let dir = tempfile::tempdir().unwrap();
        let app_data = dir.path().join("Roaming");
        std::fs::create_dir_all(app_data.join(r"Adobe\CameraRaw")).unwrap();
        let roots = SearchRoots {
            app_data: Some(app_data),
            program_data: None,
        };

        let found = AdobeLocations::find_all_in(&roots);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, FolderKind::User);
        assert!(AdobeLocations::find_all_in(&SearchRoots::default()).is_empty());
    }

    #[test]
    fn adobes_eigener_ordner_ist_tabu() {
        let dir = tempfile::tempdir().unwrap();
        let roots = fake_computer(dir.path(), false);
        let own = dir.path().join(r"ProgramData\Adobe\CameraRaw");

        for chosen in [
            own.clone(),
            own.join("Settings"),
            own.join(r"Settings\Adobe"),
        ] {
            let result = AdobeLocations::resolve_in(Some(&chosen), &roots);
            assert!(
                matches!(result, Err(CoreError::AdobeOwnFolder { .. })),
                "{chosen:?}: {result:?}"
            );
        }
        assert!(AdobeLocations::find_all_in(&roots)
            .iter()
            .all(|folder| !folder
                .camera_raw_dir
                .starts_with(dir.path().join("ProgramData"))));
    }

    #[test]
    fn ordner_ohne_preset_unterordner_ist_kein_camera_raw_ordner() {
        let dir = tempfile::tempdir().unwrap();

        let result = AdobeLocations::resolve(Some(dir.path()));

        assert!(matches!(result, Err(CoreError::NotACameraRawFolder { .. })));
    }
}
