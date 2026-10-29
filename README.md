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

## New in 0.4

- Generate Artwork for Illustrator: the app has Illustrator build the specimen as an .ai file with live text in the real fonts (one A3 artboard per page). If Illustrator can't be reached, the script is saved next to the file to run from File > Scripts > Other Script.
- New app icon.
- Artwork files are named "Buni Font Manager_[collection]".
- Same width mode never cuts previews; All caps updates the card width; previews aren't clipped at the top or bottom.
- Add to collection stays at the top in focus mode; the import report shows how many files were dropped; quieter sliders.

## New in 0.3

- Generate Artwork: an A3 PDF specimen of a collection (button at the top right of a collection).
- Drop Illustrator files (.ai) to see the fonts they use: fonts from your library are turned on, missing ones are listed, and each name can be copied.
- Focus mode: choose the text and background colours (with the last 10 colours remembered).
- Same width mode: every preview scaled to the same width.
- Better style detection (Candara, Century Gothic, blackletter, scripts); Script and Handwritten are now one style.
- With a weight filter on, cards show that weight.
- List view scrolls; cards follow the typical preview width; faster Size slider; darker sliders in dark mode.

## New in 0.2

- System fonts and Adobe Fonts appear in All fonts and the Sources filter, and can go into collections and get tags. They can't be deactivated or deleted from here.
- Add recent Adobe Fonts (today or this week) to a collection with the clock button.
- The import report warns when an imported font has the same name as one already on the computer.
- Clear filters button; previews aligned left or centred; cards stretch to fill the row.
- Adobe Fonts folder can be changed in Settings.

## Where your fonts live

`C:\Users\<you>\Buni Font Manager\Library`, organised by origin and family. The library index is `library.json` next to it.

## For developers

- `crates/bfm-core`: reading fonts, importing, the library file (no OS-specific code; `cargo test -p bfm-core`).
- `src-tauri`: the desktop app (Tauri 2). OS-specific code lives in `src-tauri/src/platform.rs`.
- `src`: the interface (React + TypeScript).
- Every push to `main` builds the Windows installer with GitHub Actions and publishes it on the Releases page.
