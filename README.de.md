<div align="center">

<img src="crates/preset_manager/assets/icon.png" width="96" alt="Symbol von Preset Manager">

# Preset Manager

**Camera-Raw-Presets und -Profile für Photoshop und Lightroom Classic installieren und ordnen – per Ziehen & Ablegen, sicher, an einem Ort.**

[![Neueste Version](https://img.shields.io/github/v/release/suebi76/preset-manager?label=download&color=2680eb)](https://github.com/suebi76/preset-manager/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/suebi76/preset-manager/total?color=9256d9)](https://github.com/suebi76/preset-manager/releases)
[![CI](https://github.com/suebi76/preset-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/suebi76/preset-manager/actions/workflows/ci.yml)
[![Lizenz: MIT](https://img.shields.io/badge/license-MIT-2d9d78)](LICENSE)
![Windows 10/11](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-555)

[English](README.md) · **Deutsch**

![Preset Manager mit einer Demo-Bibliothek aus Presets und Profilen](docs/bildschirmfoto.png)

</div>

## Wozu?

Ein Presetpaket gekauft, entpackt – und jetzt? Adobe erwartet die
`.xmp`-Dateien in einem versteckten Ordner tief in `AppData`, Photoshop und
Lightroom Classic teilen ihn sich, Profile und Presets sehen gleich aus, und
ein falscher Klick überschreibt ein bezahltes Preset.

**Preset Manager** nimmt einem das ab:

- **Ordner oder ZIP ins Fenster ziehen** – die App findet alle Presets und
  Profile darin (auch in verschachtelten ZIPs), zeigt eine Vorschau und
  installiert sie an die richtige Stelle für **Photoshop, Photoshop (Beta)
  und Lightroom Classic** zugleich.
- **Sehen, was man hat** – Presets und Profile getrennt wie in Camera Raw,
  mit Suche und Filter.
- **Ordnung halten** – umbenennen, löschen, in Gruppen verschieben,
  sortieren, Gruppen anlegen und zusammenführen, per Ziehen & Ablegen,
  Rechtsklick oder Tastatur.
- **Nie ein Preset verlieren** – Gelöschtes kommt in den Papierkorb, vor
  jedem Ersetzen oder Ändern wird das Original gesichert, Dateien werden nie
  halb geschrieben.

## Herunterladen

1. **[PresetManager-Setup.exe](https://github.com/suebi76/preset-manager/releases/latest)**
   aus dem neuesten Release herunterladen und starten.
2. Fertig – ohne Administratorrechte. Preset Manager steht im Startmenü und
   auf dem Desktop und **aktualisiert sich selbst** (mit Hinweis oder auf
   Wunsch automatisch).

Ohne Installation: `PresetManager-Portable.zip` aus demselben Release
entpacken und `PresetManager.exe` starten.

Das Installationsprogramm ist nicht signiert; Windows SmartScreen fragt deshalb
einmal nach („Weitere Informationen → Trotzdem ausführen“). Wer keinem
fertigen Programm trauen will, baut es selbst aus dem Quellcode – siehe
[Aus dem Quellcode](#aus-dem-quellcode).

## Funktionen

| | |
|---|---|
| **Installieren** | Ordner und ZIP-Dateien per Ziehen & Ablegen oder Auswahldialog · Vorschau, bevor etwas kopiert wird · bereits installierte Presets werden erkannt (gleiche UUID) · bei vorhandenen Dateien: überspringen, ersetzen (mit Sicherung) oder beide behalten · auf eine Gruppe gezogen, landet alles direkt darin |
| **Überblick** | Presets und Profile getrennt, mit Typ-Symbolen · Suche (Strg+F) · Filter (Strg+1/2/3) · Details: Name, Gruppe, Herkunft, Datei, UUID |
| **Verwalten** | Umbenennen (F2) · Löschen in den Papierkorb (Entf) · in andere Gruppe verschieben · Reihenfolge (Alt+↑/↓) · neue Gruppe aus Auswahl (Strg+G) · Gruppen zusammenführen · Mehrfachauswahl (Strg/Umschalt+Klick, Strg+A) |
| **Sicher** | Sicherungen der letzten 20 Aktionen · atomares Schreiben · ZIP-Prüfung gegen Pfadtricks, ZIP-Bomben und Riesendateien · arbeitet nur innerhalb der Adobe-Ordner |
| **Komfort** | Oberfläche auf Deutsch und Englisch (folgt Windows, in den Einstellungen umstellbar) · Hinweis, wenn Photoshop oder Lightroom läuft (sie zeigen Änderungen nach einem Neustart) · einstellbare Größe der Oberfläche · Logdatei für die Fehlersuche · Selbst-Update |

## So funktioniert es

Photoshop, Photoshop (Beta) und Lightroom Classic lesen Presets aus einem
gemeinsamen Ordner, `%APPDATA%\Adobe\CameraRaw\Settings`. Preset Manager
findet ihn selbst (ein anderer Ordner lässt sich in den Einstellungen wählen)
und legt jedes Paket in einen eigenen Unterordner. In den `.xmp`-Dateien
ändert die App nur Name, Gruppe und Sortierung; alle anderen Einstellungen
bleiben unberührt. Danach Photoshop bzw. Lightroom neu starten.

| Was | Wo |
|---|---|
| Presets und Profile | `%APPDATA%\Adobe\CameraRaw\Settings\<Paket>\` |
| Das Programm | `%LOCALAPPDATA%\PresetManagerApp\` |
| Einstellungen | `%APPDATA%\PresetManager\` |
| Sicherungen (letzte 20 Aktionen), Logdateien (7 Tage) | `%LOCALAPPDATA%\PresetManager\` |

Deinstallieren (Windows-Einstellungen → Apps) entfernt nur das Programm;
Presets, Einstellungen und Sicherungen bleiben.

## Aus dem Quellcode

Voraussetzung: [Rust](https://rustup.rs) (stable).

`Start.bat` im Projektordner:

- **Doppelklick:** öffnet das Fenster.
- **Ordner oder ZIP auf `Start.bat` ziehen:** öffnet gleich die Importvorschau.

Beim ersten Start wird das Programm übersetzt, das dauert einige Minuten.

### Ausprobieren ohne die eigenen Presets

`scripts/demo_bibliothek.py` legt einen Camera-Raw-Ordner mit ausgedachten
Demo-Presets an. Über die Umgebungsvariable `APM_CAMERA_RAW_DIR` arbeitet die
App dann nur dort; oben im Fenster steht „Anderer Ordner“. In der
Eingabeaufforderung (`cmd`) im Projektordner:

```bat
python scripts\demo_bibliothek.py C:\Temp\DemoCameraRaw
set APM_CAMERA_RAW_DIR=C:\Temp\DemoCameraRaw
Start.bat
```

### Aufbau

| Ordner | Inhalt |
|---|---|
| `crates/preset_core` | Bibliothek mit der gesamten Logik, ohne Oberfläche |
| `crates/preset_manager` | Das Fenster (`src/main.rs`, [egui](https://github.com/emilk/egui)) und das Konsolenwerkzeug `preset_cli` |
| `Planung` | Anforderungen, Architektur, Fehlerkonzept, Tests, Roadmap, Oberflächen-Entwurf |
| `scripts` | Demo-Bibliothek, Release-Skript |

Einstieg in die Planung: [Planung/00-Uebersicht.md](Planung/00-Uebersicht.md)

### Entwickeln

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Ein neues Release: Version in `Cargo.toml` erhöhen, Abschnitt in
`CHANGELOG.md` schreiben, committen und pushen, dann
`scripts/New-Release.ps1 -Publish`. Das setzt das Tag; GitHub baut
Installationsprogramm und Update-Pakete mit [Velopack](https://velopack.io)
und veröffentlicht sie (`.github/workflows/release.yml`). Versionen wie
`1.2.0-beta.1` werden Testversionen. Ohne `-Publish` entsteht ein Probe-Build;
mit `-Version 1.2.0-probe.1` und der Umgebungsvariable `APM_UPDATE_DIR` lassen
sich Installation und Update lokal ausprobieren.

### Konsolenwerkzeug

`preset_cli` listet ohne Angaben die installierten Einträge auf;
`importieren` zeigt eine Vorschau, kopiert wird erst mit `--ausfuehren`:

```bash
cargo run -p preset_manager --bin preset_cli -- importieren "Downloads\Paket.zip"
```

`cargo run -p preset_manager --bin preset_cli -- --hilfe` zeigt alle Angaben.

## Unterstützen

Wenn Preset Manager dir Zeit spart, **gib ihm einen ⭐ auf GitHub** – so finden
ihn auch andere Fotografinnen und Fotografen. Fehler und Ideen gern als
[Issue](https://github.com/suebi76/preset-manager/issues).

## Lizenz und Marken

MIT-Lizenz, siehe [LICENSE](LICENSE).

Adobe, Photoshop, Lightroom und Camera Raw sind Marken der Adobe Inc. Dieses
Projekt steht in keiner Verbindung zu Adobe und wird von Adobe weder
unterstützt noch geprüft.
