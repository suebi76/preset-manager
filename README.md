# Preset Manager

Windows-App zum Installieren und Verwalten von Camera-Raw-Presets und
-Profilen für Photoshop, Photoshop (Beta) und Lightroom Classic.

![Preset Manager mit einer Demo-Bibliothek](docs/bildschirmfoto.png)

## Funktionen

- **Installieren:** Ordner oder ZIP-Dateien ins Fenster ziehen (oder
  „Presets importieren“). Eine Vorschau zeigt, was wohin kommt; bereits
  installierte Presets sind abgewählt, bei vorhandenen Dateien wählt man
  „überspringen“, „ersetzen (vorher sichern)“ oder „beide behalten“.
  Auf eine Gruppe links gezogen, landet alles direkt in dieser Gruppe.
- **Überblick:** Presets und Profile getrennt, wie in Camera Raw, mit
  Typ-Symbolen, Suche (Strg+F) und Filter (Strg+1/2/3).
- **Verwalten:** Umbenennen (F2), Löschen (Entf), in andere Gruppen
  verschieben, Reihenfolge ändern (Alt+↑/↓), neue Gruppen anlegen (Strg+G),
  Gruppen zusammenführen – per Ziehen & Ablegen, Rechtsklick oder Tastatur,
  mit Mehrfachauswahl (Strg/Umschalt+Klick, Strg+A).
- **Sicher:** Gelöschtes kommt in den Windows-Papierkorb, vor jedem Ersetzen
  oder Ändern wird das Original gesichert, Dateien werden nie halb
  geschrieben. ZIP-Archive werden geprüft, bevor etwas installiert wird.
- **Einstellungen** (Zahnrad oder Strg+,): anderer Camera-Raw-Ordner,
  Größe der Oberfläche, Ordner für Sicherungen und Logdateien.

Nach Änderungen Photoshop bzw. Lightroom neu starten – erst dann zeigen sie
die neuen Presets. Die App weist darauf hin, wenn eines davon läuft.

## Installieren und starten

**Fertiges Programm:** Unter *Releases* die ZIP-Datei herunterladen,
entpacken und `Preset Manager.exe` starten. Eine Installation ist nicht
nötig.

**Aus dem Quellcode:** `Start.bat` im Projektordner (braucht
[Rust](https://rustup.rs)):

- **Doppelklick:** öffnet das Fenster.
- **Ordner oder ZIP auf `Start.bat` ziehen:** öffnet das Fenster gleich mit
  der Importvorschau.

Beim ersten Start wird das Programm übersetzt, das dauert einige Minuten.

## Wo die App was ablegt

| Was | Wo |
|---|---|
| Presets und Profile | `%APPDATA%\Adobe\CameraRaw\Settings\<Paket>\` – derselbe Ordner für alle drei Adobe-Programme |
| Einstellungen | `%APPDATA%\PresetManager\` |
| Sicherungen (die letzten 20 Aktionen) | `%LOCALAPPDATA%\PresetManager\backups\` |
| Logdateien (die letzten 7 Tage) | `%LOCALAPPDATA%\PresetManager\logs\` |

## Ausprobieren ohne die eigenen Presets

`scripts/demo_bibliothek.py` legt einen Camera-Raw-Ordner mit ausgedachten
Demo-Presets an. Über die Umgebungsvariable `APM_CAMERA_RAW_DIR` arbeitet
die App dann nur dort; oben im Fenster steht „Anderer Ordner“.

In der Eingabeaufforderung (`cmd`) im Projektordner:

```bat
python scripts\demo_bibliothek.py C:\Temp\DemoCameraRaw
set APM_CAMERA_RAW_DIR=C:\Temp\DemoCameraRaw
Start.bat
```

## Aufbau

| Ordner | Inhalt |
|---|---|
| `crates/preset_core` | Bibliothek mit der gesamten Logik, ohne Oberfläche |
| `crates/preset_manager` | Das Fenster (`src/main.rs`) und ein Konsolenwerkzeug (`src/bin/preset_cli`) |
| `Planung` | Anforderungen, Architektur, Fehlerkonzept, Tests, Roadmap, Oberflächen-Entwurf |
| `scripts` | Hilfsskripte (Demo-Bibliothek) |

Einstieg in die Planung: [Planung/00-Uebersicht.md](Planung/00-Uebersicht.md)

## Entwickeln

Voraussetzung: [Rust](https://rustup.rs) (stable).

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo run -p preset_manager --bin preset_manager
```

Ein neues Release entsteht, wenn ein Tag wie `v1.0.1` gepusht wird
(`.github/workflows/release.yml`); die Version in `Cargo.toml` muss dazu
passen.

### Konsolenwerkzeug

Für Tests und Skripte gibt es `preset_cli`. Ohne Angaben listet es die
installierten Einträge auf; `importieren` zeigt eine Vorschau, kopiert wird
erst mit `--ausfuehren`:

```bash
cargo run -p preset_manager --bin preset_cli -- importieren "Downloads\Paket.zip"
```

`cargo run -p preset_manager --bin preset_cli -- --hilfe` zeigt alle Angaben.

## Lizenz und Marken

MIT-Lizenz, siehe [LICENSE](LICENSE).

Adobe, Photoshop, Lightroom und Camera Raw sind Marken der Adobe Inc. Dieses
Projekt steht in keiner Verbindung zu Adobe und wird von Adobe weder
unterstützt noch geprüft.
