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

## 1.3 – Gründlichere automatische Suche ✔ 18.11.2026

- [x] Alle Adobe-Pfade an einer Stelle (`adobe::locations`, dazu
      `adobe::lightroom` für die Einstellungsdatei von Lightroom); die
      Systemordner kommen über `SearchRoots`, damit Tests einen nachgebauten
      Rechner benutzen
- [x] Fundorte (F-47): Benutzerordner `%APPDATA%\Adobe\CameraRaw` und Ordner
      neben Lightroom-Classic-Katalogen („Presets mit diesem Katalog
      speichern“, Versuch P-6 in 02). `%ProgramData%\Adobe\CameraRaw`
      **verworfen**: dort liegen nur Adobes mitgelieferte Presets – die App
      lehnt den Ordner ab (`CoreError::AdobeOwnFolder`)
- [x] Standard bleibt der Benutzerordner; der Einstellungsdialog zeigt alle
      Fundorte zur Auswahl und warnt, wenn Lightroom Presets mit dem Katalog
      speichert; „Automatisch suchen“ bleibt
- [x] Konsolenwerkzeug: `preset_cli fundorte`

**Abnahme 18.11.2026:** `fundorte` auf dem Entwicklungsrechner findet
Benutzerordner und den Katalogordner aus P-6. Im Dialog zum Katalogordner und
zurück gewechselt (Bibliothek wechselt mit); Warnung und englische Texte an
einem nachgebauten Rechner (`APPDATA` auf einen Testordner) geprüft. Vom
Nutzer bestätigt: Update 1.2.0 → 1.3.0 über GitHub.

## Nächste Versionen (geplant 18.11.2026, Reihenfolge nach Wunsch des Nutzers)

Der Hinweis „Gruppen stehen in den Dateien selbst (crs:Group) …“ unter der
Gruppenliste ist entfallen – Technik, die der Nutzer nicht braucht.

### 1.4 – Was macht dieses Preset? (F-39, Teil 1: Regler) ✔ 18.11.2026

Bestand am Entwicklungsrechner (410 Presets und Profile): Gradationskurven in
fast allen, Grundeinstellungen (Lichter, Tiefen, Weiß, Schwarz, Kontrast,
Klarheit) in rund der Hälfte, Color Grading und Teiltonung in rund 190,
Masken in rund 190, Schwarzweiß in 29 Dateien.

- [x] `preset_core::preset::settings`: alle Camera-Raw-Werte einer Datei,
      Gradationskurven, eingebettetes Profil (`crs:Look`) und lokale
      Anpassungen (Masken mit Art und geänderten Reglern)
- [x] `preset_core::preset::adjustments`: Bereiche wie in Camera Raw (nur
      was vom Standard abweicht), Kurven als Spline, Farbmischer,
      Schwarzweiß-Mischer, Color Grading, Weißabgleich und sprachneutrale
      Aussagen (`Fact`) für den Klartext
- [x] Details rechts: Seiten „Wirkung“ und „Verwalten“; Regler mit
      Mittelpunkt (Temperatur und Tönung mit Farbverlauf), Kurve gezeichnet,
      Color-Grading-Räder, Farbmischer als farbige Balken, Masken als Liste
- [x] Ein Satz in Klartext, z. B. „Heller, matte, angehobene Schwarztöne,
      wärmer, Grün entsättigt, Filmkorn.“ – zweisprachig
- [x] Profile mit Farbtabelle: Hinweis, dass sich der Look nicht als Regler
      zeigen lässt
- [x] Demo-Bibliothek mit ausgedachten, aber vollständigen Looks

**Abnahme 18.11.2026:** Alle 410 Dateien des Entwicklungsrechners werden
gelesen (Test `real_data`, liest nur). Im Fenster an der Demo-Bibliothek
geprüft: Grundeinstellungen, Kurve, Color-Grading-Räder, Farbmischer,
Schwarzweiß-Mischer, Effekte, Masken, Profile. Vom Nutzer bestätigt: Update
1.3.0 → 1.4.0 über GitHub.

### 1.5 – Aktiv und inaktiv (F-48) ✔ 18.11.2026

Alle Presets und Profile im Manager behalten, aber selbst bestimmen, welche
Photoshop und Lightroom zeigen.

- [x] Versuch P-7 (siehe 02): Favoriten überstehen Herausnehmen, Neustart und
      Zurücklegen; `crs:ShowInPresets="False"` blendet in Camera Raw aus, ist
      für Lightroom Classic aber ungeprüft – deshalb Verschieben
- [x] **Deaktiviert** heißt: Die Datei liegt im selben Camera-Raw-Ordner unter
      `Preset Manager (inactive)\Settings\…` bzw. `…\ImportedSettings\…`
      mit demselben Unterpfad (dazu eine Erklärung als Textdatei). Aktivieren
      legt sie genau dorthin zurück; der Inhalt bleibt unverändert.
      Umbenennen auf demselben Laufwerk, sonst Kopieren mit Prüfung
- [x] Belegtes Ziel: Der Eintrag bleibt, der Bericht nennt die Datei
- [x] Umschalten: Auge am Zeilenende, Rechtsklick, Strg+D, Schalter in den
      Details (Eintrag, Auswahl, Gruppe), ganze Gruppen im Kontextmenü,
      Ziehen auf eine Gruppe des anderen Zustands (verschiebt und schaltet um)
- [x] Deutlich getrennt: links **Presets · Profile**, darunter mit Trennlinie
      **DEAKTIVIERT** mit eigenen Bereichen; eine Gruppe mit beiden Zuständen
      steht in beiden Teilen mit ihrer Anzahl. Deaktivierte Zeilen blasser mit
      durchgestrichenem Auge, Ansicht mit Etikett „DEAKTIVIERT“, Details mit
      orangefarbenem Hinweis
- [x] Gruppe löschen wirkt nur auf den angezeigten Zustand; Umbenennen einer
      Gruppe auf beide

**Abnahme 18.11.2026** an der Demo-Bibliothek: einzeln über das Auge, ganze
Gruppe, Strg+D, Aktivieren über die Details, Ziehen eines deaktivierten
Eintrags auf eine aktive Gruppe – jeweils Dateien am richtigen Ort, leere
Paketordner entfernt, Ansicht wechselt mit. Kerntests für Hin und Zurück,
belegtes Ziel, Doppeltes, Verwalten deaktivierter Einträge. Vom Nutzer
bestätigt: Update 1.4.0 → 1.5.0 über GitHub, Aktivieren/Deaktivieren in der
installierten App.

### 1.6 – Vorher/Nachher (F-39, Teil 2) ✔ 18.11.2026 (nach Testversion `1.6.0-beta.1`)

Auf Wunsch des Nutzers als **Testversion** im Zweig `test/vorher-nachher`;
`main` bleibt bei 1.5. Taugt die Vorschau nichts, wird der Zweig gelöscht und
von 1.5 aus weiterentwickelt. Die Testversion bekommt nur, wer in den
Einstellungen „Testversionen“ einschaltet.

- [x] Machbarkeit Farbtabellen: `crs:RGBTable` (3D-LUT, 32³ Punkte,
      ProPhoto mit Gamma 1,8) und `crs:LookTable` (Farbton × Sättigung ×
      Helligkeit, wie die HueSatMap der DNG-Spezifikation) lassen sich
      vollständig lesen – Base85 mit Adobes Alphabet, zlib, dann die Werte.
      Alle 148 Tabellen am Entwicklungsrechner gelesen
      (`preset_core::preset::color_table`)
- [x] `preset_core::preview`: Foto in sRGB nachrechnen – Farbtabelle(n) des
      Profils (auch über die Kennung, wenn das Preset nur verweist),
      Weißabgleich, Belichtung, Licht, Kurven, Farbmischer, Schwarzweiß,
      Color Grading, Vignette, Körnung; `missing` nennt, was fehlt
- [x] Details „Wirkung“: Vorher/Nachher mit Schieber, Foto wählen oder
      Testtafel, große Ansicht; gerechnet im Hintergrund
- [x] Konsolenwerkzeug `preset_cli vorschau` (Vorher/Nachher als PNG)
- [x] **Bewertung durch den Nutzer:** übernehmen („Keep it“). Zweig in `main`
      zusammengeführt, als 1.6.0 veröffentlicht
- [x] Dazu auf Wunsch des Nutzers: Details aufgeräumt (Name als Titel,
      Schalter statt Kasten, Satz vor der Vorschau, Regler einzeilig,
      Werkzeuge als Symbole, Hinweis als Tooltip), Einstellungsdialog passt
      sich kleinen Fenstern an, Bildlaufleisten überall sichtbar (feste
      Leisten, hellerer Griff, ausgeblendeter Rand)

### 1.6.1 ✔ 18.11.2026

- [x] Wirkung: „Alle aufklappen“ / „Alle zuklappen“ über den Bereichen
      (Wunsch des Nutzers, Bedienbarkeit)
- [x] Hinweis „Testversion“ in der Vorschau entfernt

### 1.7 – ZIP-Export (F-37) ✔ 18.11.2026

- [x] Auswahl oder Gruppen als ZIP speichern, das sich mit Preset Manager
      oder Camera Raw wieder installieren lässt (`preset_core::export`: je
      Gruppe ein Ordner, Dateien unverändert, gleiche Namen nummeriert, ZIP
      erst im Speicher, dann in einem Zug geschrieben)
- [x] Rechtsklick auf Eintrag und Gruppe, Schaltflächen in den Details,
      Strg+E (Auswahl, sonst geöffnete Gruppe)
- [x] Geprüft: Export, dann Import in eine leere Bibliothek – Gruppe und
      Dateien (byte-gleich) kommen wieder an

### Reihenfolge ab 1.8 (festgelegt am 19.11.2026)

Erst Rückgängig, dann ein Aufräum- und Sicherheitsdurchgang, bevor die großen
neuen Teile kommen. Danach in der Reihenfolge, in der eins auf dem anderen
aufbaut: Suche und Profilprüfung nutzen die vorhandene Auswertung, die
Galerie die Vorschau, das PDF die Galerie, die LUT-Umwandlung den
Farbtabellen-Dekoder. Nikon zuletzt, weil das Format erst erforscht werden
muss. Am 19.11.2026 eingeschoben: der Microsoft Store als 1.10, vor den
neuen Funktionen.

### 1.8 – Rückgängig (F-29) ✔ 19.11.2026

- [x] Die letzte Aktion zurücknehmen, auch mehrere nacheinander (Strg+Z),
      und wiederholen (Strg+Y, Strg+Umschalt+Z): Umbenennen, Verschieben,
      Reihenfolge, Gruppen, Aktivieren, Papierkorb, Import
- [x] Jede Aktion führt ein Protokoll (`safe_fs::Journal`: Inhalt vorher
      und nachher, angelegt, entfernt, verschoben) in ihrer
      `BackupSession`; `history::revert` wendet die Umkehrung an.
      Wiederholen ist die Rücknahme der Rücknahme. Gelöschtes kommt aus
      dem Windows-Papierkorb zurück (`Trash::restore`)
- [x] Geändert wird nur, was noch genau so ist, wie die Aktion es
      hinterließ; sonst `ChangedSinceAction` mit Dateiname
- [x] Bis zu 20 Schritte bzw. 256 MB, nur für die laufende Sitzung
- [x] Oberfläche: Pfeile oben rechts mit Tooltip („Rückgängig: „Matt“
      umbenennen“), „Rückgängig“ in der Einblendung (10 s, bleibt beim
      Darüberfahren), Zurückgeholtes wird ausgewählt
- [x] Geprüft: 9 neue Tests; im Fenster Umbenennen, Löschen (aus dem
      echten Papierkorb zurück, byte-gleich), Gruppe deaktivieren,
      Wiederholen

### 1.9 – Aufräumen, Fehlersuche, Sicherheit (N-01, N-08, N-09) ✔ 19.11.2026

Keine neuen Funktionen; die Grundlage für alles Weitere.

- [x] **Aufteilen:** 13 Dateien über 500 Zeilen nach Themen in Module
      zerlegt (App: `actions`, `app`, `worker`, `ui/preset_view`,
      `ui/list`, `ui/settings_dialog`, `ui/details`; Kern: `library/ops`,
      `preset/adjustments`, `preset/xmp_edit`, `preset/color_table`,
      `preview`, `import/plan`). Größte Datei jetzt `texts.rs` (484)
- [x] **Lesbarkeit:** jede `mod.rs` nennt ihre Teile; Sichtbarkeit nur so
      weit wie nötig, Importe explizit; doppelte Hilfsfunktionen
      zusammengeführt (`file_name`, `same_name`); Planung/03 erneuert
- [x] **Fehlersuche:** Code-Review aller Module, die Dateien ändern
      (8 Befunde, 7 behoben, 1 bewusst gelassen: Gleichstand bei der
      Löschzeit im Papierkorb – ohne Datenverlust); neue Tests
- [x] **Sicherheit:**
      - Lesen mit Obergrenze überall (`safe_fs::read_preset_file`, 50 MB)
      - Farbtabellen prüfen die Datenmenge vor dem Reservieren
      - XML: nur Zeichenreferenzen und die fünf Standard-Entitäten (kein
        XXE, keine Entitäten-Bomben) – geprüft, war schon so
      - ZIP: Anzahl, Tiefe, Größe, Pfade, Symlinks – geprüft, war schon so
      - Explorer mit vollem Pfad
      - `cargo audit` in der CI; zip 8.6 über Velopack begründet
        ausgenommen (`.cargo/audit.toml`), bis Velopack 1.3 stabil ist
      - Update-Weg: nur HTTPS von GitHub; `APM_UPDATE_DIR` nur zum Testen

### 1.10 – Microsoft Store (F-56)

Vor den neuen Funktionen, festgelegt am 19.11.2026. Ziel: die App kostenlos
und ohne SmartScreen-Warnung anbieten, ohne ein Zertifikat zu kaufen.

**Beide Wege parallel:** GitHub bleibt wie gewohnt (Setup, Selbst-Update),
der Store kommt dazu. Ein Release baut beides aus demselben Code mit
derselben Versionsnummer; im Store wird jede Version zusätzlich eingereicht
und von Microsoft geprüft (meist 1–3 Tage), erscheint dort also etwas
später. Beide Fassungen bearbeiten dieselben Adobe-Ordner – auf einem
Rechner sollte nur eine installiert sein.

Stand der Recherche (19.11.2026):
- Die Anmeldung als Einzelentwickler ist seit 10.09.2025 kostenlos (früher
  19 $); nötig ist ein Identitätsnachweis mit Ausweis.
- Ein **MSIX-Paket** signiert Microsoft nach der Prüfung selbst. Ein
  EXE/MSI-Installationsprogramm im Store müssten wir dagegen selbst mit
  einem gekauften Zertifikat signieren – also MSIX.
- **Haken AppData:** Eine MSIX-App legt neue Dateien unter `AppData` in
  einem privaten Ordner ab, nicht im echten. Photoshop und Lightroom sähen
  importierte Presets dann nicht. Abhilfe: die eingeschränkte Berechtigung
  `unvirtualizedResources`, ab Windows 11 auf einzelne Ordner begrenzbar
  (`virtualization:ExcludedDirectories`). Microsoft muss sie bei der
  Prüfung freigeben; Begründung: Die App verwaltet genau diese Ordner.
- **Haken Updates:** Eine Store-App aktualisiert sich nur über den Store.

- [x] **Store-Build:** Cargo-Merkmal `self-update` (Standard) schaltet
      Velopack ein; die Store-Fassung baut mit `--no-default-features`,
      sucht nie selbst und nennt in den Einstellungen „neue Versionen
      installiert der Microsoft Store“. Die CI prüft beide Fassungen
- [x] **MSIX-Paket:** `packaging/msix/AppxManifest.xml` (volle
      Vertrauensstufe; Windows 11: Ausnahme nur für `Adobe\CameraRaw` und
      die eigenen Ordner, Windows 10: AppData ohne Umleitung), Symbole aus
      `make_icon.py`, `scripts/New-StorePackage.ps1` (C-Laufzeit fest
      eingebaut, keine VC++-Abhängigkeit). Der Release-Workflow baut das
      Paket als Artefakt `store-package`, nicht für Testversionen
- [x] **Versuch P-10** (19.11.2026, Windows 11, Testpaket unsigniert im
      Entwicklermodus eingerichtet): Dateien aus dem Paket landen in
      `Adobe\CameraRaw`, `Roaming\PresetManager` und
      `Local\PresetManager` im echten Ordner, anderes unter AppData wird
      umgeleitet – das Manifest wirkt. Log im echten Ordner; Papierkorb und
      Rückgängig klappen. Offen für den Nutzer: ein Import ins echte
      Camera-Raw erscheint in Photoshop und Lightroom
      (Vorsicht beim Nachprüfen: Die Claude-App ist selbst ein MSIX-Paket;
      Befehle aus ihr sehen AppData durch ihre eigene Umleitung)
- [x] **Store-Eintrag:** `packaging/msix/Store-Eintrag.md` – Texte Deutsch
      und Englisch, Bildschirmfotos aus `docs/`, `PRIVACY.md`,
      Altersfreigabe, Hinweise für die Prüfer mit Begründung für
      `unvirtualizedResources`; Name ohne „Adobe“
- [ ] **Nutzer:** Konto auf storedeveloper.microsoft.com anlegen, Namen
      reservieren, einreichen
- [ ] Lehnt Microsoft die Berechtigung ab, bleibt es bei GitHub; F-56 wird
      dann gestrichen

### 1.11 – Suche nach Wirkung, fehlende Profile (F-49, F-50)

- [ ] Die Suche findet auch Wirkungen („warm“, „Schwarzweiß“, „Filmkorn“,
      „Maske“), auf Deutsch und Englisch
- [ ] Filter „nutzt Profil …“
- [ ] Presets mit fehlendem Profil: Hinweis in Liste und Details, Name des
      gesuchten Profils; eigener Filter „Profil fehlt“

### 1.12 – Galerie (F-51)

- [ ] Umschalter **Liste | Galerie** in der Kopfzeile der Mitte, neben den
      Filtern – keine neue Leiste, kein neuer Bereich
- [ ] Vorschaubilder auf dem Vorschaufoto (bzw. der Testtafel), im
      Hintergrund berechnet und zwischengespeichert; Größe per Regler
- [ ] Auswählen, Ziehen, Rechtsklick wie in der Liste

### 1.13 – Kontaktabzug als PDF (F-52)

- [ ] Eigener Dialog „Kontaktabzug …“ (Rechtsklick auf Auswahl oder
      Gruppe, Schaltfläche in der Galerie) – die Hauptansicht bekommt nur
      diesen einen Einstieg
- [ ] Inhalt: Auswahl, eine oder mehrere Gruppen (Häkchenliste); Foto;
      Seitenformat A4/A3, hoch/quer; Bilder je Zeile; mit Name,
      Kurzbeschreibung, Gruppenüberschriften
- [ ] Seitenvorschau im Dialog, dann speichern; PDF ohne Fremdprogramm
      (reine Rust-Bibliothek)

### 1.14 – Als LUT exportieren (F-53)

- [ ] Profile mit Farbtabelle: genaue `.cube`-Datei (17/33/65 Stützstellen)
- [ ] Presets: Annäherung aus den globalen Farb- und Tonwerten, klar als
      solche gekennzeichnet; Masken und Effekte werden genannt, nicht
      übernommen
- [ ] Farbraum der LUT wählbar (sRGB / Rec. 709)

### 1.15 – LUT in Profil umwandeln (F-54)

- [ ] `.cube` einlesen und als Camera-Raw-Profil (Farbtabelle wie in 1.6
      dekodiert, nur umgekehrt) mit Name, Gruppe und Stärke-Bereich
      installieren. Größere LUTs auf 32×32×32 umrechnen – mehr liest
      Lightroom laut den bekannten Konvertern nicht (in P-9 nachprüfen)
- [ ] Abgrenzung: Es gibt mehrere kostenlose Browser-Konverter (einzelne
      Datei rein, Datei raus). Unser Vorteil: offline, viele auf einmal,
      direkt installiert, mit Vorher/Nachher und Gruppe
- [ ] Danach `.3dl` und Hald-CLUT-Bilder (`.png`)
- [ ] Prüfung in Photoshop: das erzeugte Profil sieht aus wie die LUT

### 1.16 – Nikon Picture Control `.NP3` (F-55), als Testversion

Stand der Recherche (19.11.2026): Nikon dokumentiert `.NP3` nicht, der
Aufbau der „Flexible Color“-Picture-Controls ist aber offengelegt –
`ssssota/nikon-flexible-color-picture-control` (TypeScript, MIT) liest und
schreibt Schärfe, Mittenschärfe, Klarheit, Tonwertkurve, Kontrast, Lichter,
Tiefen, Weiß-/Schwarzwert, Sättigung, Color Blender (Farbton, Chroma,
Helligkeit je Farbe) und Color Grading. Eine 3D-LUT kennt NP3 nicht. Kameras
laut `krafta-visio/nikon-flexible-color-creator`: Z6III, Z5II, Z50II, Z8, Z9,
Zf; dazu NX Studio. Es gibt bereits Browser-Konverter XMP → NP3 (z. B.
recipe.shuttercoach.app, „48 Parameter“).

- [ ] **Versuch P-8:** Lese-/Schreiblogik nach Rust übertragen (MIT, Quelle
      nennen); Beispieldateien aus NX Studio Byte für Byte nachbauen;
      erzeugte Datei in NX Studio öffnen und – wenn eine passende Kamera da
      ist – an der Kamera laden. Bekanntes Risiko: manche Dateien aus dem
      Netz lädt die Kamera nicht; Abhilfe in anderen Werkzeugen ist eine von
      der Kamera exportierte Datei als Vorlage
- [ ] Umwandlung Preset → NP3 als Annäherung: Grundregler, Gradationskurve,
      Farbmischer → Color Blender, Color Grading; was fehlt (Masken, Profil-
      Farbtabellen, Körnung …), nennt die App
- [ ] Ganze Gruppen auf einmal, Ausgabe wahlweise direkt auf die
      Speicherkarte (`NIKON\CUSTOMPC`)
- [ ] Später möglich: NP3 → Preset (der umgekehrte Weg)
- [ ] Zeigt P-8, dass es nicht verlässlich geht, wird F-55 gestrichen

### Danach

- Doppelte finden und Duplizieren (F-30, F-36)
- Kameraprofile `.dcp` und alte `.lrtemplate` (F-10, F-11)
- Mehrere umbenennen (F-38) · helles Thema
- **macOS-Fassung** (wieder aufgenommen am 19.11.2026, vorher verworfen):
  egui, `trash` und Velopack laufen auch auf macOS; die Adobe-Pfade sind
  in `adobe::locations` gekapselt (N-07). Verteilt wird über GitHub wie
  unter Windows – GitHub ist der Weg für alle Systeme, der Microsoft Store
  ein zusätzlicher Weg nur für Windows. Ohne Warnung von macOS braucht es
  Signatur und Beglaubigung durch Apple (Apple Developer Program, 99 $ im
  Jahr); unsigniert muss der Nutzer die App einmal in den
  Systemeinstellungen freigeben. Den Mac App Store erst später prüfen:
  Dort ist die Sandbox Pflicht, und der Zugriff auf
  `~/Library/Application Support/Adobe` ist nur mit Freigabe durch den
  Nutzer oder einer Ausnahme möglich, die Apple selten genehmigt. Zum
  Testen ist ein Mac nötig

**Verworfen (18.11.2026):** Plugins für Photoshop (UXP) und
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
