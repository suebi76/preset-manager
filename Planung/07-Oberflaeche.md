# 07 – Oberfläche: Gestaltung und Bedienung

Ziel: Wer Lightroom oder Photoshop kennt, soll sich sofort zurechtfinden. Die
App sieht aus wie ein Werkzeug aus derselben Familie – dunkel, ruhig, mit
klaren Bedienfeldern – und braucht keine Anleitung.

„Im Stil von Adobe“ heißt dabei: gleiche **Gestaltungsprinzipien** (dunkle
Bedienfelder, ein blauer Akzent, kompakte Typografie, aufklappbare Panels).
Logos, Symbole und die Schrift „Adobe Clean“ werden **nicht** übernommen – sie
sind geschützt.

## Leitlinien

1. **Eine Hauptaktion, gut sichtbar.** „Presets importieren“ ist die einzige
   farbig hervorgehobene Schaltfläche. Alles andere ist zurückhaltend.
2. **Ziehen statt suchen.** Ordner/ZIP aufs Fenster ziehen, Presets in eine
   andere Gruppe ziehen, Reihenfolge durch Ziehen ändern.
3. **Erst zeigen, dann tun.** Jede Änderung zeigt vorher, was passiert
   (Importvorschau, Löschbestätigung mit Anzahl und Namen).
4. **Rückmeldung ohne Unterbrechung.** Ergebnisse erscheinen als kurze
   Einblendung unten rechts („18 Presets installiert“), nicht als Dialog,
   den man wegklicken muss. Nur Fehler bleiben stehen, bis man sie liest.
5. **Nichts geht verloren.** Löschen geht in den Papierkorb; die Einblendung
   sagt das auch.
6. **Tastatur gleichwertig.** Jede Aktion hat ein Tastenkürzel, die gängigen
   wie in Lightroom.

## Aufbau des Fensters

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  ▣ Preset Manager      [ 🔍 Suchen …          ]        [ ⬇ Presets importieren ] │  Kopfleiste
├────────────────────┬─────────────────────────────────────────┬───────────────┤
│ PRESETS        ＋  │  [Alle | Presets | Profile]              │ DETAILS       │
│  ⚙ Sommerlicht (11)│  Sommerlicht · 11 Presets                │ ⚙ Preset      │
│  ⚙ Leica Q2    (8) │  ─────────────────────────────────────── │ Name          │
│  ⚙ Reisetag. ⤓ (18)│  ⠿ ⚙ Sommerlicht – Black & White Classic │ [Sommerl. – …]│
│  ┆ Neue Gruppe ┆   │  ⠿ ⚙ Sommerlicht – Black & White Soft    │ Gruppe        │
│ PROFILE        ＋  │  ⠿ ⚙ Sommerlicht – Bondi        ◀ ausgew.│ [Sommerlicht▾]│
│  ◎ Filmprof. … (14)│  ⠿ ⚙ Sommerlicht – Brighton              │ Datei: …      │
│  ◎ Filmkl. …   (55)│  …                                       │ [Im Explorer] │
├────────────────────┴─────────────────────────────────────────┴───────────────┤
│ ⚠ Photoshop läuft – Änderungen erscheinen nach einem Neustart.          [×]   │  Hinweisleiste
└──────────────────────────────────────────────────────────────────────────────┘

⚙ = Preset (blau) · ◎ = Profil (violett) · ⤓ = über Adobe installiert · ┆…┆ = leere, neue Gruppe
```

- **Links – Gruppen** (wie das Vorgaben-Bedienfeld in Lightroom): aufklappbar,
  mit Anzahl. Zwei Bereiche wie in Camera Raw: **Presets** und **Profile**
  (F-32), jeweils mit eigenem „＋ Neue Gruppe“. Ein kleines Symbol hinter der
  Gruppe zeigt, ob sie über Adobes Importdialog kam. Rechtsklick: Umbenennen,
  Löschen, Als ZIP exportieren.
- **Filter über der Mitte:** `Alle · Presets · Profile` als Segmentschalter.
- **Mitte – Presets der Gruppe**: Liste mit Griff `⠿` zum Umsortieren.
  Mehrfachauswahl mit Strg/Umschalt. Doppelklick oder F2 benennt um, direkt
  in der Zeile.
- **Rechts – Details**: Name und Gruppe direkt bearbeitbar, darunter Art,
  Dateipfad, UUID und „Im Explorer zeigen“. Bei Mehrfachauswahl: „3 Presets
  ausgewählt“ mit den Aktionen Verschieben und Löschen.
- **Unten – Hinweisleiste**: nur sichtbar, wenn es etwas zu sagen gibt
  (Photoshop/Lightroom läuft, Neustart nötig).

## Arten auf einen Blick (F-31)

Jede Zeile beginnt mit einem Symbol in der Farbe ihrer Art. Dieselben
Symbole stehen in der Gruppenliste, im Filter, in der Importvorschau und in
den Details – man lernt sie einmal.

| Art | Symbol (Phosphor) | Farbe | Wo in Camera Raw |
|---|---|---|---|
| Preset | `sliders-horizontal` (Regler) | Blau `#2680EB` | Vorgaben |
| Profil (kreatives Profil, „Look“) | `aperture` (Blende) | Violett `#9256D9` | Bearbeiten → Profil → Durchsuchen |
| Kameraprofil (`.dcp`, ab 1.x) | `camera` | Türkis `#12A5A5` | Profilbrowser, Kamera-Abgleich |
| Altes Lightroom-Preset (`.lrtemplate`, später) | `clock-counter-clockwise` | Grau `#9A9A9A` | nur Lightroom Classic |
| Unbekannte Art | `question` | Grau | – |
| Defekt / nicht lesbar | `warning` | Orange `#E68619` | wird von Adobe übergangen |

Zusätzlich, klein und gedämpft hinter dem Namen:

- `download-simple` – über Adobes Importdialog installiert (`ImportedSettings`)
- `copy` – gleiches Preset (UUID) liegt mehrfach vor (F-30)

Farbe ist nie das einzige Merkmal: Symbol und – beim Überfahren – ein Tooltip
(„Profil · erscheint im Profilbrowser“) tragen dieselbe Information.

## Gruppen anlegen (F-33)

- **„＋“ neben „Presets“ bzw. „Profile“**: Es erscheint eine neue Zeile mit
  Eingabefeld „Neue Gruppe“. Enter bestätigt.
- **„Neue Gruppe aus Auswahl“** (Rechtsklick oder Strg+G): legt die Gruppe
  an und verschiebt die ausgewählten Einträge sofort hinein.
- **Leere Gruppen** sind gestrichelt umrandet, mit dem Hinweis „Zieh Presets
  hierher“. Camera Raw kennt keine leeren Gruppen; sie werden sichtbar,
  sobald etwas darin liegt (siehe 01).

## Ziehen & Ablegen (F-12, F-34, F-35)

| Was wird gezogen | Wohin | Ergebnis |
|---|---|---|
| Ein oder mehrere Einträge | andere Gruppe links | in diese Gruppe verschieben |
| Einträge | „＋ Neue Gruppe“ | neue Gruppe mit diesen Einträgen |
| Einträge | innerhalb der Liste | Reihenfolge ändern |
| Gruppe | andere Gruppe | zusammenführen (mit Rückfrage) |
| Ordner/ZIP aus dem Explorer | Fenster | Importvorschau, Gruppen aus den Dateien |
| Ordner/ZIP aus dem Explorer | bestimmte Gruppe | Importvorschau, alles in diese Gruppe |

Rückmeldung während des Ziehens:

- Am Mauszeiger hängt ein Stapel mit Anzahl („3 Presets“) und Typ-Symbol.
- Ein gültiges Ziel leuchtet mit blauem Rahmen auf; zwischen Listenzeilen
  zeigt eine blaue Linie die Einfügestelle.
- Ein ungültiges Ziel (Profil auf eine Preset-Gruppe und umgekehrt, Gruppe
  mit gleichnamigem Eintrag …) bleibt grau, der Zeiger zeigt ⦸ und ein
  Tooltip erklärt kurz, warum.
- Nach dem Ablegen: Einblendung „3 Presets nach ‚Sommer‘ verschoben ·
  Rückgängig“ (F-29, seit 1.8).

## Weitere Aktionen

Über Rechtsklick, Detailspalte und Tastatur erreichbar:

- Umbenennen (F2), auch mehrere auf einmal (F-38)
- Duplizieren (F-36)
- Als ZIP exportieren (Strg+E, F-37)
- Im Explorer zeigen (F-45)
- Löschen (Entf) – mit Anzahl und Namen in der Bestätigung, „in den
  Papierkorb“ steht ausdrücklich dabei

### Leerer Zustand

Ohne Presets zeigt die Mitte eine große
Ablagefläche: gestricheltes Feld, Symbol, „Ordner oder ZIP hierher ziehen –
oder klicken, um auszuwählen“.

### Beim Ziehen

Zieht man Dateien über das Fenster, legt sich eine halbtransparente Fläche
mit blauem Rahmen darüber: „Loslassen zum Importieren“.

## Importvorschau

Ein Dialog in der Mitte des Fensters:

```
┌ Presets importieren ────────────────────────────────────────────┐
│ Sommerlicht Presets.zip → Gruppe aus den Dateien                │
│                                                                 │
│  ☑ Sommerlicht – Bondi                    neu                   │
│  ☑ Sommerlicht – Byron                    neu                   │
│  ☐ Sommerlicht – Miami                    schon installiert ⓘ   │
│  ☑ Sommerlicht – Sorrento                 Datei vorhanden ⓘ     │
│                                                                 │
│  Wenn eine Datei schon vorhanden ist:  (•) überspringen         │
│                                         ( ) ersetzen            │
│                                         ( ) beide behalten      │
│                                                                 │
│  3 von 4 werden installiert          [Abbrechen] [Installieren] │
└─────────────────────────────────────────────────────────────────┘
```

Schon anderswo installierte Presets sind abgewählt (Entscheidung vom
17.11.2026, siehe 06). Die Schaltfläche nennt immer die tatsächliche Anzahl.

## Einstellungen (M6)

Zahnrad rechts oben oder Strg+, öffnet einen Dialog mit vier Abschnitten:

- **Camera-Raw-Ordner:** Pfad, Herkunft (automatisch / selbst gewählt /
  Umgebungsvariable), „Camera-Raw-Ordner wählen …“, „Automatisch suchen“,
  „Öffnen“. Während eine Aktion läuft, ist der Wechsel gesperrt.
- **Darstellung:** Größe der Oberfläche in Stufen (80–200 %) mit − / + und
  „Zurücksetzen“ – bewusst kein Schieberegler, der würde beim Ziehen mit der
  Oberfläche mitwachsen.
- **Sicherungen und Protokoll:** kurze Erklärung, je „Ordner öffnen“.
- **Über:** Version, Lizenz, Markenhinweis.

Wird der Camera-Raw-Ordner nicht gefunden, zeigt das Fenster statt der
Bibliothek ein Fehlerbild mit „Camera-Raw-Ordner wählen …“ und – wenn ein
gewählter Ordner fehlt – „Wieder automatisch suchen“.

## Gestaltung

### Farben (dunkles Thema, Standard)

| Rolle | Wert | Verwendung |
|---|---|---|
| Hintergrund | `#1D1D1D` | Fenster, Mitte |
| Bedienfeld | `#252525` | linke und rechte Spalte, Kopfleiste |
| Fläche erhöht | `#323232` | Eingabefelder, Listenzeile beim Überfahren |
| Trennlinie | `#3A3A3A` | 1 px zwischen Bereichen |
| Text | `#E8E8E8` | Namen, Beschriftungen |
| Text gedämpft | `#9A9A9A` | Anzahl, Dateipfade, Überschriften der Panels |
| Akzent | `#2680EB` | Hauptschaltfläche, Auswahl, Fokusrahmen |
| Auswahl | `#2680EB` mit 25 % Deckkraft | ausgewählte Zeilen |
| Warnung | `#E68619` | Hinweisleiste |
| Fehler | `#E34850` | Fehlermeldungen |
| Erfolg | `#2D9D78` | Einblendung „installiert“ |

Ein helles Thema kommt später (1.x); die Farben stehen deshalb an einer
Stelle (`ui/theme.rs`) und werden nie direkt im Code verteilt.

### Schrift, Abstände, Formen

- Schrift: **Segoe UI Variable** (Windows-Systemschrift), ersatzweise Inter.
  Größen: 12 px Standard, 11 px gedämpfte Angaben, 11 px Großbuchstaben mit
  Abstand für Panel-Überschriften („PRESETS“, „PROFILE“), 15 px Dialogtitel.
- Raster: 4 px. Zeilenhöhe in Listen 28 px, Innenabstand der Panels 12 px.
- Ecken: 4 px bei Schaltflächen und Feldern, 8 px bei Dialogen.
- Symbole: einheitlicher, schlichter Liniensatz (Phosphor Icons, frei
  lizenziert), 16 px.
- Bewegung: kurze Übergänge (120 ms) beim Aufklappen und bei Einblendungen,
  nichts, was wartet lässt.

## Tastenkürzel

| Aktion | Kürzel |
|---|---|
| Importieren | Strg+Umschalt+I (wie Lightroom) |
| Suchen | Strg+F |
| Einstellungen | Strg+, |
| Größe der Oberfläche | Strg+Plus / Strg+Minus / Strg+0 |
| Umbenennen | F2 |
| Löschen | Entf |
| Alles auswählen | Strg+A |
| In andere Gruppe verschieben | Strg+M |
| Neue Gruppe aus Auswahl | Strg+G |
| Aktivieren / Deaktivieren | Strg+D |
| Als ZIP exportieren | Strg+E |
| Filter Alle / Presets / Profile | Strg+1 / Strg+2 / Strg+3 |
| Nach oben / unten verschieben | Alt+↑ / Alt+↓ |
| Rückgängig (F-29) | Strg+Z |
| Wiederholen | Strg+Y oder Strg+Umschalt+Z |

## Umsetzung mit egui

egui (Entscheidung E-1) bringt von Haus aus ein schlichtes, technisches
Aussehen mit. Für den beschriebenen Stil wird es vollständig angepasst:

- eigenes `Visuals`/`Style` aus `ui/theme.rs` (Farben, Abstände, Ecken)
- eigene Schrift über `FontDefinitions`
- Symbole über das Crate `egui-phosphor`
- Einblendungen, Ablagefläche und Listenzeilen als kleine eigene Bausteine
  in `ui/widgets/`

Vor Meilenstein 4 entsteht ein klickbarer Entwurf (HTML) zur Abstimmung des
Aussehens. Erst wenn er passt, wird er in egui umgesetzt.

**Entwurf (17.11.2026):** [Entwurf/Oberflaeche-Entwurf.html](Entwurf/Oberflaeche-Entwurf.html)
– im Browser öffnen, keine Installation nötig. Enthält Beispieldaten,
speichert nichts. Zeigt Bereiche, Typ-Symbole, Gruppen anlegen, Ziehen &
Ablegen (mit Ablehnung ungültiger Ziele), Mehrfachauswahl, Umbenennen in der
Zeile, Löschen mit Bestätigung, Kontextmenüs, Tastenkürzel und die
Importvorschau.

## Offene Punkte

- ~~**Name der App**~~ – entschieden (M6): „Preset Manager“, mit
  Markenhinweis im README und im Dialog „Über“.
