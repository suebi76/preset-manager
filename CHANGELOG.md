# Änderungen

Jede Version hat hier einen Abschnitt; er wird beim Veröffentlichen zum Text
des GitHub-Releases (`scripts/New-Release.ps1`).

## [1.2.0] – 2026-11-18

### English

- **English interface.** Preset Manager now speaks English and German. It
  follows the Windows language; Settings (gear icon) → *Language* switches
  it at any time.
- Error messages – e.g. for invalid names or rejected ZIP files – appear in
  the chosen language, too.

### Deutsch

- **Englische Oberfläche.** Preset Manager spricht jetzt Englisch und
  Deutsch. Er richtet sich nach der Sprache von Windows; Einstellungen
  (Zahnrad) → *Sprache* stellt jederzeit um.
- Auch Fehlermeldungen – etwa zu unzulässigen Namen oder abgelehnten
  ZIP-Dateien – erscheinen in der gewählten Sprache.

## [1.1.0] – 2026-11-18

### Neu

- **Installationsprogramm:** `PresetManager-Setup.exe` richtet Preset Manager
  ohne Administratorrechte für den angemeldeten Benutzer ein, mit Eintrag im
  Startmenü. Alternativ gibt es die portable Fassung als ZIP.
- **Updates:** Preset Manager sieht nach neuen Versionen und installiert sie
  auf Wunsch selbst. In den Einstellungen (Zahnrad) wählbar: Hinweis zeigen
  und mit einem Klick installieren (empfohlen), automatisch beim Schließen
  installieren oder nur auf Nachfrage suchen. Testversionen lassen sich
  zusätzlich einschalten.

### Hinweis

Version 1.0.0 gab es nur als ZIP-Datei ohne Selbst-Update. Wer sie benutzt,
installiert 1.1.0 einmal mit `PresetManager-Setup.exe`; danach kommen Updates
von selbst. Einstellungen, Sicherungen und Logdateien bleiben erhalten.

## [1.0.0] – 2026-11-18

Erste Version: Presets und Profile als Ordner oder ZIP installieren
(Vorschau, Konfliktregeln, Sicherung), anzeigen, suchen, umbenennen,
löschen (Papierkorb), verschieben, sortieren, Gruppen anlegen und
zusammenführen – per Ziehen & Ablegen, Rechtsklick oder Tastatur.
Einstellungen, Logdatei, App-Symbol.
