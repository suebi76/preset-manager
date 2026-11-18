<div align="center">

<img src="crates/preset_manager/assets/icon.png" width="96" alt="Preset Manager icon">

# Preset Manager

**Install and organize Camera Raw presets and profiles for Photoshop and Lightroom Classic – by drag & drop, safely, in one place.**

[![Latest release](https://img.shields.io/github/v/release/suebi76/preset-manager?label=download&color=2680eb)](https://github.com/suebi76/preset-manager/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/suebi76/preset-manager/total?color=9256d9)](https://github.com/suebi76/preset-manager/releases)
[![CI](https://github.com/suebi76/preset-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/suebi76/preset-manager/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-2d9d78)](LICENSE)
![Windows 10/11](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-555)

**English** · [Deutsch](README.de.md)

![Preset Manager showing a demo library of presets and profiles](docs/screenshot.png)

</div>

## Why?

You bought a preset pack, unzipped it – and now? Adobe expects the `.xmp`
files in a hidden folder deep inside `AppData`, Photoshop and Lightroom
Classic share it, profiles and presets look alike, and one wrong click
overwrites a preset you paid for.

**Preset Manager** takes care of that:

- **Drop a folder or ZIP into the window** – it finds every preset and
  profile inside (even in nested ZIPs), shows a preview and installs them
  into the right place for **Photoshop, Photoshop (Beta) and Lightroom
  Classic** at once.
- **See what you have** – presets and profiles separated, just like in
  Camera Raw, with search and filters.
- **Keep it tidy** – rename, delete, move between groups, reorder, create
  and merge groups by drag & drop, right-click or keyboard.
- **Never lose a preset** – deleted items go to the Recycle Bin, originals
  are backed up before anything is replaced or changed, and files are never
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
| **Install** | Folders and ZIP files by drag & drop or file dialog · preview before anything is copied · already installed presets are detected (same UUID) · on name clashes: skip, replace (with backup) or keep both · drop onto a group to install straight into it |
| **Browse** | Presets and profiles in separate sections, with type icons · search (Ctrl+F) · filters (Ctrl+1/2/3) · details: name, group, origin, file, UUID |
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
