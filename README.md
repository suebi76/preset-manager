<div align="center">

<a href="README.md"><img src="https://img.shields.io/badge/English-2680eb?style=for-the-badge" alt="English (this page)" height="36"></a>
&nbsp;
<a href="README.de.md"><img src="https://img.shields.io/badge/Auf%20Deutsch%20lesen-f0f0f0?style=for-the-badge" alt="Auf Deutsch lesen" height="36"></a>

<br><br>

<img src="crates/preset_manager/assets/icon.png" width="96" alt="Preset Manager icon">

# Preset Manager

**Manage the presets and profiles of Photoshop and Lightroom Classic in one place – organize them, understand them, change them safely.**

Free and open source · for Windows 10 and 11

[![Latest release](https://img.shields.io/github/v/release/suebi76/preset-manager?label=download&color=2680eb)](https://github.com/suebi76/preset-manager/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/suebi76/preset-manager/total?color=9256d9)](https://github.com/suebi76/preset-manager/releases)
[![CI](https://github.com/suebi76/preset-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/suebi76/preset-manager/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-2d9d78)](LICENSE)
![Windows 10/11](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-555)

![Preset Manager showing a demo library of presets and profiles](docs/screenshot.png)

</div>

## Why?

Photoshop and Lightroom Classic can import presets on their own. But once your
collection grows to a few hundred presets and profiles from many packs,
keeping it tidy there gets tedious: restructuring groups, moving many presets
at once, finding duplicates, understanding what a preset actually does.

**Preset Manager** works directly on the presets and profiles that Photoshop,
Photoshop (Beta) and Lightroom Classic use – no separate library, no copies on
the side. Whatever you change in Preset Manager, Adobe shows exactly that
after a restart.

- **See everything at once** – all installed presets and profiles, separated
  just like in Camera Raw, with search and filters.
- **Keep it tidy** – rename, move between groups, reorder, create and merge
  groups, for many items at once: by drag & drop, right-click or keyboard.
- **Activate and deactivate** – keep your whole collection, but decide what
  Photoshop and Lightroom show. Deactivated items are listed separately and
  return to exactly the same place.
- **Understand each preset** – a plain-language summary (“brighter, faded
  blacks, warmer, film grain”) plus its sliders, tone curve and color
  grading drawn like in Camera Raw.
- **Add new packs** – drop a folder or ZIP into the window, even nested ZIPs.
  Already installed presets are detected; Photoshop and Lightroom Classic
  get everything at once.
- **Never lose a preset** – deleted items go to the Recycle Bin, originals
  are backed up before anything is changed, and files are never
  half-written.

The interface speaks **English and German** – it follows the Windows
language and can be switched in the settings.

## Download

1. Download **[PresetManager-Setup.exe](https://github.com/suebi76/preset-manager/releases/latest)**
   from the latest release and run it.
2. That's it – no administrator rights needed. Preset Manager appears in the
   Start menu and on the desktop and **updates itself** (with a notice, or
   automatically if you prefer).

Prefer no installation? Take `PresetManager-Portable.zip` from the same
release, unzip it and start `PresetManager.exe`.

The installer is not code-signed, so Windows SmartScreen asks once
(“More info → Run anyway”). If you would rather not trust a binary, build it
yourself from the source code – see [Build from source](#build-from-source).

## Features

| | |
|---|---|
| **Add** | Folders and ZIP files by drag & drop or file dialog · preview before anything is copied · already installed presets are detected (same UUID) · on name clashes: skip, replace (with backup) or keep both · drop onto a group to install straight into it |
| **Understand** | See what a preset does before you use it: a plain-language summary and every changed setting, arranged like in Camera Raw – sliders, tone curve graph, color grading wheels, color mixer, effects, masks |
| **Browse** | Presets and profiles in separate sections, with type icons · search (Ctrl+F) · filters (Ctrl+1/2/3) · details: name, group, origin, file, UUID |
| **Activate & deactivate** | Hide presets and profiles from Photoshop and Lightroom without deleting them – single items, selections or whole groups (eye icon, right-click, Ctrl+D, drag & drop); deactivated ones are listed separately and come back to exactly the same place, favorites included |
| **Organize** | Rename (F2) · delete to Recycle Bin (Del) · move to another group · reorder (Alt+↑/↓) · new group from selection (Ctrl+G) · merge groups · multi-select (Ctrl/Shift+click, Ctrl+A) |
| **Safe** | Backups of the last 20 actions · atomic writes · ZIP checks against path tricks, ZIP bombs and oversized files · works only inside the Adobe folders |
| **Comfort** | English and German interface · hint when Photoshop or Lightroom is running (they show changes after a restart) · adjustable interface size · log file for troubleshooting · self-update |

## How it works

Photoshop, Photoshop (Beta) and Lightroom Classic read presets from one shared
folder, `%APPDATA%\Adobe\CameraRaw\Settings`. Preset Manager finds it
automatically and installs each pack into its own subfolder. If Lightroom
Classic stores presets with your catalog, the settings list that folder too
and let you switch with one click. It edits only the name, group and sort order
inside the `.xmp` files and leaves all other settings untouched. Restart
Photoshop or Lightroom afterwards to see the changes.

| What | Where |
|---|---|
| Presets and profiles | `%APPDATA%\Adobe\CameraRaw\Settings\<pack>\` |
| The program | `%LOCALAPPDATA%\PresetManagerApp\` |
| Settings | `%APPDATA%\PresetManager\` |
| Backups, log files | `%LOCALAPPDATA%\PresetManager\` |

Uninstalling (Windows Settings → Apps) removes only the program; your
presets, settings and backups stay.

## Build from source

Needs [Rust](https://rustup.rs) (stable).

```bash
cargo run --release -p preset_manager --bin preset_manager
```

Or double-click `Start.bat` – drop a folder or ZIP onto it to open the import
preview right away. Try it without touching your own presets:
`scripts/demo_bibliothek.py` creates a demo library (`--english` for English
names), and the environment variable `APM_CAMERA_RAW_DIR` points the app to it.

The project is planned and documented in German in [`Planung`](Planung/00-Uebersicht.md)
(requirements, architecture, error handling, tests, roadmap).
`crates/preset_core` holds all logic and is tested without a window;
`crates/preset_manager` is the [egui](https://github.com/emilk/egui) app plus
the command-line tool `preset_cli`. Releases are built on GitHub with
[Velopack](https://velopack.io) (`scripts/New-Release.ps1`).

## Support the project

If Preset Manager saves you time, **give it a ⭐ on GitHub** – it helps other
photographers find it. Bugs and ideas are welcome as
[issues](https://github.com/suebi76/preset-manager/issues).

## License and trademarks

MIT License, see [LICENSE](LICENSE).

Adobe, Photoshop, Lightroom and Camera Raw are trademarks of Adobe Inc. This
project is not affiliated with, endorsed or reviewed by Adobe.
