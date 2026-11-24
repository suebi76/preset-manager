# 03 – Architektur

## Grundidee

Die App besteht aus zwei Teilen, die als getrennte Crates in einem
Cargo-Workspace liegen:

```
┌──────────────────────────────┐
│ preset_manager  (Programm)   │  Fenster, Listen, Dialoge, Drag & Drop
│ kennt: preset_core           │
└──────────────┬───────────────┘
               │ ruft auf
┌──────────────▼───────────────┐
│ preset_core  (Bibliothek)    │  Ordner finden, XMP lesen/ändern, ZIP,
│ kennt: nichts von der GUI    │  kopieren, löschen, umbenennen, sortieren
└──────────────────────────────┘
```

Warum diese Trennung: Die gesamte Logik lässt sich ohne Fenster testen, und wer
den Code liest, findet Fachlogik und Oberfläche nie vermischt. Die Oberfläche
könnte später ausgetauscht werden, ohne die Logik anzufassen.

## Ordnerstruktur

Jedes Modul beginnt mit einer Beschreibung, was es enthält. Ein Ordner mit
`mod.rs` nennt dort seine Teile in einer Tabelle. Keine Datei hat mehr als
rund 500 Zeilen Programmcode (N-09).

```
preset-manager/
├── Cargo.toml                 Workspace, gemeinsame Abhängigkeiten und Lints
├── README.md, README.de.md
├── CHANGELOG.md               Text der Releases
├── LICENSE                    MIT
├── PRIVACY.md                 Datenschutzerklärung (auch für den Microsoft Store)
├── .cargo/audit.toml          begründete Ausnahmen für `cargo audit`
├── .github/workflows/
│   ├── ci.yml                 fmt, clippy, Tests; `cargo audit`
│   └── release.yml            Tag `v*` → Installationsprogramm und Update-Pakete
├── Planung/                   diese Dokumente
├── docs/                      Projektseite (GitHub Pages) und Bildschirmfotos
├── packaging/msix/            Manifest, Symbole und Store-Eintrag (Microsoft Store)
├── scripts/                   Demo-Bibliothek, New-Release.ps1, New-StorePackage.ps1,
│                              Vorschaubild
├── dotnet-tools.json          Packwerkzeug vpk (gleiche Version wie velopack)
└── crates/
    ├── preset_core/           die Logik, ohne Oberfläche
    │   ├── src/
    │   │   ├── lib.rs         Einstieg, Modulübersicht
    │   │   ├── error.rs       alle Fehlerarten der Bibliothek
    │   │   ├── report.rs      Ergebnisbericht einer Aktion
    │   │   ├── adobe/         Wo liegen die Adobe-Ordner? Läuft Adobe gerade?
    │   │   ├── preset/        Was ist ein Preset? XMP lesen und ändern,
    │   │   │                  Einstellungen deuten, Farbtabellen
    │   │   ├── import/        Quelle durchsuchen → Plan → ausführen
    │   │   ├── library/       Installierte Presets einlesen und verwalten
    │   │   ├── safe_fs/       Sicher schreiben und lesen, Sicherungen,
    │   │   │                  Papierkorb, Protokoll der Änderungen
    │   │   ├── history.rs     Rückgängig und Wiederholen
    │   │   ├── export.rs      als ZIP speichern
    │   │   └── preview/       Vorher/Nachher nachrechnen
    │   └── tests/             Integrationstests mit Testdaten
    └── preset_manager/        das Fenster und das Konsolenwerkzeug
        ├── build.rs           bettet Symbol und Versionsangaben in die .exe ein
        ├── assets/            App-Symbol
        └── src/
            ├── main.rs        Programmstart
            ├── app/           Zustand der Anwendung, Ereignisse, Abfragen
            ├── actions/       was der Nutzer auslösen kann, je Thema eine Datei
            ├── worker/        Hintergrundarbeit und Geschichte (Rückgängig)
            ├── settings.rs    Einstellungen und leere Gruppen
            ├── logging.rs     Logdatei
            ├── updates/       neue Versionen: Selbst-Update (Velopack, Merkmal
            │                  `self-update`) oder über den Microsoft Store
            ├── texts.rs       Oberflächentexte und Fehlerbeschreibungen
            ├── ui/            je Bereich eine Datei oder ein Ordner
            └── bin/preset_cli Konsolenwerkzeug für Tests und Skripte
```

## Module von `preset_core`

| Modul | Aufgabe | Wichtigste Elemente |
|---|---|---|
| `adobe::locations` | Adobe-Ordner ermitteln, alle Fundorte (auch neben Lightroom-Katalogen) | `AdobeLocations::detect()`, `AdobeLocations::find_all()` |
| `adobe::lightroom` | Lightroom-Einstellungen lesen (Kataloge, „Presets mit Katalog“) | `parse_preferences()` |
| `adobe::running` | Prüfen, ob Photoshop/Lightroom läuft | `running_adobe_apps()` |
| `preset::model` | Datentypen | `Preset`, `PresetOrigin`, `Section` |
| `preset::xmp` | Kennfelder aus XMP lesen | `read_metadata()` |
| `preset::xmp_edit` | Name, Gruppe, Reihenfolge, UUID gezielt ändern | `rewrite_metadata()`, `replace_uuid()` |
| `preset::settings` | alle Einstellungen eines Presets lesen | `read_settings()`, `PresetSettings` |
| `preset::summary` | Kurzfassung für Suche und Hinweise: Wirkung, benötigtes Profil | `PresetSummary`, `ProfileRef` |
| `preset::adjustments` | Einstellungen deuten: Regler, Kurven, Farbe, Aussagen | `SLIDERS`, `Fact` |
| `preset::color_table` | Farbtabellen der Profile dekodieren und anwenden | `ColorTable` |
| `import::source` | Ordner oder ZIP einheitlich durchsuchen | `scan()` |
| `import::archive` | ZIP sicher entpacken (Grenzen, keine Pfade nach außen) | `ArchiveLimits` |
| `import::plan` | Plan erstellen und ausführen | `ImportPlan`, `ConflictPolicy` |
| `library::scan` | installierte (auch deaktivierte) Presets einlesen | `Library::load()` |
| `library::profiles` | bekannte Profile (Nutzer und Adobe), „Profil fehlt“ | `ProfileIndex`, `profile_status()` |
| `library::ops` | löschen, umbenennen, verschieben, sortieren, aktivieren | `delete()`, `rename_preset()`, `move_to_group()`, `set_active()` |
| `history` | Rückgängig und Wiederholen | `History`, `revert()` |
| `export` | als ZIP speichern | `export_zip()` |
| `preview` | Vorher/Nachher (Annäherung) | `develop()`, `missing()` |
| `safe_fs` | atomar schreiben, mit Grenze lesen, sichern, Papierkorb, Protokoll | `write_atomic()`, `read_preset_file()`, `BackupSession`, `Journal` |
| `report` | Was hat geklappt, was nicht | `OperationReport`, `ItemOutcome` |
| `error` | Fehlerarten | `CoreError` |

### Prinzip „erst planen, dann ausführen“

Jede verändernde Aktion läuft in zwei Schritten:

1. **Planen** – liest nur. Ergebnis ist ein Datenobjekt (z. B. `ImportPlan`), das
   genau beschreibt, welche Datei wohin kopiert wird und wo es Konflikte gibt.
2. **Ausführen** – nimmt den Plan und arbeitet ihn ab. Ergebnis ist ein
   `OperationReport`.

Dadurch kann die Oberfläche vor jeder Aktion eine ehrliche Vorschau zeigen, und
Tests können Pläne prüfen, ohne eine einzige Datei zu verändern.

### XMP ändern, ohne etwas kaputt zu machen

Ein Preset enthält hunderte Einstellungen, die die App nicht kennt und nicht
verstehen muss. Deshalb wird die Datei **nicht** in eine Struktur eingelesen und
neu geschrieben. Stattdessen wird sie als Ereignisstrom durchgereicht
(`quick-xml`: Reader → Writer), und nur der Text innerhalb von `crs:Name`,
`crs:Group` bzw. `crs:SortName` wird ersetzt. Alles andere bleibt, wie es war.

## Oberfläche (`preset_manager`)

```
┌───────────────────────────────────────────────────────────────┐
│ [Importieren…]  [Löschen] [Umbenennen] [Verschieben]  Suche:▢ │
├──────────────────────┬────────────────────────────────────────┤
│ Gruppen              │ Presets in „Sommerlicht Presets“        │
│  ▸ Sommerlicht (11)  │  ☐ Sommerlicht - Bondi                  │
│  ▸ Reisetagebuch (18)│  ☑ Sommerlicht - Brighton               │
│  ▸ Filmklassiker (42)│  ☑ Sommerlicht - Byron                  │
│                      │  …                                      │
├──────────────────────┴────────────────────────────────────────┤
│ ⚠ Photoshop läuft – Änderungen werden nach Neustart sichtbar  │
│ Ordner oder ZIP hierher ziehen                                │
└───────────────────────────────────────────────────────────────┘
```

Der Entwurf oben ist der ursprüngliche Plan; wie die Oberfläche heute
aussieht, zeigen die Bildschirmfotos im README und `07-Oberflaeche.md`.

| Datei | Inhalt |
|---|---|
| `ui/topbar.rs` | Kopfleiste: Suche, Rückgängig/Wiederholen, Importieren, Einstellungen |
| `ui/groups.rs` | linke Spalte: Gruppen, aktiv und deaktiviert |
| `ui/list/` | Mitte: Einträge, Zeilen, Kontextmenü, Ziehen und Ablegen |
| `ui/details/` | rechte Spalte: Wirkung bzw. Verwalten für Preset, Auswahl, Gruppe |
| `ui/preset_view/` | „Was macht dieses Preset?“: Satz, Regler, Kurve, Farbmischer, Color Grading |
| `ui/before_after.rs` | Vorher/Nachher-Vorschau |
| `ui/import_dialog.rs` | Importvorschau mit Konfliktauswahl |
| `ui/confirm_dialog.rs` | Bestätigung vor Löschen und Zusammenführen |
| `ui/settings_dialog/` | Einstellungen, je Abschnitt eine Datei |
| `ui/overlays.rs` | Einblendungen, Hinweisleisten, Fehlerseite |
| `ui/theme.rs`, `ui/widgets.rs` | Farben, Schrift, wiederverwendete Bausteine |

Dateioperationen laufen in einem Hintergrund-Thread (`worker/`). Oberfläche
und Worker tauschen Nachrichten über Kanäle (`std::sync::mpsc`) aus:
`Command` hin, `Event` (Fortschritt, Ergebnis) zurück.

## Bibliotheken

| Zweck | Crate | Begründung |
|---|---|---|
| Oberfläche | `eframe` / `egui` | reines Rust, eine `.exe` ohne Laufzeitumgebung, Datei-Drop eingebaut |
| Auswahldialog | `rfd` | nativer Windows-Dialog |
| ZIP | `zip` | Standard-Crate |
| XML | `quick-xml` | Ereignisstrom, erhält unbekannte Inhalte |
| Fehlerarten | `thiserror` | lesbare Fehlerdefinitionen |
| Papierkorb | `trash` | Windows-Papierkorb |
| Temporäre Ordner | `tempfile` | räumt selbst auf |
| Ordner durchlaufen | `walkdir` | |
| UUID | `uuid` | neue Kennung bei „beide behalten“ |
| Protokoll | `tracing`, `tracing-appender`, `tracing-subscriber` | Logdatei für Fehlersuche |
| Einstellungen | – (eigenes Format `schlüssel=wert`) | zwei Werte rechtfertigen keine drei Abhängigkeiten; die Datei bleibt von Hand lesbar |
| Programmsymbol | `winresource` (nur beim Bauen) | Symbol und Versionsangaben in der `.exe` |
| Installation, Updates | `velopack` + Packwerkzeug `vpk` | wie CloudDrive-Sync: Installationsprogramm ohne Adminrechte, Updates aus GitHub-Releases, Delta-Pakete |
| Symbole in der Oberfläche | `egui-phosphor` | einheitliche Symbolschrift |
| Laufende Prozesse | `sysinfo` | Hinweis F-41 |

Abhängigkeiten werden erst in dem Meilenstein eingetragen, der sie braucht. Das
Grundgerüst enthält nur `thiserror`.

## Entscheidungen

| Nr. | Entscheidung | Alternative | Grund |
|---|---|---|---|
| E-1 | Oberfläche mit `egui` | Tauri (HTML/JS), Slint, iced | Alles in einer Sprache; kein zweiter Technologiestapel, den man verstehen muss |
| E-2 | Workspace mit zwei Crates | ein Crate | Logik ohne GUI testbar, klare Grenze |
| E-3 | Ein Zielordner `CameraRaw\Settings` | Kopie je Anwendung | So arbeitet Adobe selbst (siehe 02) |
| E-4 | Nicht in `ImportedSettings` schreiben, nur lesen und verwalten | dorthin importieren | Der Ordner gehört Adobes Importdialog; eigene Importe bleiben unterscheidbar |
| E-5 | XMP als Strom ändern | komplett parsen und neu schreiben | Unbekannte Felder bleiben garantiert erhalten |
| E-6 | Bezeichner englisch, Kommentare und Oberfläche deutsch | alles englisch | Englische Bezeichner sind Rust-Konvention; deutsche Kommentare für die Nachvollziehbarkeit. Vor der Veröffentlichung umstellbar |
| E-7 | Synchroner Code mit einem Worker-Thread | `async`/`tokio` | Für lokale Dateioperationen reicht das und ist leichter zu lesen |

## Code-Regeln

- `cargo fmt` und `cargo clippy -- -D warnings` müssen sauber sein.
- `unsafe` ist per Lint verboten; `unwrap`/`expect`/`panic` sind außerhalb von Tests per Lint verboten.
- Jedes öffentliche Element hat einen Doc-Kommentar, der sagt, **wozu** es da ist.
- Funktionen bleiben kurz genug, um auf einen Bildschirm zu passen; eine Datei hat ein Thema
  und höchstens rund 500 Zeilen Programmcode (N-09).
- Fremde Dateien sind nicht vertrauenswürdig: gelesen wird mit Grenze
  (`safe_fs::read_preset_file`), geschrieben nur in den Adobe-Ordnern (N-08).
- Pfade werden nie per String zusammengesetzt, immer über `Path::join`.

## Hinweis zur Entwicklung in OneDrive

Der Projektordner liegt in OneDrive. Der Build-Ordner `target/` wird mehrere
Gigabyte groß und ändert sich ständig – er sollte nicht synchronisiert werden.
Empfehlung: Umgebungsvariable `CARGO_TARGET_DIR` auf einen Ordner außerhalb von
OneDrive setzen (z. B. `C:\Dev\cargo-target`).
