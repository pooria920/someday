# Phase 1 decisions

> Markdown authoring (2026-11-02): New note and Edit text accept typed Markdown and offer an optional live preview below the textarea. The insertion toolbar was removed at the user's request. Task lists retain visible, accessible completed/incomplete markers in previews and saved items. Source remains Markdown and follows existing draft/save behavior. Sanitization, inert links and remote-image blocking remain. No native command or schema changes. Verification after toolbar removal: 91 Angular tests, 72 Rust tests, rustfmt, frontend/CSP build and Windows NSIS packaging passed. Earlier browser Axe checks of the preview renderer passed in dark/light; the simplified layout has not been rechecked in a browser. Native save/restart behavior was not manually exercised. The existing app.css size warning remains.

> Always on top (2026-11-02): an off-by-default header toggle uses the main-only set_panel_on_top boolean command. Native topmost state also suppresses the final deferred external-focus-loss dismissal check. Disabling resumes dismissal; explicit hiding remains available. State lasts for the current process, with no database/backup changes. Verification: 89 Angular tests, 72 Rust tests, rustfmt and frontend/CSP checks passed. Browser Axe including contrast passed for both pressed states in light/dark mode at 360 × 420 with mocked native IPC and no horizontal overflow. Native topmost/focus-loss behavior has not been manually exercised.

> Appearance extension (2026-11-02, schema v9): Settings offers five complete palettes (Lime, Violet, Ocean, Rose, Amber), each with light/dark variants and miniature previews. The existing main-only set_theme saves the mode and optional closed palette atomically; header mode toggles preserve the palette. Migration v9 appends a constrained color_palette defaulting to lime without changing existing rows, mode or drafts. Backups preserve the palette and older backups migrate to lime. Failed writes retain the previous appearance. Verification: 87 Angular tests, 72 Rust tests, frontend/CSP build, rustfmt and Windows NSIS packaging passed. Browser Axe checks including contrast found no violations across all ten palette/mode combinations in Settings and Inbox using mocked native IPC; no horizontal overflow at 360px width. Native appearance persistence was not manually exercised. The existing app.css size warning remains.

> Current approved extension (2026-11-02, schema v8): items can be pinned in Inbox/Archive and pinned rows sort first without changing insertion history. An in-memory content-type filter combines All/Text/Links/Images with search and category filters. The New note draft now persists in SQLite across normal exit/restart, with stale-write protection, safe error/retry feedback and a dirty-draft exit flush. A successful manual save clears the draft in the insertion transaction; duplicate/failed saves keep it. Main-only IPC is set_item_pinned, load_note_draft and save_note_draft. Backups include pins/draft, restore replaces them and rescue retains the previous draft. Inline edit drafts still remain in memory only. This supersedes older draft restart exclusions below. See AGENTS.md for the complete current behavior.

Someday is a new sibling project. Daystash source, user data and migrations are not reused or modified.

- Keep Angular standalone, strict TypeScript, Signals, Tauri 2 and Rust. No dashboard, browser extension, process inspection or workday/session features.
- SQLite stays in Rust via bundled rusqlite. The webview receives no SQL, path, filesystem or shell privileges. Explicit application permissions restrict storage reads, item/category mutations and panel hiding to the main window; feedback has status/event access only.
- Database initialization/migration and reads run on a blocking worker. Migration uses an immediate transaction and rejects unknown future schemas. No reset or sample records.
- Native errors become fixed safe messages; clipboard text is never logged or read in this phase.
- Foundation panel is 420 by 480 pixels and stays alive when hidden. Escape and the close button hide it; tray Open Someday returns it; tray Quit exits. The single-instance plugin returns the existing panel.
- Windows 10/11 are the initial target. Platform-specific behavior belongs behind native adapters. Mobile is planned for later; this desktop scaffold does not claim mobile support. Do not introduce desktop dependencies into reusable content/data logic.
- Selection capture, persisted lists, delete/archive/restore and category management are implemented. Search and open actions are later phases. No disabled pretend search or non-working capture buttons.
- Active-monitor placement, auto height and durable theme preferences remain later integration/polish work. The current center operation targets the existing window monitor. Native corner/shadow appearance depends on Windows; no transparency tricks.

## Plan

1. Foundation: panel, storage schema/migrations, restricted IPC, launch/build verification.
2. Capture: explicit clipboard reads, HTTP(S) detection, save shortcut, persisted creation, brief confirmation and persistent duplicate suppression.
3. Interactions: persisted item list/search, open/copy, complete/restore/delete, keyboard navigation and toggle shortcut.
4. Integration: shortcut conflicts, active-monitor focus behavior, native lifecycle checks and measurements.
5. Polish: accessibility audit, persisted theme, error states and remaining tests.

## Sources checked

- https://v2.tauri.app/start/prerequisites/
- https://tailwindcss.com/docs/installation/framework-guides/angular
- npm registry stable tags checked during setup; lockfiles are the exact resolved versions.

## Current extension: selection capture

The approved selection-capture plan supersedes the original copy-first capture flow. Ctrl + Shift + S saves selected text directly with UI Automation, with a guarded one-shot Ctrl+C fallback for unsupported non-password controls. The fallback intentionally replaces the clipboard; there is no automatic restoration or clipboard history polling. The capture phase originally retained schema v1; the approved item-actions extension below upgrades it to v2. Rust owns registration, worker execution, deduplication and persistence. Both local surfaces subscribe to a safe typed capture-result event; only the non-focusable feedback window is shown during capture. Item browsing and management are covered by the extension below.

VS Code/Monaco input proxies can expose empty or incomplete UI Automation text. A bounded raw ancestor scan recognizes `native-edit-context` or the combined `inputarea monaco-mouse-cursor-text` classes; a lone `inputarea` class or the generic `monaco-editor` container is insufficient. Recognized proxies always use guarded Ctrl+C to obtain full and multicursor selections. For `Code.exe` and `Code - Insiders.exe`, empty/whitespace-only selections or text-pattern read failures also use metadata-required copying, while ordinary nonempty fields retain direct selection. Password/focus checks, fresh clipboard sequence/source ownership and Windows integrity restrictions remain in force. Fresh Chromium metadata must explicitly rule out an empty selection; missing/malformed metadata and whole-line copies from an empty selection are rejected. Copying replaces the clipboard. Other applications remain best-effort. Live VS Code verification of this change is pending because desktop automation cannot attach to its window (window-owner mismatch).


## Current extension: item actions and categories

Schema v2 appends a categories table and a nullable item category foreign key with ON DELETE SET NULL. Existing text/link content and timestamps are preserved. Archived items reuse completed/completed_at; restoration clears the completion timestamp. Category names are trimmed, nonempty and unique with SQLite exact-text comparison. No categories are seeded.

The main panel has Inbox/Archive views, a single-category filter and always-visible category/archive/delete controls outside the content disclosure. Item deletion requires confirmation and is permanent. Category deletion also requires confirmation and preserves its items as uncategorized. Categories can be created, renamed and deleted; a new category can be immediately assigned to an item. No trash, multi-category tags or extra category styling.

Rust performs all validation and persistence on blocking workers. list_items returns items/categories/pendingCount from one read transaction; explicit mutation commands are permitted only in the main window. Angular uses Signals, discards stale reads and refreshes after mutations/capture. Failed mutations keep content visible with safe errors. Native dialogs trap focus; Escape dismisses a dialog before hiding the panel. Capture deduplication checks exact content against all persisted items, including archived items, with no time limit. The check and insertion share an immediate transaction, preventing concurrent duplicate saves. Matches preserve existing items, categories and timestamps. Deleting all matching items allows recapture; existing duplicate rows are not removed. Whitespace, case and line endings remain significant.

## Current extension: Settings and custom shortcuts

Settings opens from the main header and currently contains only keyboard shortcuts. Both global actions support Ctrl and/or Alt, optional Shift, and A–Z, 0–9, Space or F1–F12. Duplicate and unsupported combinations are rejected in Rust. Schema v3 appends a singleton shortcut-settings table without changing existing items/categories. Startup loads stored choices on a blocking worker before registration. Changes register new combinations before persisting or releasing previous ones; failures remove the new registrations and retain the old choices. Swapping the two actions reuses their registrations. The guarded capture fallback waits for all supported keys to be released. The main-only set_shortcuts command exposes settings, never input injection or raw capture. Future settings can be added to the same surface when approved; no other settings are implemented.

Saved-item copying: the Copy item icon in Inbox and Archive copies the full saved text or link, preserving whitespace and line endings. Rust looks up the saved item by ID and writes Windows Unicode clipboard data on a blocking worker. Only the main window has copy_item permission; no arbitrary clipboard read/write API is exposed. Success and safe failure feedback appear in the panel. Copy replaces the current clipboard contents without changing stored items.

Manual notes: New note in the panel header opens a labeled multiline editor. Save note uses main-only save_note IPC and the same Rust validation, exact deduplication and SQLite transaction as selection capture, without reading the clipboard. Content remains unchanged; a single HTTP(S) URL is detected as a link. Successful saves clear the draft and show the unfiltered Inbox. Empty/oversized notes are rejected; duplicate or storage failures retain the draft. Cancel or hiding the panel retains the draft in memory until saved or the app exits; drafts are not persisted across app restarts. No note editing, rich text or autosave is included.

## Current extension: images

The approved image extension adds an Add image dialog with Choose file and Paste image. The native picker accepts a single static PNG, JPEG or WebP and preserves its original bytes. Explicit Paste image snapshots only PNG/DIB clipboard data and stores lossless PNG. Neither route changes the global selected-text shortcut or reads the clipboard in the background. File-dialog cancellation is neutral; successful saves show the unfiltered Inbox, and duplicates retain existing status, category and timestamps.

Schema v4 rebuilds items transactionally, preserving every existing field, category, setting and index while adding an image type and constrained image-only columns. Original bytes, a <=320px PNG thumbnail, detected MIME, dimensions and SHA-256 hash live in the same row. Hash plus byte comparison prevents exact duplicates across both views. Saving and deleting images remain atomic without sidecar files; migration failure rolls back and future schemas are rejected. Image data is local, unencrypted and increases database/backup size.

Rust validates file/encoded PNG size (20 MiB), pixel count (40 megapixels), each dimension (16,384px), decoding allocation limits and static formats. Raw clipboard snapshots have a 192 MiB ceiling before copying and are decoded on a blocking worker with bounded PNG/BMP decoders. Orientation and alpha are retained in previews and image clipboard copy. Native clipboard operations share a lock with text writes and the guarded selection-copy fallback.

Main-only import_image, paste_image and load_item_image commands are explicitly permitted; the feedback surface receives no image content or new privileges. The webview sends saved item IDs rather than filesystem paths. Lists select metadata without BLOBs. Original/thumbnail bytes are loaded separately through binary IPC and temporary Blob URLs, revoked on replacement/close/destroy. CSP gains only img-src 'self' blob:. Existing item disclosure, copy, category, archive/restore and confirmed permanent deletion are reused. Export, drag/drop, OCR, animation, remote fetching and a new global shortcut are outside scope.


## Current extension: automatic clipboard images

Settings now contains an off-by-default, immediately applied checkbox for automatic image saving. Schema v5 appends auto_save_images to shortcut_settings with a boolean CHECK; old rows, image payloads, categories and custom shortcuts are preserved. When enabled, a Rust worker uses the already-installed clipboard-win monitor feature (Windows clipboard update messages, no polling). It reuses the image snapshot, decoding limits, deduplication and transaction. The listener is created only while enabled, resumed from the saved preference at startup, and stopped on disable. Generations cancel in-flight saves; the settings lock spans persistence so disabling cannot return before an earlier save completes. Failed activation/persistence keeps the old setting. Startup/enabling skip existing clipboard data; fresh events are checked by clipboard sequence inside the shared clipboard lock, with bounded retries for contention. Text/file paths and own image copies are ignored. Own image writes retain their clipboard sequence because arboard may leave the owner null.

Background saves update count/list and safe status through capture-result without showing a popup or taking focus. A deferred list refresh runs after busy actions finish. Main-only set_auto_save_images exposes a boolean setting, never arbitrary clipboard access. The checkbox explains local/private-image behavior and the requirement that screenshots reach the clipboard. Screen capture itself, folder watching, autostart and new shortcuts remain outside scope.


## Current extension: dismiss panel on external focus

Rust handles main-window Focused(false) by rechecking visibility/focus on the UI thread after 100ms and hiding only when focus remains outside. Windows modal ownership/disabled-parent checks keep the native image picker and owned popups usable. Image import also schedules a recheck after its picker returns. The feedback surface is excluded. Hiding does not exit the process, destroy the WebView, discard in-memory note drafts, stop shortcuts or stop the enabled clipboard-image monitor. Tray Open Someday, the panel shortcut and single-instance activation continue to show/focus the panel. No new Angular listener, settings flag, IPC, permission or schema change.


## Formatted text and Markdown (approved scope, schema v6)

Explicit selection capture now checks UI Automation first, then performs one guarded Ctrl+C to obtain fresh Unicode text and optional Windows HTML Format from the same clipboard snapshot. This replaces the current clipboard even when direct selection reading succeeds. Password, focus, size, ownership and VS Code selection-metadata checks remain mandatory. A failed copy may retain a still-valid direct selection as plain text; old clipboard text is never used. There is no background text monitoring.

Schema v6 appends nullable formatted_html, with a 4 MiB UTF-8 limit and text/link-only constraint. Rust validates CF_HTML byte offsets and sanitizes its fragment using ammonia before storage. Allowed structure includes headings, emphasis, lists, quotations, tables and code. Only bounded bold/italic/underline/strikethrough inline styles survive; source colors, fonts, layout, images, scripts and external resources are removed. Original content, whitespace and line endings remain unchanged. Exact-text deduplication still includes the archive and never changes existing formatting.

Expanded text items prefer saved HTML, otherwise render Markdown (including existing items and manual notes). Raw Markdown HTML is displayed literally. DOMPurify sanitizes rendered output, limited inline styles become semantic elements, and normal Angular sanitization remains enabled. Links do not navigate, remote images do not load, and CSP stays unchanged. Show original text / Show formatted text is an in-memory per-item toggle. Plain links remain plain text. Persian paragraphs use automatic direction; code blocks use LTR and wide code/tables scroll locally.

Copy item looks up content by saved ID in Rust and writes Unicode text plus sanitized CF_HTML when saved formatting exists. Markdown-only items copy their original source. IPC command signatures and main-only permissions are unchanged; SavedItem adds formattedHtml (nullable; null for images). Clipboard operations remain serialized with image copy, image monitoring and selection capture.

File import for .md, RTF, rich-text editing, source-color/font fidelity and syntax highlighting are outside this feature. This section supersedes earlier plain-text-only rendering and copy-fallback descriptions; historical verification sections describe their original checkpoints.


Long-selection capture: after the guarded copy passes clipboard freshness/ownership, focus and editor-metadata checks, its full text takes precedence over a differing UIA result. Some UIA providers return only visible text. Preserve UIA line endings only when both complete texts match after newline normalization. Copy failure may still use the valid direct-text fallback. Never truncate at the size limit; reject oversized text.


## Search, link opening and panel keyboard (2026-10-31)

Approved and implemented: search filters the current Inbox/Archive view together with its category, matching text, URLs and image names. Matching normalizes Arabic/Persian yeh and kaf, case, diacritics and spacing without changing saved content. Image pixels are not searched. Arrow keys focus item summaries (Down also enters from search); Enter toggles details; Ctrl+C copies the focused summary's saved item unless browser text is selected. Editable fields and dialogs retain their keyboard behavior. Escape keeps its existing dialog/panel behavior.

Open link explicitly opens a saved link in the default browser. Main-only open_item accepts only an item ID; Rust retrieves a link row and validates HTTP(S) plus hostname before invoking the Windows URL handler. No arbitrary URL, shell command or filesystem API is exposed. Saved formatted-text links remain inert. No database migration. This supersedes earlier statements excluding search and open actions.


## Everyday details (approved 2026-10-31, schema v7)

Theme selection is saved in SQLite as a constrained nullable light/dark preference; null initially follows the OS. Main-only set_theme applies the new choice only after a successful write, retaining the old theme on failure. Migration v7 appends theme without changing existing items, categories, shortcuts or image settings.

Panel show paths (initial launch, shortcut, tray and single-instance activation) target the monitor containing the mouse pointer, falling back to existing centering if unavailable (approved 2026-11-02). Position uses the monitor work area with DPI-adjusted dimensions, clamps oversized panels and supports negative monitor coordinates.

Edit text is a plain multiline editor for saved text/link items in either view. Main-only update_note takes saved ID and new source, enforces the existing text size/empty rules and exact duplicate checks excluding itself in one immediate transaction. Content whitespace, item ID, date, category and archive state remain unchanged except the explicitly edited content/type; changed content clears stale saved HTML, unchanged content preserves it. The UI warns that changing text removes formatting. Failed saves retain the editor draft. New-note drafts remain separate and in memory. No rich-text editor or edit history.

Save image to file uses main-only export_image with a saved ID. Rust looks up original bytes and MIME and opens a native Save As dialog with a format-specific extension. Cancel changes nothing. Bytes are written and synced to a new temporary file beside the chosen destination, then atomically replace it using Windows MoveFileExW; failed writes retain the old destination and clean up the temporary file. Original PNG/JPEG/WebP bytes are preserved; no clipboard or general frontend filesystem API. Native picker ownership and dismissal recheck are retained. This supersedes previous exclusions of editing, image export and persistent theme/active-monitor positioning.


## Latest-item duplicate policy (approved 2026-10-31)

New text/link/image saves compare only with the most recently inserted existing item, across all types and including Archive. SQLite rowid order defines insertion order independently of wall-clock changes. A different intervening item allows recapturing identical content. Text remains exact (including whitespace/line endings); image hash plus exact original bytes are compared only against that one latest row. Checks and inserts remain in the same immediate transaction, including automatic image saves. Editing also checks only the latest item, excluding the edited ID without falling back to older rows; unchanged content stays a no-op. Deletion naturally exposes the previous latest existing row; archive/category/edit actions do not reorder insertion history. No schema change or existing-row cleanup. This supersedes earlier all-items duplicate policy descriptions.


## Local backup and restore (2026-10-31)

Settings > Backup & restore creates a manual `.someday-backup` file containing all Inbox/Archive items, exact text and saved HTML, original images/thumbnails, categories and settings. The file is an unencrypted standalone SQLite snapshot; keep it private. Use a destination outside Someday's app-data folder. There is no cloud upload, schedule, merge or encryption.

Restore backup opens a native picker, validates a private staged copy and shows its filename/item count before replacement. Confirming replaces the entire database after an automatic restart; save note drafts first, since in-memory drafts do not survive restart. The source backup stays unchanged. Supported older schemas migrate on the staged copy; unknown/newer schemas, unexpected schema objects, invalid constraints/foreign keys, oversized text, unsafe HTML or invalid image data are rejected.

Before replacement on startup, Someday saves the latest current database to a timestamped `backups/rescue-<timestamp>-<id>.someday-backup` under its app-data directory. A failed rescue write aborts replacement. The panel reports the result and rescue path. To undo a successful restore, choose that rescue file using Restore backup. Rescue files are retained until you remove them yourself and can contain private data. Shortcut conflicts and image-monitor startup failures use the existing Settings/tray recovery behavior.

Rust owns the dialogs and file operations on blocking workers. `create_backup`, `prepare_restore` and `restore_backup` are main-only commands; Angular receives an opaque temporary token, never arbitrary filesystem access. SQLite's backup API creates a consistent snapshot, including committed WAL data. Snapshot publication and startup replacement use synced temporary files plus Windows atomic replacement. A durable pending marker and staged-file hash recognize interruption after replacement without importing the same backup again. Startup restore runs before IPC storage work, shortcut registration and automatic image monitoring. No new database migration is introduced.


Backup verification (2026-10-31): 72 Angular tests and 64 Rust tests passed, including AXE structure checks in jsdom (contrast disabled), byte/format/settings round-trip, rowid preservation through old-schema migration, committed WAL snapshots, rejected schema/constraint/image corruption, stale tokens, rescue-write failure and interrupted-replacement recovery. Rustfmt and frontend/CSP checks passed. An isolated `com.someday.backup-qa` native run opened Settings and its owned Save As dialog without dismissing the panel; the Windows UI helper could not reliably control the filename input, so end-to-end picker/automatic-restart/tray behavior remains unverified. Actual disk exhaustion, full visual contrast checks and installer installation were not exercised. The existing app.css budget warning remains.
