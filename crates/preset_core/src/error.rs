//! Alle Fehlerarten der Bibliothek.
//!
//! Jede Funktion, die scheitern kann, gibt `Result<_, CoreError>` zurück. Die
//! Texte hier sind für die Logdatei gedacht; die Oberfläche zeigt eigene,
//! ausführlichere Meldungen (siehe `Planung/04-Fehlerbehandlung.md`).

use std::path::PathBuf;

use thiserror::Error;

/// Ein Fehler aus `preset_core`.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Der Camera-Raw-Ordner existiert nicht.
    #[error("Camera-Raw-Ordner nicht gefunden: {searched}")]
    AdobeFolderNotFound {
        /// Der Ordner, der erwartet wurde.
        searched: PathBuf,
    },

    /// Der vom Nutzer gewählte Ordner existiert, ist aber kein
    /// Camera-Raw-Ordner (er enthält weder `Settings` noch `ImportedSettings`
    /// noch `CameraProfiles`). Häufige Ursache: eine Ebene zu tief gewählt.
    #[error("Kein Camera-Raw-Ordner: {path}")]
    NotACameraRawFolder {
        /// Der gewählte Ordner.
        path: PathBuf,
    },

    /// Der gewählte Ordner enthält Adobes mitgelieferte Presets und Profile
    /// (`%ProgramData%\Adobe\CameraRaw`). Die App verwaltet ihn nicht:
    /// Ändern würde Photoshop und Lightroom beschädigen.
    #[error("Ordner mit Adobes mitgelieferten Presets: {path}")]
    AdobeOwnFolder {
        /// Der gewählte Ordner.
        path: PathBuf,
    },

    /// Lesen oder Schreiben einer Datei ist gescheitert.
    #[error("Dateizugriff gescheitert: {path}")]
    Io {
        /// Die betroffene Datei oder der betroffene Ordner.
        path: PathBuf,
        /// Die Ursache, wie das Betriebssystem sie meldet.
        #[source]
        source: std::io::Error,
    },

    /// Das Archiv ist defekt oder passwortgeschützt.
    #[error("Archiv nicht lesbar: {path} ({reason})")]
    InvalidArchive {
        /// Das Archiv.
        path: PathBuf,
        /// Was daran nicht stimmt.
        reason: String,
    },

    /// Ein Eintrag im Archiv würde außerhalb des Zielordners landen.
    #[error("Archiv enthält einen unzulässigen Pfad: {entry}")]
    UnsafeArchiveEntry {
        /// Der Pfad des Eintrags, wie er im Archiv steht.
        entry: String,
    },

    /// Das Archiv überschreitet eine der Schutzgrenzen (Größe, Dateianzahl,
    /// Verschachtelung). Schützt vor „ZIP-Bomben“.
    #[error("Archiv zu groß: {path} ({limit})")]
    ArchiveTooLarge {
        /// Das Archiv.
        path: PathBuf,
        /// Welche Grenze überschritten wurde.
        limit: ArchiveLimit,
    },

    /// Die Quelle ist weder ein Ordner noch ein ZIP-Archiv.
    #[error("Weder Ordner noch ZIP-Archiv: {path}")]
    UnsupportedSource {
        /// Die abgelehnte Quelle.
        path: PathBuf,
    },

    /// Die Zieldatei existiert bereits und sollte nicht überschrieben werden.
    #[error("Datei existiert bereits: {path}")]
    AlreadyExists {
        /// Die vorhandene Datei.
        path: PathBuf,
    },

    /// Ein Zielpfad liegt außerhalb der Adobe-Ordner. Das darf nie passieren;
    /// die Prüfung ist eine letzte Sicherung gegen Programmierfehler.
    #[error("Ziel außerhalb der Adobe-Ordner: {path}")]
    OutsideAdobeFolder {
        /// Der abgelehnte Pfad.
        path: PathBuf,
    },

    /// Die Datei ist eine XMP-Datei, aber kein Camera-Raw-Preset.
    #[error("Kein Preset: {path}")]
    NotAPreset {
        /// Die geprüfte Datei.
        path: PathBuf,
    },

    /// Die XMP-Datei ist beschädigt.
    #[error("XMP-Datei beschädigt: {path} ({reason})")]
    MalformedXmp {
        /// Die beschädigte Datei.
        path: PathBuf,
        /// Was der XML-Leser gemeldet hat.
        reason: String,
    },

    /// Der gewünschte Name ist nicht zulässig.
    #[error("Unzulässiger Name „{name}“: {problem}")]
    InvalidName {
        /// Der abgelehnte Name.
        name: String,
        /// Warum er abgelehnt wurde.
        problem: NameProblem,
    },

    /// Eine Gruppe dieses Namens gibt es nicht.
    #[error("Gruppe nicht gefunden: {name}")]
    GroupNotFound {
        /// Der gesuchte Gruppenname.
        name: String,
    },

    /// Es gibt bereits ein Preset mit diesem Namen in der Gruppe.
    #[error("Name bereits vergeben: {name}")]
    NameConflict {
        /// Der doppelte Name.
        name: String,
    },

    /// Die Quelle enthält keine Presets.
    #[error("Keine Presets gefunden in: {source_path}")]
    NoPresetsFound {
        /// Der durchsuchte Ordner oder das durchsuchte Archiv.
        source_path: PathBuf,
    },

    /// Eine Datei wurde seit der Aktion geändert (z. B. in Camera Raw) –
    /// Rückgängig lässt sie deshalb unangetastet.
    #[error("Seit der Aktion geändert: {path}")]
    ChangedSinceAction {
        /// Die geänderte oder fehlende Datei.
        path: PathBuf,
    },

    /// Die Datei konnte nicht in den Papierkorb verschoben werden.
    #[error("Papierkorb nicht verfügbar für: {path} ({reason})")]
    Trash {
        /// Die Datei, die gelöscht werden sollte.
        path: PathBuf,
        /// Was das Betriebssystem gemeldet hat.
        reason: String,
    },
}

/// Warum ein Name abgelehnt wurde. Ein fester Wert statt eines Satzes, damit
/// die Oberfläche ihn in ihrer Sprache beschreiben kann; der deutsche Text
/// hier ist für Logdatei und Konsolenwerkzeug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NameProblem {
    /// Leer oder nur Leerzeichen.
    #[error("darf nicht leer sein")]
    Empty,
    /// Länger als erlaubt.
    #[error("ist länger als {max} Zeichen")]
    TooLong {
        /// Die erlaubte Höchstlänge in Zeichen.
        max: usize,
    },
    /// Enthält Zeilenumbrüche, Tabulatoren o. Ä.
    #[error("enthält Steuerzeichen")]
    ControlCharacters,
}

/// Welche Schutzgrenze ein Archiv überschritten hat (`import::archive`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ArchiveLimit {
    /// Zu viele Einträge insgesamt.
    #[error("mehr als {max} Einträge")]
    TooManyEntries {
        /// Die erlaubte Anzahl.
        max: usize,
    },
    /// Zu viele ZIP-Dateien ineinander.
    #[error("mehr als {max} ZIP-Ebenen ineinander")]
    TooDeep {
        /// Die erlaubte Verschachtelungstiefe.
        max: usize,
    },
    /// Entpackt zu groß.
    #[error("entpackt größer als {max_mb} MB")]
    TooLarge {
        /// Die erlaubte Größe in Megabyte.
        max_mb: u64,
    },
}
