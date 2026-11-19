# Änderungen

Jede Version hat hier einen Abschnitt; er wird beim Veröffentlichen zum Text
des GitHub-Releases (`scripts/New-Release.ps1`).

## [1.9.0] – 2026-11-19

A maintenance release: no new features, but safer and easier to work on.

### English

- **Security:** every place that reads a whole preset file now stops at
  50 MB – a huge `.xmp` in the Camera Raw folder can no longer fill the
  memory. Profile color tables are checked before memory is reserved.
  Explorer is started by its full path. Every build checks all libraries
  for known vulnerabilities (`cargo audit`).
- **Fixes:**
  - Undoing a deletion of many presets reads the Recycle Bin only once,
    so it no longer takes long with a full bin.
  - “Replace” during import works again when the existing file cannot be
    read.
  - After choosing another Camera Raw folder, Undo no longer offers steps
    from the previous folder.
  - Undo that finds nothing to change now says so.
- **Code:** the largest source files are split by topic; no file has more
  than about 500 lines of code.

### Deutsch

Eine Wartungsversion: keine neuen Funktionen, dafür sicherer und leichter
weiterzuentwickeln.

- **Sicherheit:** Alle Stellen, die eine Preset-Datei ganz lesen, hören bei
  50 MB auf – eine riesige `.xmp`-Datei im Camera-Raw-Ordner kann den
  Speicher nicht mehr füllen. Farbtabellen von Profilen werden geprüft,
  bevor Speicher reserviert wird. Der Explorer wird mit vollem Pfad
  gestartet. Jeder Build prüft alle Bibliotheken auf bekannte Lücken
  (`cargo audit`).
- **Behoben:**
  - Rückgängig nach dem Löschen vieler Presets liest den Papierkorb nur
    noch einmal und dauert bei vollem Papierkorb nicht mehr lange.
  - „Ersetzen“ beim Import klappt wieder, wenn die vorhandene Datei nicht
    lesbar ist.
  - Nach dem Wechsel des Camera-Raw-Ordners bietet Rückgängig keine
    Schritte aus dem vorigen Ordner mehr an.
  - Rückgängig ohne etwas zu ändern sagt das jetzt.
- **Code:** Die größten Quelldateien sind nach Themen aufgeteilt; keine
  Datei hat mehr als rund 500 Zeilen Programmcode.

## [1.8.0] – 2026-11-19

### English

- **Undo and redo.** Every change made in Preset Manager can be taken back:
  renaming, moving, reordering, groups, activating and deactivating,
  deleting (the files come back out of the Recycle Bin) and installing.
  Use **Ctrl+Z** and **Ctrl+Y** (or Ctrl+Shift+Z), the arrows at the top
  right – their tooltip says what will be undone – or **Undo** in the
  notice that appears after each action. Up to 20 steps, for as long as the
  app is open.
- Undo only touches files that are still exactly as the action left them.
  If Camera Raw or you changed a file in the meantime, it stays as it is
  and the app says which one.
- Notices with an Undo button stay for 10 seconds and do not disappear
  while the mouse is over them.

### Deutsch

- **Rückgängig und Wiederholen.** Jede Änderung im Preset Manager lässt sich
  zurücknehmen: Umbenennen, Verschieben, Reihenfolge, Gruppen, Aktivieren
  und Deaktivieren, Löschen (die Dateien kommen aus dem Papierkorb zurück)
  und Installieren. Mit **Strg+Z** und **Strg+Y** (oder
  Strg+Umschalt+Z), mit den Pfeilen oben rechts – ihr Tooltip sagt, was
  zurückgenommen wird – oder mit **Rückgängig** in der Einblendung nach
  jeder Aktion. Bis zu 20 Schritte, solange die App geöffnet ist.
- Rückgängig ändert nur Dateien, die noch genau so sind, wie die Aktion sie
  hinterlassen hat. Hat Camera Raw oder jemand anderes eine Datei
  inzwischen geändert, bleibt sie, wie sie ist, und die App nennt sie.
- Einblendungen mit „Rückgängig“ bleiben 10 Sekunden stehen und
  verschwinden nicht, solange die Maus darüber ist.

## [1.7.0] – 2026-11-18

### English

- **Export as ZIP.** Save a selection or a whole group as a ZIP file, to back
  it up, pass it on or move it to another computer: right-click a preset or a
  group, use the button in the details, or press **Ctrl+E** (the selection,
  or the open group if nothing is selected). Each group becomes a folder in
  the ZIP; the files are copied unchanged. Preset Manager installs the ZIP
  again with its groups, and Photoshop and Lightroom Classic can import it
  as well.

### Deutsch

- **Als ZIP exportieren.** Eine Auswahl oder eine ganze Gruppe als ZIP-Datei
  speichern, zum Sichern, Weitergeben oder Umziehen auf einen anderen
  Rechner: per Rechtsklick auf ein Preset oder eine Gruppe, mit der
  Schaltfläche in den Details oder mit **Strg+E** (die Auswahl, ohne Auswahl
  die geöffnete Gruppe). Jede Gruppe wird im ZIP ein Ordner, die Dateien
  bleiben unverändert. Preset Manager installiert das ZIP samt Gruppen
  wieder; Photoshop und Lightroom Classic können es ebenfalls importieren.

## [1.6.1] – 2026-11-18

### English

- On the *Effect* page, **Expand all** and **Collapse all** above the
  settings sections (Basic, Tone curve, Color grading …) open or close all
  of them at once.
- The note below the before/after preview no longer says “Test version”.

### Deutsch

- Auf der Seite *Wirkung* öffnen oder schließen **Alle aufklappen** und
  **Alle zuklappen** über den Bereichen (Grundeinstellungen,
  Gradationskurve, Color Grading …) alle auf einmal.
- Der Hinweis unter der Vorher/Nachher-Vorschau sagt nicht mehr
  „Testversion“.

## [1.6.0] – 2026-11-18

### English

- **Before/after preview.** The *Effect* page shows a photo – your own or a
  built-in test chart – with the preset applied, plus a slider to compare
  before and after; a button opens a large view. It is an **approximation**:
  Camera Raw processes raw files differently. Profile looks (color tables)
  are applied exactly; what isn't reproduced (masks, calibration,
  sharpening, lens corrections) is listed when you hover “Approximation”.
- **Tidier details panel.** The preset name is the title, active/deactivated
  is a simple switch, the plain-language summary comes first, every slider
  fits on one line, and actions are grouped with “Move to Recycle Bin” set
  apart.
- **Scrolling is visible.** Every scrollable area shows a scroll bar at all
  times, and content fades out at the edge where there's more.
- **Settings in small windows.** The settings dialog adapts to the window
  size; its title and “Close” always stay visible.

### Deutsch

- **Vorher/Nachher.** Die Seite *Wirkung* zeigt ein Foto – ein eigenes oder
  eine eingebaute Testtafel – mit dem Preset, dazu einen Schieber zum
  Vergleichen; eine Schaltfläche öffnet eine große Ansicht. Das ist eine
  **Annäherung**: Camera Raw rechnet auf Rohdaten anders. Die Farbtabellen
  der Profile werden genau angewendet; was nicht nachgebildet wird (Masken,
  Kalibrierung, Schärfen, Objektivkorrekturen), steht im Tooltip von
  „Annäherung“.
- **Aufgeräumte Details.** Der Name des Presets ist der Titel, aktiv/
  deaktiviert ein einfacher Schalter, der Satz in Klartext steht vorn, jeder
  Regler passt in eine Zeile, Aktionen sind gebündelt und „In den
  Papierkorb“ abgesetzt.
- **Rollen ist sichtbar.** Jeder Bereich, der rollt, zeigt immer eine
  Bildlaufleiste, und der Inhalt wird am Rand ausgeblendet, wo es weitergeht.
- **Einstellungen in kleinen Fenstern.** Der Dialog passt sich der
  Fenstergröße an; Titel und „Schließen“ bleiben immer sichtbar.

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
