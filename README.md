# Buni Font Manager

A Windows font manager built for Illustrator and Affinity work: import Envato zips in one go, build project collections, compare everything with your own sample text, and keep only the fonts you need active.

## Install

Open the [Releases page](../../releases), download the file ending in **-setup.exe** and run it. Windows may show "Windows protected your PC" the first time: click **More info**, then **Run anyway**.

## What works in 0.1

- Drag zips, OTF or TTF files (or folders) anywhere onto the window, or use **Import fonts**. Nested zips are opened, duplicates are skipped, licence files are kept, and two families from the same zip are tagged **Duo**. A report shows what happened.
- Fonts are activated without installing them, and re-activated after a restart (the app starts with Windows and sits in the tray).
- Collections: create, rename, delete, drag fonts onto them, favourites per collection, each collection remembers its sample text, "Deactivate everything else".
- Compare fonts with your sample text: size, padding, capitals, grid or list, focus mode, dark mode.
- Filters: source, style, spacing, weight, languages (Swedish, Portuguese, Cyrillic, Greek) and other tags. Search by name, sort by name or recently added.
- Tags: read from the file (bold), plus your own; right-click a tag to remove it. Works on several fonts at once.
- Undo (Ctrl+Z) for collection, tag and delete actions.

Coming next: Adobe Fonts and system fonts in collections, AI tagging, Generate Artwork (Illustrator, Affinity, PDF).

## Where your fonts live

`C:\Users\<you>\Buni Font Manager\Library`, organised by origin and family. The library index is `library.json` next to it.

## For developers

- `crates/bfm-core`: reading fonts, importing, the library file (no OS-specific code; `cargo test -p bfm-core`).
- `src-tauri`: the desktop app (Tauri 2). OS-specific code lives in `src-tauri/src/platform.rs`.
- `src`: the interface (React + TypeScript).
- Every push to `main` builds the Windows installer with GitHub Actions and publishes it on the Releases page.
