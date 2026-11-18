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

```
preset-manager/
├── Cargo.toml                 Workspace, gemeinsame Abhängigkeiten und Lints
├── README.md
├── rustfmt.toml
├── .gitignore
├── LICENSE                    MIT
├── Start.bat                  baut und startet das Fenster
├── .github/workflows/
│   ├── ci.yml                 fmt + clippy + test auf Windows
│   └── release.yml            Tag `v*` → .exe als GitHub-Release
├── Planung/                   diese Dokumente
├── docs/                      Bildschirmfoto für das README
├── scripts/                   Demo-Bibliothek zum Ausprobieren
└── crates/
    ├── preset_core/
    │   ├── src/
    │   │   ├── lib.rs         Einstieg, Modulübersicht
    │   │   ├── error.rs       alle Fehlerarten der Bibliothek
    │   │   ├── report.rs      Ergebnisbericht einer Aktion
    │   │   ├── adobe/         Wo liegen die Adobe-Ordner? Läuft Adobe gerade?
    │   │   ├── preset/        Was ist ein Preset? XMP lesen und ändern
    │   │   ├── import/        Quelle durchsuchen → Plan → ausführen
    │   │   ├── library/       Installierte Presets einlesen und verwalten
    │   │   └── safe_fs/       Sicheres Schreiben, Papierkorb, Sicherungen
    │   └── tests/             Integrationstests mit Testdaten
    └── preset_manager/
        ├── build.rs           bettet Symbol und Versionsangaben in die .exe ein
        ├── assets/            App-Symbol (icon.ico/.png, make_icon.py)
        └── src/
            ├── main.rs        Programmstart
            ├── app.rs         Zustand der Anwendung, Ordnerwahl
            ├── actions.rs     was der Nutzer auslösen kann
            ├── worker.rs      Hintergrundarbeit
            ├── settings.rs    Einstellungen und leere Gruppen
            ├── logging.rs     Logdatei
            ├── texts.rs       alle Oberflächentexte
            ├── ui/            je Ansicht eine Datei (theme, topbar, groups,
            │                  list, details, import_dialog, confirm_dialog,
            │                  settings_dialog, overlays, widgets)
            └── bin/preset_cli Konsolenwerkzeug für Tests und Skripte
```

## Module von `preset_core`

| Modul | Aufgabe | Wichtigste Elemente |
|---|---|---|
| `adobe::locations` | Adobe-Ordner ermitteln | `AdobeLocations::detect()` |
| `adobe::running` | Prüfen, ob Photoshop/Lightroom läuft | `running_adobe_apps()` |
| `preset::model` | Datentypen | `Preset`, `PresetKind`, `PresetGroup` |
| `preset::xmp` | Felder aus XMP lesen und gezielt ändern | `read_metadata()`, `rewrite_metadata()` |
| `import::source` | Ordner oder ZIP einheitlich durchsuchen | `ImportSource`, `scan()` |
| `import::archive` | ZIP sicher entpacken | `extract_to_temp()` |
| `import::plan` | Plan erstellen und ausführen | `ImportPlan`, `ConflictPolicy`, `execute()` |
| `library::scan` | Installierte Presets einlesen | `Library::load()` |
| `library::ops` | Löschen, umbenennen, verschieben, sortieren | `delete()`, `rename_preset()`, `rename_group()`, `move_to_group()`, `reorder()` |
| `safe_fs` | Atomar schreiben, Papierkorb, Sicherung | `write_atomic()`, `move_to_trash()`, `backup()` |
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

| Datei | Inhalt |
|---|---|
| `ui/toolbar.rs` | Werkzeugleiste |
| `ui/group_list.rs` | linke Spalte |
| `ui/preset_list.rs` | rechte Spalte mit Mehrfachauswahl und Sortierung |
| `ui/import_dialog.rs` | Importvorschau mit Konfliktauswahl |
| `ui/confirm_dialog.rs` | Bestätigung vor Löschen |
| `ui/report_dialog.rs` | Ergebnisbericht |
| `ui/drop_zone.rs` | Drag & Drop |

Dateioperationen laufen in einem Hintergrund-Thread (`worker.rs`). Oberfläche
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
- Funktionen bleiben kurz genug, um auf einen Bildschirm zu passen; eine Datei hat ein Thema.
- Pfade werden nie per String zusammengesetzt, immer über `Path::join`.

## Hinweis zur Entwicklung in OneDrive

Der Projektordner liegt in OneDrive. Der Build-Ordner `target/` wird mehrere
Gigabyte groß und ändert sich ständig – er sollte nicht synchronisiert werden.
Empfehlung: Umgebungsvariable `CARGO_TARGET_DIR` auf einen Ordner außerhalb von
OneDrive setzen (z. B. `C:\Dev\cargo-target`).
