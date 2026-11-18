# Änderungen

Jede Version hat hier einen Abschnitt; er wird beim Veröffentlichen zum Text
des GitHub-Releases (`scripts/New-Release.ps1`).

## [1.6.0-beta.1] – 2026-11-18 – Testversion

### English

- **Before/after preview (test version).** The *Effect* page now shows a
  photo – your own or a built-in test chart – with the preset applied, and a
  slider to compare before and after; *Large* opens a big view. It is an
  **approximation**: Camera Raw processes raw files differently. Profile
  looks (color tables) are applied exactly; masks, calibration, sharpening
  and lens corrections are not reproduced – the preview says what's missing.
- This is a test version to find out whether the preview is useful. If it
  isn't, it will be removed again.

### Deutsch

- **Vorher/Nachher (Testversion).** Die Seite *Wirkung* zeigt jetzt ein Foto
  – ein eigenes oder eine eingebaute Testtafel – mit dem Preset, dazu einen
  Schieber zum Vergleichen; *Groß* öffnet eine große Ansicht. Das ist eine
  **Annäherung**: Camera Raw rechnet auf Rohdaten anders. Die Farbtabellen
  der Profile werden genau angewendet; Masken, Kalibrierung, Schärfen und
  Objektivkorrekturen nicht – die Vorschau sagt, was fehlt.
- Eine Testversion, um herauszufinden, ob die Vorschau nützt. Wenn nicht,
  fliegt sie wieder raus.

## [1.5.0] – 2026-11-18

### English

- **Activate and deactivate.** Keep every preset and profile in Preset
  Manager, but choose which ones Photoshop and Lightroom show. Deactivated
  items move to a folder Adobe doesn't read and come back to exactly the
  same place when activated – favorites stay. Switch them with the eye at the
  end of a row, right-click, Ctrl+D, the buttons in the details or by
  dragging onto a group; whole groups work too.
- Deactivated groups are listed separately under **DEACTIVATED**, shown
  paler and marked clearly, so it's always obvious what Adobe will show.

### Deutsch

- **Aktivieren und deaktivieren.** Alle Presets und Profile im Preset Manager
  behalten, aber selbst bestimmen, welche Photoshop und Lightroom zeigen.
  Deaktivierte liegen in einem Ordner, den Adobe nicht liest, und kommen
  beim Aktivieren an genau dieselbe Stelle zurück – Favoriten bleiben.
  Umschalten über das Auge am Zeilenende, Rechtsklick, Strg+D, die
  Schaltflächen in den Details oder Ziehen auf eine Gruppe; auch ganze
  Gruppen.
- Deaktivierte Gruppen stehen getrennt unter **DEAKTIVIERT**, blasser und
  deutlich markiert – so ist immer klar, was Adobe anzeigt.

## [1.4.0] – 2026-11-18

### English

- **What does this preset do?** Select a preset or profile and the new
  *Effect* page on the right shows it at a glance: one plain sentence
  (“Brighter, faded blacks, warmer, green desaturated, film grain.”) and
  every changed setting, arranged like in Camera Raw – sliders, the tone
  curve as a graph, color grading wheels, the color mixer, effects and
  masks. Name, group and file are on the *Manage* page.
- The hint about `crs:Group` under the group list is gone.

### Deutsch

- **Was macht dieses Preset?** Ein Preset oder Profil auswählen – die neue
  Seite *Wirkung* rechts zeigt es auf einen Blick: ein Satz in Klartext
  („Heller, matte Schwarztöne, wärmer, Grün entsättigt, Filmkorn.“) und
  alle geänderten Einstellungen, geordnet wie in Camera Raw – Regler, die
  Gradationskurve als Zeichnung, Color-Grading-Räder, Farbmischer, Effekte
  und Masken. Name, Gruppe und Datei stehen auf der Seite *Verwalten*.
- Der Hinweis zu `crs:Group` unter der Gruppenliste ist entfallen.

## [1.3.0] – 2026-11-18

### English

- **Finds every Camera Raw folder.** Besides your user folder, Preset Manager
  now also finds the preset folders next to your Lightroom Classic catalogs
  (*Store presets with this catalog*). Settings (gear icon) lists them all;
  one click switches.
- **Warning for Lightroom users:** if Lightroom Classic stores presets with
  the catalog, presets installed into the user folder show up only in
  Photoshop. The settings now say so.
- The folder with Adobe's own presets (`%ProgramData%\Adobe\CameraRaw`) is
  refused, so they can't be changed by mistake.

### Deutsch

- **Findet alle Camera-Raw-Ordner.** Neben dem Benutzerordner findet Preset
  Manager jetzt auch die Preset-Ordner neben Lightroom-Classic-Katalogen
  („Presets mit diesem Katalog speichern“). Die Einstellungen (Zahnrad)
  zeigen alle; ein Klick wechselt.
- **Hinweis für Lightroom:** Speichert Lightroom Classic Presets mit dem
  Katalog, sieht es Presets im Benutzerordner nicht – nur Photoshop. Die
  Einstellungen sagen das jetzt.
- Der Ordner mit Adobes mitgelieferten Presets
  (`%ProgramData%\Adobe\CameraRaw`) wird abgelehnt, damit sie nicht aus
  Versehen geändert werden.

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
