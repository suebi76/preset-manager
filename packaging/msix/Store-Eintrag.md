# Store-Eintrag im Microsoft Store

Alles, was beim Einreichen im [Partner Center](https://partner.microsoft.com/dashboard)
gebraucht wird – zum Abschreiben. Texte für den Store auf Englisch und Deutsch,
Anmerkungen für die Prüfer auf Englisch.

## Einmalig: Konto und Name

1. Konto auf [storedeveloper.microsoft.com](https://storedeveloper.microsoft.com)
   anlegen: **Einzelperson**, kostenlos, mit Ausweis bestätigen.
2. Partner Center → **Apps und Spiele** → **Neues Produkt** → **MSIX- oder
   PWA-App** → Namen reservieren: **Preset Manager**. Ist er vergeben:
   „Preset Manager for Photographers“ oder „Preset Manager – Presets &
   Profiles“. Kein „Adobe“, „Lightroom“, „Photoshop“ oder „Camera Raw“ im
   Namen (Marken von Adobe).
3. Produkt → **Produktidentität**: die drei Werte als Repository-Variablen
   auf GitHub eintragen (Repository → Settings → Secrets and variables →
   Actions → **Variables**):

   | Partner Center | Variable |
   |---|---|
   | Package/Identity/Name | `STORE_IDENTITY_NAME` |
   | Package/Identity/Publisher | `STORE_PUBLISHER` |
   | Package/Properties/PublisherDisplayName | `STORE_PUBLISHER_DISPLAY_NAME` |

   Danach baut jedes Release das Paket mit diesen Werten.

## Je Version: Paket hochladen

1. Release wie gewohnt (`scripts/New-Release.ps1 -Publish`).
2. Im Lauf des Release-Workflows auf GitHub (Actions → Release) das Artefakt
   **store-package** herunterladen und entpacken: `PresetManager-<Version>-Store.msix`.
3. Partner Center → Produkt → **Einreichung** (beim ersten Mal „Einreichung
   starten“, danach „Update“) → **Pakete** → die `.msix` hochladen →
   **An den Store übermitteln**.

Testversionen (`-beta.1`) gibt es nur auf GitHub.

## Einreichung beim ersten Mal

| Abschnitt | Angabe |
|---|---|
| Preise und Verfügbarkeit | Kostenlos · alle Märkte · öffentlich sichtbar |
| Eigenschaften → Kategorie | Foto und Video (Unterkategorie, falls gefragt: keine) |
| Eigenschaften → Datenschutzrichtlinie | `https://github.com/suebi76/preset-manager/blob/main/PRIVACY.md` |
| Eigenschaften → Website | `https://suebi76.github.io/preset-manager/` (Projektseite, `docs/index.html`) |
| Eigenschaften → Support | `https://github.com/suebi76/preset-manager/issues` |
| Eigenschaften → Systemanforderungen | Mindestens: Tastatur, Maus; Windows 11, x64 |
| Altersfreigaben | Fragebogen (IARC): Kategorie „Dienstprogramm/Produktivität“, überall „Nein“ – kein Austausch zwischen Nutzern, keine Käufe, keine Standortdaten. Ergebnis: ab 0 / 3+ |
| Pakete | die `.msix` aus dem Artefakt |
| Store-Einträge | Deutsch und Englisch, Texte unten |
| Einreichungsoptionen → Hinweise für die Zertifizierung | Text unten („Notes for certification“) |

### Bildschirmfotos

Mindestens eins je Sprache, 1366 × 768 oder größer, PNG. Die
README-Bilder passen (1386 × 873, mit der Demo-Bibliothek, ohne private
Daten):

- Englisch: `docs/screenshot.png`, `docs/screenshot-deactivated.png`
- Deutsch: `docs/bildschirmfoto.png`, `docs/bildschirmfoto-deaktiviert.png`

Store-Logo (falls gefragt, 300 × 300): `packaging/msix/Assets/Square150x150Logo.scale-200.png`.

## Texte für den Store – English

**Short description**

Organize, understand and switch on or off the presets and profiles of
Photoshop and Lightroom Classic.

**Description**

Preset Manager keeps your Camera Raw presets and profiles in order – the ones
Photoshop and Lightroom Classic share.

• Add presets: drop folders or ZIP files, see what will be installed before
anything is copied. Already installed presets are recognized.
• Understand them: a short description in plain words, a before/after preview
on your own photo, and every changed setting as in Camera Raw.
• Switch them on or off: deactivated presets disappear from Photoshop and
Lightroom but stay available here.
• Organize: rename, move between groups, change the order, merge groups,
export as ZIP, move to the Recycle Bin.
• Safe: undo and redo, backups of replaced files, files are written
completely or not at all.

No account, no ads, no data collection. Free and open source (MIT):
github.com/suebi76/preset-manager

Adobe, Photoshop, Lightroom and Camera Raw are trademarks of Adobe Inc.
Preset Manager is not affiliated with Adobe.

**Features** (one per line)

- Import presets from folders and ZIP files with a preview
- Before/after preview and a plain-words description of each preset
- Activate and deactivate presets, profiles and whole groups
- Rename, regroup, reorder and merge groups
- Export a selection or group as a ZIP file
- Undo and redo, with backups of replaced files
- English and German

**Keywords** (max. 7)

presets · profiles · XMP · Camera Raw · Lightroom · Photoshop · photo editing

**What's new in this version** – aus `CHANGELOG.md`, Abschnitt „English“.

## Texte für den Store – Deutsch

**Kurzbeschreibung**

Presets und Profile von Photoshop und Lightroom Classic ordnen, verstehen
und ein- oder ausschalten.

**Beschreibung**

Preset Manager hält deine Camera-Raw-Presets und -Profile in Ordnung – die,
die Photoshop und Lightroom Classic gemeinsam nutzen.

• Presets hinzufügen: Ordner oder ZIP-Dateien hineinziehen und vorher sehen,
was installiert wird. Bereits installierte Presets werden erkannt.
• Verstehen: eine kurze Beschreibung in Worten, Vorher/Nachher auf deinem
eigenen Foto und alle geänderten Einstellungen wie in Camera Raw.
• Ein- und ausschalten: Deaktivierte Presets verschwinden aus Photoshop und
Lightroom, bleiben hier aber erhalten.
• Verwalten: umbenennen, in andere Gruppen verschieben, Reihenfolge ändern,
Gruppen zusammenführen, als ZIP exportieren, in den Papierkorb legen.
• Sicher: Rückgängig und Wiederholen, Sicherungen ersetzter Dateien, Dateien
werden ganz oder gar nicht geschrieben.

Kein Konto, keine Werbung, keine Datensammlung. Kostenlos und quelloffen
(MIT): github.com/suebi76/preset-manager

Adobe, Photoshop, Lightroom und Camera Raw sind Marken der Adobe Inc.
Preset Manager steht in keiner Verbindung zu Adobe.

**Funktionen** (je Zeile eine)

- Presets aus Ordnern und ZIP-Dateien importieren, mit Vorschau
- Vorher/Nachher und eine Beschreibung in Worten für jedes Preset
- Presets, Profile und ganze Gruppen aktivieren und deaktivieren
- Umbenennen, umgruppieren, sortieren, Gruppen zusammenführen
- Auswahl oder Gruppe als ZIP exportieren
- Rückgängig und Wiederholen, mit Sicherungen ersetzter Dateien
- Deutsch und Englisch

**Suchbegriffe** (höchstens 7)

Presets · Profile · XMP · Camera Raw · Lightroom · Photoshop · Bildbearbeitung

**Neuerungen in dieser Version** – aus `CHANGELOG.md`, Abschnitt „Deutsch“.

## Begründungen für die eingeschränkten Funktionen

Unter „Übermittlungsoptionen“, je höchstens 500 Zeichen. Die erste
Einreichung (19.11.2026) wurde am 23.11.2026 wegen `unvirtualizedResources`
abgelehnt (Richtlinie 10.6.3): zu wenig Begründung, es fehlte eine
Support-Adresse oder Website **in der Begründung selbst**. Seitdem steht
beides darin; als Support-Adresse die E-Mail des Herausgebers (nicht im
Repo, siehe Partner Center).

**runFullTrust**

> Preset Manager is a Win32 desktop app (Rust, egui) packaged as MSIX. It
> needs runFullTrust to run as a regular desktop app: it reads and writes
> preset files (.xmp) in the Camera Raw folders used by Adobe Photoshop and
> Lightroom Classic, imports folders and ZIP files the user picks or drops,
> and moves files to the Windows Recycle Bin. No network access, no
> elevation. Open source: github.com/suebi76/preset-manager

**unvirtualizedResources**

> Core purpose: install presets so Photoshop/Lightroom Classic can use them.
> Adobe reads them only from %APPDATA%\Adobe\CameraRaw and offers no install
> API; virtualized, imported files stay private and never appear in Adobe
> apps. Win11: writes limited by ExcludedDirectories to Adobe\CameraRaw and
> the app's own PresetManager folders. Open source. Website:
> https://github.com/suebi76/preset-manager Support: (E-Mail) or
> https://github.com/suebi76/preset-manager/issues

## Notes for certification

Unter „Zusätzliche Testinformationen“. Seit der zweiten Einreichung
ausführlicher: Kontakt, warum es keinen anderen Weg gibt (Adobe liest nur
diesen Ordner, keine Schnittstelle), im Versuch nachgewiesen, die drei
Ausnahmen einzeln begründet, Sicherheit (Papierkorb, Sicherungen,
Rückgängig, quelloffen), Testanleitung. Kurzfassung der ersten Einreichung:

> Preset Manager is a desktop app (Rust, full trust) that manages the Camera
> Raw presets and profiles used by Adobe Photoshop and Lightroom Classic. It
> is open source: https://github.com/suebi76/preset-manager
>
> **Why `unvirtualizedResources`:** the app's only purpose is to add, rename,
> move and deactivate files in `%APPDATA%\Adobe\CameraRaw`, the folder that
> Photoshop and Lightroom Classic read their presets and profiles from. With
> AppData write virtualization, imported presets would land in the package's
> private folder and never appear in Photoshop or Lightroom. The exclusion
> is limited to three folders (`virtualization:ExcludedDirectories`):
> `RoamingAppData\Adobe\CameraRaw`, and the app's own
> `RoamingAppData\PresetManager` (settings) and `LocalAppData\PresetManager`
> (backups, undo log, log files, which the user can open in Explorer from
> the settings). The Store version therefore requires Windows 11.
>
> **How to test without Adobe software:** start the app. If no Camera Raw
> folder exists, it offers to choose one. Any folder works; the app lists the
> `.xmp` presets inside `Settings` and the profiles inside `CameraProfiles`.
> Test data: a folder with a few `.xmp` presets, or a ZIP of presets, can be
> imported with “Import presets”.
>
> The app makes no network connections and has no account or in-app
> purchases. Updates come only from the Store.

## Nach der Freigabe

- Store-Link ins README (Abschnitt „Download“ / „Herunterladen“), mit dem
  Hinweis: nur eine Fassung je PC installieren – beide bearbeiten dieselben
  Ordner und nutzen dieselben Einstellungen, ein Wechsel ist also ohne
  Verlust möglich.
- Roadmap 1.10 abhaken.
