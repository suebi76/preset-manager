# 06 – Roadmap

Jeder Meilenstein endet mit lauffähigem, getestetem Stand. Die Reihenfolge ist
so gewählt, dass zuerst die Annahmen geprüft und dann die Logik ohne Oberfläche
gebaut wird – Fehler in der Logik fallen so früh und billig auf.

## M0 – Grundlagen klären ✔ teilweise erledigt

- [x] Adobe-Ordner und XMP-Aufbau auf dem Rechner geprüft (siehe 02)
- [x] Projektgerüst angelegt, `cargo check` und `cargo test` laufen
- [x] Versuche P-1 bis P-5 aus 02 durchgeführt (Photoshop 2027, Beta und Lightroom Classic)
- [x] Testdaten (`fixtures`) erstellt – von Hand geschrieben statt in Camera Raw exportiert
- [x] Lizenz festlegen (MIT), `git init`, erstes Commit (M6)

**Fertig, wenn:** alle fünf Fragen beantwortet sind und die Architektur ggf. angepasst ist.

## M1 – Lesen (`preset_core`) ✔ erledigt am 17.11.2026

- [x] `adobe::locations` vollständig, mit manueller Überschreibung (`resolve`)
- [x] `adobe::locations::contains` als Schutzprüfung nach Windows-Regeln
- [x] `preset::xmp::read_metadata`
- [x] `library::scan`: `Settings` und `ImportedSettings` einlesen, nach Gruppen ordnen
- [x] Fehlerarten und `OperationReport`

**Fertig, wenn:** ein Integrationstest einen Testordner einliest und Gruppen,
Namen und Anzahl stimmen; defekte Dateien erscheinen im Bericht.

**Ergebnis:** 33 automatische Tests grün (30 Unit-, 3 Integrationstests). Lesetest am echten Ordner: 410
Presets in 22 Gruppen, 0 Fehler, Adobe-Ordner nachweislich unverändert.
Code-Review mit 10 Befunden, 8 behoben, 2 auf später verschoben (P-5,
Test für unlesbare Ordner).

## M2 – Importieren (`preset_core`) ✔ erledigt am 17.11.2026

- [x] `import::source` für Ordner und ZIP (auch ZIP im Ordner, ZIP im ZIP)
- [x] `import::archive` mit allen Schutzmaßnahmen
- [x] `import::plan`: Plan erstellen, Konfliktregeln, ausführen
- [x] `import::naming`: gültige Windows-Namen
- [x] `safe_fs::write_atomic`, dazu `safe_fs::backup` (aus M3 vorgezogen, weil „Ersetzen“ sichern muss)
- [x] `preset::xmp::replace_uuid` für „beide behalten“
- [x] Konsolenbefehl `preset_manager importieren` (Vorschau; Installieren nur mit `--ausfuehren`)

**Fertig, wenn:** alle Pflichtfälle „Import“ aus 05 grün sind.

**Ergebnis:** 84 automatische Tests grün (58 Unit-, 26 Integrationstests).
Probelauf mit zwei echten Paketen als ZIP: Vorschau gegen den echten Ordner
(nachweislich unverändert), Installation in eine Kopie mit allen drei
Konfliktregeln; installierte Dateien byte-gleich mit dem Original, Kopie bei
„beide behalten“ unterscheidet sich nur in der UUID. Code-Review mit 8
Befunden, 5 behoben, 3 bewusst zurückgestellt (Schönheitsfehler bei
Nummerierung, doppelte Hilfsfunktion, Zeitstempel in UTC).

**Abweichungen vom Plan:**
- Freien Speicherplatz prüft die App nicht vorab (die Standardbibliothek
  kann es nicht, Presets sind wenige KB groß). Durch atomares Schreiben
  entsteht trotzdem nie eine halbe Datei; ein voller Datenträger erscheint
  als Fehler im Bericht.
- Sicherungsordner tragen die Uhrzeit in UTC.
- Einzelne `.xmp`-Dateien als Quelle werden noch abgelehnt (nur Ordner und ZIP).

**Entschieden am 17.11.2026:** Ist ein Preset mit derselben UUID schon an
anderer Stelle installiert (meist über Adobes Importdialog in
`ImportedSettings`), ist es in der Vorschau **abgewählt** – sonst zeigte
Camera Raw es doppelt. Der Nutzer kann es wieder auswählen
(`ImportPlan::set_selected`, Konsole: `--auch-installierte`). Geprüft mit
dem Beispielpaket in `Beispielpreset/`: alle 26 Presets abgewählt, echter
Ordner unverändert.

**Start ohne Unterordner:** `Start.bat` im Projektordner. Doppelklick zeigt
die installierten Presets; ein Ordner oder ZIP, auf die Datei gezogen,
zeigt die Vorschau und fragt vor dem Installieren nach.

Stand der Tests danach: 87 (58 Unit-, 29 Integrationstests).

## M3 – Verwalten (`preset_core`) ✔ erledigt am 17.11.2026

- [x] `preset::xmp_edit::rewrite_metadata` (Name, Gruppe, Sortierschlüssel; `replace_uuid` hierher verschoben)
- [x] `safe_fs::Trash` mit `RecycleBin` (Windows-Papierkorb), `prune_backups` (letzte 20 behalten)
- [x] `library::ops`: löschen (auch ganze Gruppen), Preset umbenennen, Gruppe umbenennen, verschieben, sortieren

**Fertig, wenn:** alle Pflichtfälle „XMP“ und „Verwalten“ aus 05 grün sind.

**Ergebnis:** 125 automatische Tests grün (78 Unit-, 47 Integrationstests).
Zusätzlich `tests/real_data.rs` (nur lesend, von Hand gestartet): alle 410
echten Presets lassen sich ändern und wieder lesen; Name und Gruppe hin und
zurück ergeben jede Datei **byte-gleich** zurück. Code-Review mit 8 Befunden,
alle 8 behoben – darunter: Felder werden in jedem obersten
`rdf:Description` geändert; Pfadprüfungen (`AdobeLocations::ensure_contains`,
`relative_path`, `same_path`) und `safe_fs::write_to_free_name` gibt es nur
noch einmal für Import und Verwalten.

**Festlegungen dabei:**
- Gleiche Namen werden ohne Rücksicht auf Groß-/Kleinschreibung verglichen
  („Warm“ und „WARM“ in einer Gruppe gelten als Konflikt).
- Gruppe umbenennen auf eine vorhandene Gruppe wird abgelehnt; zum
  Zusammenführen dient „Verschieben“.
- Umbenennen ändert auch den Dateinamen; Gruppe und Reihenfolge ändern nur
  den Dateiinhalt.
- Sortieren schreibt `0001`, `0002` … und lässt Dateien mit schon passendem
  Schlüssel unangetastet.

**Erkenntnis aus dem Test mit `Beispielpreset/`:** Viele Pakete enthalten
**Profile** (`crs:PresetType="Look"`), keine Presets. Camera Raw zeigt sie im
Profilbrowser („Bearbeiten → Profil → Durchsuchen“), nicht in der
Vorgabenliste. Die Konsole sagt das jetzt nach jedem Import; die Oberfläche
trennt Presets und Profile ohnehin (07).

### Nachtrag zu M3 (17.11.2026): Presets und Profile getrennt

Für die Bereiche der Oberfläche (F-32) gruppiert `Library` jetzt nach
**Bereich und Name** (`Section::Presets` / `Section::Profiles`). Vorher
landeten z. B. die 20 Presets und 20 Profile von „Filmklassiker Kodak-Stil
v3.1“ in einer gemeinsamen Gruppe. Gruppe umbenennen/löschen und die
Namensprüfung arbeiten nur noch im jeweiligen Bereich. Am echten Ordner:
267 Presets, 143 Profile, 23 Gruppen. 131 Tests grün.

## M4 – Oberfläche: Anzeigen und Importieren ◐ gebaut, Abnahme offen

Gestaltung nach [07-Oberflaeche.md](07-Oberflaeche.md).

- [x] Klickbarer HTML-Entwurf zur Abstimmung des Aussehens (abgenommen 17.11.2026)
- [x] Thema (`ui/theme.rs`), Schrift (Segoe UI Variable), Symbole (Phosphor)
- [x] Fenster mit Gruppen- und Presetliste
- [x] Drag & Drop, Auswahldialog (Ordner oder mehrere ZIPs)
- [x] Importvorschau mit Häkchen und Konfliktregel, Ergebnis als Einblendung
- [x] Hintergrund-Thread (`worker.rs`), Anzeige „wird gelesen/installiert“
- [x] Hinweis bei laufendem Photoshop/Lightroom (`adobe::running`, alle 5 s)
- [x] Bereiche **Presets** und **Profile** getrennt, Filter Alle/Presets/Profile (F-32)
- [x] **Typ-Symbole** in Farbe für jeden Eintrag (F-31)
- [x] Suche (F-28), „Im Explorer zeigen“ (F-45)
- [x] Ordner/ZIP auf eine bestimmte Gruppe ziehen → Import in diese Gruppe
  (F-12; `ImportPlan::set_target_group`)
- [x] Nicht lesbare Dateien als eigener Bereich „NICHT LESBAR“
- [x] `Start.bat` startet das Fenster; Konsolenwerkzeug jetzt als eigenes
  Programm `preset_cli`
- [x] Abnahme Photoshop 2027 und Photoshop (Beta) (17.11.2026, siehe unten)
- [ ] Gegenprobe Lightroom Classic (durch Versuch P-1 abgedeckt, nicht erneut geprüft)
- [ ] Ziehen aus dem Explorer von Hand testen (ins Fenster und auf eine Gruppe)

**Fertig, wenn:** die ersten drei Punkte der manuellen Abnahme (05) bestanden sind.

**Abnahme 17.11.2026 (per Bildschirmsteuerung, eigenes Testpaket):**
- Fenster: Bereiche Presets/Profile, Symbole, Gruppenwahl, Details, Suche
  („bondi“ → 1 Treffer), Filter Strg+3 – alles wie im Entwurf.
- Import über Startparameter (= Ordner auf `Start.bat` gezogen): Vorschau
  korrekt („2 von 2“, beide „neu“, Konfliktregel ausgegraut), nach
  „2 installieren“ neue Gruppe ausgewählt, Einblendung mit Neustart-Hinweis.
  Auf der Platte genau zwei neue Dateien in `Settings\APM-Abnahme`,
  byte-gleich mit der Quelle.
- **Photoshop 2027 und Photoshop (Beta)** zeigen Gruppe und beide Presets.
- Hinweisleiste erscheint bei laufendem Photoshop und nennt die Beta
  richtig („Photoshop (Beta) läuft …“); verschwindet nach dem Schließen.
- Zweiter Import desselben Pakets: beide „Datei vorhanden“, „überspringen“
  → „Nichts zu installieren“, „beide behalten“ → „2 installieren“.
- Testpaket danach entfernt; Preset-Ordner wieder im Ausgangszustand.
- Gefunden und behoben: Ordner-Quellen zeigten ein ZIP-Symbol.

**Technik:** egui/eframe 0.36 (passend zu `egui-phosphor` 0.14), OpenGL,
`rfd` 0.18, `sysinfo` 0.40. 137 automatische Tests grün, Clippy ohne Befund.

## M5 – Oberfläche: Verwalten ◐ gebaut, Abnahme an Kopie bestanden

- [x] Mehrfachauswahl (Klick, Strg+Klick, Umschalt+Klick, Strg+A, Pfeiltasten)
- [x] Löschen mit Bestätigung (Entf, Kontextmenü, Detailspalte) – in den Papierkorb
- [x] Umbenennen in der Zeile (F2, Doppelklick) für Presets und Gruppen;
  alter Name markiert, Enter übernimmt, Esc bricht ab
- [x] Verschieben über die Detailspalte („Gruppe“ bzw. „Verschieben nach“)
- [x] **Neue Gruppe** über „＋“ und „Neue Gruppe aus Auswahl“ (Strg+G); leere
  Gruppen in `%APPDATA%\PresetManager\leere-gruppen.txt` gemerkt (F-33)
- [x] **Ziehen & Ablegen** auf Gruppen und „＋ Neue Gruppe“; ungültige Ziele
  abgedunkelt, Begründung am Mauszeiger (F-34)
- [x] Sortieren per Ziehen (Einfügelinie) und Alt+↑/↓ (F-27)
- [x] Gruppen zusammenführen per Ziehen, mit Rückfrage (F-35)
- [x] Kontextmenüs für Einträge und Gruppen
- [x] `APM_CAMERA_RAW_DIR` für einen anderen Camera-Raw-Ordner (F-43,
  vorerst als Umgebungsvariable; Einstellungsdialog seit M6)
- [ ] Manuelle Abnahme am echten Ordner mit Neustart in Photoshop/Lightroom

**Fertig, wenn:** die gesamte manuelle Abnahme bestanden ist.

**Abnahme 17.11.2026 (Bildschirmsteuerung, an einer Kopie des
Camera-Raw-Ordners):** Umbenennen (Eintrag und Gruppe), Ziehen auf eine
Gruppe, Ablehnung Preset → Profil-Gruppe mit Begründung, Umsortieren mit
Einfügelinie, Alt+↓, Mehrfachauswahl, Strg+G mit anschließendem Umbenennen,
Zusammenführen mit Rückfrage, „＋“ leere Gruppe und Hineinziehen,
Kontextmenü → Papierkorb mit Bestätigung – alles wie geplant. Echter Ordner
nachweislich unverändert.

**Dabei gefunden und behoben:**
- Strg+Klick ersetzte die Auswahl, wenn Strg sehr schnell losgelassen wurde →
  Zusatztasten kommen jetzt aus dem Klick-Ereignis.
- Beim Umbenennen war der alte Name nicht markiert (Text wurde angehängt).
- Verschobene Presets nahmen ihren Sortierschlüssel mit und standen in der
  neuen Gruppe vorn → Sortierschlüssel wird beim Verschieben geleert
  (`ops::move_to_group`, Test `verschieben_leert_den_sortierschluessel`).
- Code-Review: 7 Befunde, 4 behoben (blockierte Tastenkürzel bei
  unsichtbarem Umbenennen, Folgezustand bei abgelehnter Aktion, verlorene
  Auswahl, Zusammenführen ohne Rückmeldung); zurückgestellt: leere Gruppen
  gelten für alle Camera-Raw-Ordner, Kopieren der Liste je Bild, Sprung zur
  ersten Gruppe nach dem Leeren einer Gruppe.

**Offen:** Zoomen per Strg+Plus/Minus (egui-eigene Funktion) auf dem
4K-Bildschirm von Hand prüfen; ggf. Zoomstufe merken (M6).

Stand: 140 automatische Tests grün, Clippy ohne Befund.

## M6 – Version 1.0 ◐ gebaut, Veröffentlichung offen

- [x] Name „Preset Manager“, MIT-Lizenz, Version 1.0.0, lokales git-Repository
- [x] Einstellungsdialog (Zahnrad, Strg+,): Camera-Raw-Ordner wählen oder
      automatisch, Größe der Oberfläche, Sicherungs- und Logordner öffnen,
      „Über“ mit Version, Lizenz und Markenhinweis
- [x] Logdatei (täglich, 7 Dateien), Aufträge, Ergebnisse, Fehler, Abstürze
- [x] App-Symbol (Fenster, Taskleiste, .exe) mit Versionsangaben in der Datei
- [x] README mit Bildschirmfoto (Demo-Bibliothek), Anleitung, Markenhinweis
- [x] Release-Workflow: Tag `v*` → Tests, Build, ZIP mit Prüfsumme, GitHub-Release
- [x] Abschließendes Code- und Sicherheitsreview
- [ ] Auf GitHub veröffentlichen (Repository anlegen, pushen, Tag `v1.0.0`)

**Abnahme 18.11.2026 (Bildschirmsteuerung, Demo-Bibliothek):** Dialog über
Zahnrad und Strg+,; Ordner aus Umgebungsvariable (Wahl gesperrt), aus der
Einstellungsdatei („Automatisch suchen“ wechselt zur echten Bibliothek, Chip
verschwindet), fehlender Ordner (Fehlerbild mit beiden Auswegen); Zoom aus
der Datei, per Stufen und Strg+0, gespeichert; Logdatei geschrieben;
Symbol und Versionsangaben in der .exe.

**Dabei gefunden und behoben:**
- Der Zoom-Schieberegler wuchs beim Ziehen mit der Oberfläche unter dem
  Mauszeiger mit und lief bis 200 % davon → feste Stufen mit − / +.
- Bei großem Zoom passte der Dialog nicht ins Fenster → rollt jetzt.
- Einstellungsdatei war eingerückt (Zeilenfortsetzung verloren) → Test prüft das.
- Fehlerbild bei gewähltem Ordner sprach von fehlendem Photoshop → eigener Hinweis.
- Bildschirmfoto hätte den Windows-Benutzernamen im Dateipfad gezeigt →
  Demo-Bibliothek unter neutralem Pfad.
- Release-Workflow: Tag-Name stand direkt im Skripttext (Code-Einschleusung
  über Tag-Namen möglich) → nur noch über Umgebungsvariable.
- `APM_CAMERA_RAW_DIR` mit Leerzeichen am Ende (`set X=… && …` in cmd) →
  wird abgeschnitten; nicht lesbare Zeichen führen nie zum echten Ordner.

## 1.1 – Installationsprogramm und Selbst-Update ✔ 18.11.2026

Aufbau wie in CloudDrive-Sync: Velopack 1.2.161 (Rust-Bibliothek und
Packwerkzeug `vpk` in derselben Version).

- [x] `PresetManager-Setup.exe` (ohne Administratorrechte, Startmenü und
      Desktop) und portable ZIP; Installationsordner
      `%LOCALAPPDATA%\PresetManagerApp` – getrennt von Sicherungen und Logs,
      die eine Deinstallation deshalb nicht löscht
- [x] Updates aus den GitHub-Releases: „Hinweis zeigen“ (Standard),
      „Automatisch installieren“ (beim Schließen), „Nur wenn ich nachsehe“,
      dazu „Testversionen erhalten“ (Vorabversionen); Delta-Pakete
- [x] `scripts/New-Release.ps1` (Probe-Build, `-Publish` setzt das Tag),
      Release-Workflow baut und veröffentlicht, `CHANGELOG.md` liefert den Text
- Code-Signatur: bewusst nicht (Entscheidung 18.11.2026). SmartScreen warnt
  beim ersten Start; wer dem Programm nicht traut, baut es aus dem Quellcode.

**Abnahme 18.11.2026 (lokale Probeversionen über `APM_UPDATE_DIR`):**
Installation von probe.1; „Automatisch“: probe.2 im Hintergrund geladen
(Delta 38 KB) und beim Schließen still installiert; „Hinweis“: Hinweisleiste,
Klick auf „Jetzt installieren“ → probe.3 installiert und neu gestartet;
Deinstallation entfernt nur den Programmordner.

**Dabei aufgefallen:** Aus der Claude-App gestartete Programme schreiben
unter Windows in einen umgeleiteten AppData-Bereich (MSIX-Virtualisierung).
Die Probe-Installation lag deshalb dort; die echte Installation muss außerhalb
gestartet werden (Explorer oder Doppelklick).

## 1.2 – Englische Oberfläche ✔ 18.11.2026

Damit die App auch außerhalb des deutschsprachigen Raums genutzt (und
gefunden) wird.

- [x] Alle Texte zweisprachig: wiederkehrende Texte und Fehlermeldungen in
      `texts.rs`, einmalige als `tr("Deutsch", "English")` bzw.
      `tr_format!` an ihrem Ort (F-44 angepasst); Sprache im
      Einstellungsdialog wählbar, Standard: Sprache von Windows (F-46,
      `sys-locale`); Umschalten wirkt sofort
- [x] Gründe aus der Logik, die die Oberfläche zeigt, als feste Werte statt
      deutscher Sätze: `NameProblem`, `ArchiveLimit` (Logdatei und
      Konsolenwerkzeug bleiben deutsch)
- [x] Tastenkürzel-Namen je Sprache (Strg/Ctrl, Entf/Del, Umschalt/Shift)
- [x] Bildschirmfoto und Vorschaubild auf Englisch
      (`scripts/demo_bibliothek.py --english`, `scripts/social_preview.py`)
- [x] Release-Text auf Englisch und Deutsch

**Abnahme 18.11.2026:** Hauptfenster und Einstellungsdialog auf Englisch
(Sprache über die Einstellungsdatei gesetzt, Dialog mit Strg+, geöffnet).
Vom Nutzer bestätigt: das erste echte Update über GitHub (1.1.0 → 1.2.0 per
Hinweisleiste) und das Umschalten der Sprache per Klick im Dialog.

## 1.3 – Gründlichere automatische Suche (geplant)

- [ ] Alle Adobe-Pfade an einer Stelle (`adobe::locations`), keine
      Umgebungsvariablen verstreut im Code
- [ ] Mehrere Fundorte erkennen: Benutzerordner
      `%APPDATA%\Adobe\CameraRaw`, Ordner für alle Benutzer
      `%ProgramData%\Adobe\CameraRaw`, Lightroom-Classic-Kataloge mit
      „Vorgaben mit Katalog speichern“ (Ordner erst per Versuch bestätigen)
- [ ] Standard bleibt der Benutzerordner; der Einstellungsdialog zeigt alle
      Fundorte zur Auswahl, „Automatisch suchen“ bleibt jederzeit möglich

## Danach (1.x)

Rückgängig (F-29) · Dublettenerkennung (F-30) · Duplizieren (F-36) ·
ZIP-Export (F-37) · Mehrere umbenennen (F-38) · Einstellungen eines Presets
anzeigen (F-39) · Kameraprofile `.dcp` (F-10) · `.lrtemplate` (F-11) ·
helles Thema

**Verworfen (18.11.2026):** macOS-Fassung · Plugins für Photoshop (UXP) und
Lightroom Classic (Lua-SDK) – zwei weitere Codebasen, deutlich weniger
Bedienkomfort, Photoshop bräuchte trotzdem einen Neustart.

## Risiken

| Risiko | Auswirkung | Gegenmaßnahme |
|---|---|---|
| `crs:SortName` wird von Adobe ignoriert | Sortieren braucht anderen Weg | Geklärt (P-2): wird in Photoshop und Lightroom Classic beachtet |
| Adobe ändert Ordner oder Format | App findet nichts mehr | Pfade an einer Stelle (`adobe::locations`), manuell überschreibbar |
| Fehler beschädigt Presets | Datenverlust | Sicherung, atomares Schreiben, Papierkorb, Tests (04, 05) |
| Index-Cache zeigt alten Stand | Nutzer hält die App für defekt | Geklärt (P-3): Neustart genügt; Hinweis „Neustart nötig“ |
| Build-Ordner in OneDrive | langsame Builds, Sync-Konflikte | `CARGO_TARGET_DIR` außerhalb setzen (03) |
