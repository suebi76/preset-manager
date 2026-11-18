# 04 – Fehlerbehandlung und Schutz vor Datenverlust

Die App verändert gekaufte und selbst erstellte Presets. Ein Fehler darf
höchstens dazu führen, dass eine Aktion nicht stattfindet – nie dazu, dass Daten
weg oder beschädigt sind.

## Grundsätze

1. **Fehler sind Werte.** Jede Funktion, die scheitern kann, gibt
   `Result<_, CoreError>` zurück. Kein `unwrap`, kein `panic`.
2. **Ein Fehler stoppt nicht alles.** Bei 100 Presets und einer defekten Datei
   werden 99 verarbeitet; die eine steht mit Grund im Bericht.
3. **Jeder Fehler nennt die betroffene Datei** und was der Nutzer tun kann.
4. **Nichts wird endgültig gelöscht.** Löschen = Papierkorb. Überschreiben =
   vorher sichern.
5. **Erst prüfen, dann handeln.** Alle Vorbedingungen werden beim Planen geprüft,
   bevor die erste Datei angefasst wird.

## Fehlerarten (`CoreError`)

| Variante | Wann | Meldung an den Nutzer (sinngemäß) |
|---|---|---|
| `AdobeFolderNotFound` | Camera-Raw-Ordner existiert nicht | „Camera Raw wurde nicht gefunden. Ordner in den Einstellungen wählen.“ |
| `Io { path, source }` | Lesen/Schreiben scheitert | „Datei … konnte nicht gelesen/geschrieben werden: …“ |
| `AccessDenied { path }` | Schreibschutz, Datei gesperrt | „Kein Zugriff auf … – ist die Datei in Benutzung?“ |
| `InvalidArchive { path, reason }` | ZIP defekt oder passwortgeschützt | „Das Archiv … kann nicht gelesen werden.“ |
| `UnsafeArchiveEntry { entry }` | ZIP-Eintrag zeigt aus dem Zielordner hinaus | „Das Archiv enthält einen unzulässigen Pfad und wurde nicht entpackt.“ |
| `ArchiveTooLarge { path, reason }` | Größe, Eintragszahl oder Verschachtelung über Grenze | „Das Archiv … ist zu groß (…) und wurde abgelehnt.“ |
| `UnsupportedSource { path }` | Quelle ist weder Ordner noch ZIP | „… ist weder ein Ordner noch ein ZIP-Archiv.“ |
| `AlreadyExists { path }` | Zieldatei existiert (intern, führt zur Konfliktregel) | – |
| `OutsideAdobeFolder { path }` | Zielpfad außerhalb der Adobe-Ordner (Programmierfehler) | „Interner Fehler – es wurde nichts geschrieben.“ |
| `NotAPreset { path }` | XMP ohne `crs:PresetType` | wird im Bericht als „übersprungen“ geführt |
| `MalformedXmp { path, reason }` | XML nicht lesbar | „… ist beschädigt und wurde übersprungen.“ |
| `InvalidName { name, reason }` | leer, zu lang, unzulässige Zeichen | „Der Name … ist nicht zulässig: …“ |
| `NameConflict { name }` | Zielname existiert bereits | „In dieser Gruppe gibt es bereits ein Preset namens …“ |
| `NoPresetsFound { source }` | Quelle enthält keine Presets | „In … wurden keine Presets gefunden.“ |
| `Trash { path, reason }` | Papierkorb nicht verfügbar | „… konnte nicht in den Papierkorb verschoben werden.“ Kein stilles endgültiges Löschen |

Die Oberfläche übersetzt `CoreError` in Text (`texts.rs`). Technische Details
landen zusätzlich in der Logdatei.

## Schutzmaßnahmen im Einzelnen

### Import

| Risiko | Maßnahme |
|---|---|
| ZIP mit Pfaden wie `..\..\Windows\…` („Zip Slip“) | Jeder Eintrag wird geprüft; nur relative Pfade ohne `..` innerhalb des Temp-Ordners sind erlaubt |
| „ZIP-Bombe“ | Grenzen für entpackte Gesamtgröße (500 MB), Dateianzahl (10 000), Verschachtelungstiefe (3) |
| Entpacken landet halb im Adobe-Ordner | Entpackt wird immer in einen temporären Ordner; kopiert wird erst nach erfolgreicher Prüfung |
| macOS-Müll im ZIP (`__MACOSX`, `._datei`) | wird ignoriert |
| Foto-Sidecar statt Preset | Prüfung auf `crs:PresetType` |
| Gleicher Dateiname existiert | Konfliktauswahl; Standard „überspringen“ |
| Gleiche UUID existiert schon an anderer Stelle | Eintrag ist in der Vorschau abgewählt (sonst doppelt in Camera Raw); wieder auswählbar |
| Abbruch mitten im Kopieren | Datei wird unter temporärem Namen geschrieben und erst am Ende umbenannt – es gibt nie halbe Presets |
| Datenträger voll | Atomares Schreiben verhindert halbe Dateien; der Fehler steht je Datei im Bericht (eine Vorab-Prüfung des freien Platzes gibt es nicht, siehe 06) |
| Ungültige Windows-Namen im ZIP (`CON`, `a:b`, Punkt am Ende) | Entpackt wird unter durchnummerierten Namen; der Zielname wird bereinigt (`import::naming`) |
| Riesige `.xmp`-Datei in einem Quellordner | Dateien über 50 MB werden nicht gelesen |
| Paket enthält jedes Preset doppelt (z. B. „For Lightroom CC“ und „For Lightroom Classic“) | Gleiche UUID im Import → nur einmal installiert; scheitert die erste Kopie, springt die zweite ein |

### Ändern (Umbenennen, Gruppieren, Sortieren)

| Risiko | Maßnahme |
|---|---|
| XMP nach Änderung defekt | Neue Datei wird nach dem Schreiben erneut gelesen und geprüft; erst dann ersetzt sie das Original |
| Absturz während des Schreibens | Atomar schreiben: temporäre Datei im selben Ordner, dann umbenennen |
| Änderung war ein Versehen | Original wird vorher in den Sicherungsordner kopiert |
| Sonderzeichen im Namen (`&`, `<`, `"`) | XML-Kodierung übernimmt `quick-xml`; Test mit solchen Namen |
| Unzulässiger Dateiname (`:`, `?`, `CON`, Punkt am Ende) | Der Anzeigename darf fast alles; der Dateiname wird daraus bereinigt abgeleitet |
| Gruppe mit 200 Presets, Nr. 150 scheitert | Bericht nennt die gescheiterten; die bereits geänderten bleiben gültig, Wiederholen ist gefahrlos |

### Löschen

| Risiko | Maßnahme |
|---|---|
| Versehentliches Löschen | Bestätigungsdialog nennt Anzahl und Namen; Dateien gehen in den Windows-Papierkorb |
| Falsche Dateien gelöscht | Gelöscht werden nur Dateien, die die App vorher selbst als Preset eingelesen hat, und nur innerhalb der bekannten Adobe-Ordner. Diese Prüfung sitzt in `safe_fs`, nicht in der Oberfläche |
| Leerer Ordner bleibt zurück | wird nur entfernt, wenn er wirklich leer ist |

### Sicherungen

- Ort: `%LOCALAPPDATA%\PresetManager\backups\<Datum_Uhrzeit>\`
- Inhalt: Originaldateien mit relativer Pfadstruktur
- Aufbewahrung: die letzten 20 Aktionen, ältere werden entfernt
- Grundlage für „Rückgängig“ (F-29)

### Laufende Adobe-Anwendungen

Die App blockiert nicht, wenn Photoshop oder Lightroom läuft, zeigt aber einen
Hinweis. Gesperrte Dateien führen zu `AccessDenied` für genau diese Datei.

## Protokollierung

- Logdatei unter `%LOCALAPPDATA%\PresetManager\logs\preset-manager.JJJJ-MM-TT.log`,
  täglicher Wechsel, die letzten 7 Dateien bleiben (`logging.rs`).
- Protokolliert wird: jede Aktion mit Quelle/Ziel, jeder Fehler mit Ursache.
- „Logdateien → Ordner öffnen“ im Einstellungsdialog.
- Auch Abstürze werden protokolliert, bevor die übliche Meldung erscheint.
- Kann die Logdatei nicht angelegt werden, läuft die App ohne sie weiter.

## Einstellungen

- `%APPDATA%\PresetManager\einstellungen.txt` (`schlüssel=wert`):
  gewählter Camera-Raw-Ordner und Größe der Oberfläche.
- Fehlt die Datei oder ist sie kaputt, gelten die Voreinstellungen.
- Ein gewählter Ordner wird vor dem Übernehmen geprüft; wird er später
  unerreichbar (z. B. getrenntes Laufwerk), zeigt die App das an und bietet
  „anderen Ordner wählen“ und „wieder automatisch suchen“ an – sie weicht nie
  still auf einen anderen Ordner aus.
- Die Umgebungsvariable `APM_CAMERA_RAW_DIR` geht der Einstellung vor.

## Absicherung im Code

In `Cargo.toml` des Workspace:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

Damit schlägt der Build fehl, sobald jemand eine dieser Abkürzungen einbaut.
In Testcode sind sie erlaubt.
