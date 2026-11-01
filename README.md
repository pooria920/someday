# Someday

**Save now. Deal with it someday.**

A small local-first Windows inbox for text, links and images. Capture selected text with a global shortcut, write a note, or save an image, then review everything in a compact panel. Data stays on your computer, with no accounts, cloud sync or telemetry.

## Features

- Save selected text and HTTP(S) links from other applications.
- Create and edit plain-text notes, with Markdown rendering and sanitized captured formatting.
- Import static PNG, JPEG and WebP files, paste clipboard images, or opt into automatic clipboard-image saving.
- Copy saved text or image pixels; open saved links in the default browser; export original image files.
- Search Inbox or Archive by text, URL or image name, combined with category filtering.
- Assign one optional category per item; create, rename and delete categories.
- Archive, restore and permanently delete items with confirmation.
- Customize global shortcuts and save a light/dark theme preference.
- Create local backups and restore the complete database with a recovery snapshot.

The resizable, frameless panel starts at 420 × 480 pixels. Drag the app name or empty space in the header to move it. Showing it targets the foreground window's monitor, with cursor-monitor and centering fallbacks. Hiding it keeps Someday running in the tray; **Quit** exits. Launching a second instance returns to the existing panel.

## Getting started

The initial desktop target is **Windows 10/11**. Build prerequisites:

- Node.js **22.22.3+ on the 22.x line**, **24.15.0+ on the 24.x line**, or **26+**, matching the locked Angular dependencies.
- npm (the workspace declares `npm@11.13.0`).
- Rust **1.93+**, using the MSVC toolchain.
- Microsoft C++ Build Tools with **Desktop development with C++** and a Windows SDK.
- Microsoft Edge WebView2 Runtime.

From the repository root:

```powershell
npm ci
npm start
```

`npm start` runs the Angular development server at `http://127.0.0.1:4201` and launches the native Tauri application. The browser-only preview (`npm run web:serve`) has no native storage bridge and shows a storage error; use the desktop app for working storage and native actions.

Build the Windows installer:

```powershell
npm run tauri:build
```

The NSIS installer is written under `src-tauri/target/release/bundle/nsis/`. Current development installers are unsigned.

## Everyday use

### Shortcuts and panel

| Shortcut | Action |
| --- | --- |
| Ctrl + Shift + S | Save selected text globally |
| Ctrl + Shift + Space | Show/hide the panel globally |
| Ctrl + F | Focus and select the panel search query |
| Up / Down | Move focus between item summaries; Down also enters from search |
| Enter | Expand/collapse the focused item |
| Ctrl + C | Copy the focused item's saved content when no browser text is selected |
| Escape | Dismiss the active dialog or hide the panel |

Editable fields and dialogs retain their keyboard behavior. The global shortcuts work while Someday is running and can be changed in **Settings**. Changes are validated and registered before saving; failures retain the prior shortcuts. Conflicts appear as warnings and do not prevent startup.

The close button and sustained focus loss hide the panel without discarding in-memory drafts. Owned native dialogs remain usable. Open Someday again through its shortcut or tray menu.

### Text, links and notes

Select text in another app and use the capture shortcut. Someday checks Windows UI Automation, then performs one guarded Ctrl+C to obtain fresh text and optional HTML formatting. **Capture replaces the clipboard**, including when UI Automation succeeds. Password/empty selections, stale clipboard data and focus changes are rejected. A failed copy may still use a valid direct selection as plain text. Compatibility depends on the source application; Someday does not elevate itself or bypass Windows restrictions.

Text is preserved with its whitespace and line endings, up to 1,048,576 UTF-16 units; oversized text is rejected rather than truncated. A single HTTP(S) URL becomes a link. **Open link** opens a saved link explicitly in the default browser.

**New note** opens a multiline editor without reading the clipboard. A successful save clears the draft and shows the unfiltered Inbox. Canceling or hiding the panel retains the draft in memory; quitting or restarting loses it. Failed saves retain the draft. **Edit text** updates an existing text/link item without changing its identity, category, date or archive state. Changing its source removes saved HTML formatting; saving unchanged text preserves it.

Expanded text prefers sanitized captured HTML, otherwise renders Markdown. Raw Markdown HTML is displayed literally, embedded links stay inert and remote images do not load. **Show original text / Show formatted text** switches the display for that item in memory. Copy preserves the original text and includes sanitized HTML when saved formatting exists; Markdown-only items copy their source.

### Images

Use **Add image → Choose file** or **Paste image**. File imports preserve original static PNG/JPEG/WebP bytes; clipboard PNG/DIB images become PNG. Copying a file in Explorer or an image URL is different from copying image pixels.

Limits are **20 MiB**, **40 megapixels** and **16,384 pixels per side**, with raw clipboard input capped at **192 MiB**. Unsupported, animated, invalid and oversized images are rejected. Originals and thumbnails (up to 320 pixels per side) are stored in SQLite, so moving the source file does not affect saved items.

**Copy item** writes image pixels to the clipboard. **Save image to file** exports the original bytes through a native Save As dialog. Canceling changes nothing; failed writes preserve an existing destination.

**Automatically save clipboard images** is an opt-in setting, off by default. While enabled and Someday is running, fresh clipboard images are saved without showing or focusing the panel. Enabling establishes a baseline rather than importing an old clipboard image. Disabling stops monitoring and invalidates pending work. Someday ignores its own image copies. There is no background text capture, polling, folder watching or import of screenshots saved only to disk.

### Search, categories and duplicates

Search filters the current Inbox/Archive view and selected category. It matches text, URLs and image names, normalizing case, Arabic/Persian yeh and kaf, diacritics and spacing without changing saved content. It does not search image pixels.

Each item has at most one category. Deleting a category keeps its items as uncategorized. Archiving and restoring preserve content; permanent deletion requires confirmation.

New saves compare only with the **most recently inserted existing item**, including Archive and all item types. Identical consecutive text or original image bytes are suppressed. Saving A, then B, then A keeps all three. Text comparison includes whitespace and line endings; different encodings of the same picture can remain separate. Archive, category and edit actions do not reorder insertion history. Editing checks the latest item, excluding itself without falling back to older items; unchanged content is a no-op.

## Backup and restore

In **Settings → Backup & restore**, create a manual `.someday-backup` file outside Someday's app-data directory. It contains all Inbox/Archive items, exact text and saved HTML, original images/thumbnails, categories and settings as a standalone SQLite snapshot, including committed WAL data.

**Backups and the local database are unencrypted and may contain private content.** Keep them private. There is no cloud upload, schedule, merge or backup encryption.

Restore validates a staged copy and shows its filename and item count before confirmation. Confirming replaces the entire database after an automatic restart. **Save note drafts first:** in-memory drafts do not survive restart. The source backup is unchanged. Supported older schemas migrate on the staged copy; unknown/newer schemas and invalid or unsafe contents are rejected.

Before replacement, Someday saves the current database under its app-data directory as:

```text
backups/rescue-<timestamp>-<id>.someday-backup
```

A failed rescue write aborts replacement. The panel reports the result and rescue path. To undo a restore, choose that rescue file through **Restore backup**. Rescue files remain until you remove them yourself. Durable staging and replacement markers support recovery after interruption.

## Architecture and data

Angular 22 standalone components, strict TypeScript and Signals own presentation. Tailwind CSS v4, CSS tokens, Iconsax via ng-icons and bundled Vazirmatn provide styling and offline Persian glyphs. Tauri 2 and Rust own Windows integration, clipboard access, dialogs and SQLite through bundled rusqlite.

The database is `someday.db` under Tauri's `app_data_dir`, normally:

```text
%APPDATA%\com.someday.desktop\someday.db
```

The current schema is **v7**. Append-only transactional migrations preserve existing rows and reject newer schemas without resetting data. Storage and file work run on blocking workers. Native clipboard operations are serialized. Captured content must never be logged.

All frontend Tauri imports and invocations stay under `src/app/core/platform/tauri/`. Explicit command permissions in `src-tauri/build.rs` and `src-tauri/capabilities/main.json` restrict access to the main window; the feedback window has narrower permissions. Angular receives typed data and safe errors, with no arbitrary SQL, shell, filesystem or clipboard API. Formatting is sanitized in Rust and again in the frontend, with Angular sanitization and CSP retained.

| Location | Responsibility |
| --- | --- |
| `src/app/app.*` | Main panel, item interactions and component tests |
| `src/app/items/` | Saved text and image presentation |
| `src/app/settings/` | Shortcut, automatic-image and backup settings |
| `src/app/core/platform/tauri/native.ts` | Typed native boundary |
| `src-tauri/src/lib.rs` | IPC composition and desktop lifecycle |
| `src-tauri/src/database.rs` | SQLite storage, migrations and storage tests |
| `src-tauri/src/backup.rs` | Backup validation, staging and recovery |
| `src-tauri/src/capture.rs`, `windows_selection.rs`, `windows_clipboard.rs` | Selection capture and native clipboard integration |
| `src-tauri/src/images.rs`, `auto_images.rs` | Image processing and opt-in monitoring |
| `src-tauri/src/formatted_text.rs` | Captured HTML validation and sanitization |
| `src-tauri/migrations/` | Ordered SQLite migrations |
| `src-tauri/capabilities/` | Window-specific native permissions |

## Development checks

Run from the repository root:

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
npm run tauri:build
```

`npm run rust:test` is an alias for the Rust tests. The frontend build also runs `scripts/check-frontend-build.mjs` to validate production output and CSP expectations.

Automated tests and packaging do not verify actual Windows clipboard, tray, shortcut, native dialog, multi-monitor or installer behavior. Exercise relevant native paths when changing them and report only checks actually performed. Accessibility requires WCAG AA and Axe checks; jsdom structure checks alone do not verify visual contrast.

See [AGENTS.md](AGENTS.md) for contributor rules and approved scope. [PRODUCT-BRIEF.md](PRODUCT-BRIEF.md) and [FOUNDATION.md](FOUNDATION.md) record product intent and foundation decisions; some historical phase descriptions predate implemented features.

## Scope

Mobile is a later phase; this is a Windows desktop build. Cloud sync, accounts, telemetry, AI, autostart, OCR, animated images, drag and drop, rich-text editing, Markdown/RTF file import and syntax highlighting remain outside the current scope.
