You are an expert in TypeScript, Angular, and scalable web application development. You write functional, maintainable, performant, and accessible code following Angular and TypeScript best practices.

## TypeScript Best Practices

- Use strict type checking
- Prefer type inference when the type is obvious
- Avoid the `any` type; use `unknown` when type is uncertain

## Angular Best Practices

- Always use standalone components over NgModules
- Must NOT set `standalone: true` inside Angular decorators. It's the default in Angular v20+.
- Do NOT set `changeDetection: ChangeDetectionStrategy.OnPush` explicitly. `OnPush` is the default in Angular v22+.
- Use signals for state management
- Implement lazy loading for feature routes
- Do NOT use the `@HostBinding` and `@HostListener` decorators. Put host bindings inside the `host` object of the `@Component` or `@Directive` decorator instead
- Use `NgOptimizedImage` for all static images.
  - `NgOptimizedImage` does not work for inline base64 images.

## Accessibility Requirements

- It MUST pass all AXE checks.
- It MUST follow all WCAG AA minimums, including focus management, color contrast, and ARIA attributes.

## Components

- Keep components small and focused on a single responsibility
- Use `input()` and `output()` functions instead of decorators
- Use `model()` for two-way bound properties with `[(prop)]` syntax instead of pairing `input()` with `output()`
- Use `computed()` for derived state
- Use `linkedSignal()` for state derived from multiple reactive sources that must stay synchronized
- Prefer inline templates for small components
- Prefer Signal Forms (`@angular/forms/signals`) for new forms. They are stable in Angular v22+ and provide signal-based state, type-safe field access, and schema-based validation
- When not using Signal Forms, prefer Reactive forms instead of Template-driven ones
- Do NOT use `ngClass`, use `class` bindings instead
- Do NOT use `ngStyle`, use `style` bindings instead
- Do NOT import `CommonModule`, import only the directives and pipes the template uses, such as `AsyncPipe` or `DatePipe`
- When using external templates/styles, use paths relative to the component TS file.

## State Management

- Use signals for local component state
- Use `computed()` for derived state
- Keep state transformations pure and predictable
- Do NOT use `mutate` on signals, use `update` or `set` instead

## Templates

- Keep templates simple and avoid complex logic
- Use native control flow (`@if`, `@for`, `@switch`) instead of `*ngIf`, `*ngFor`, `*ngSwitch`
- Use the async pipe to handle observables
- Do not assume globals like (`new Date()`) are available.

## Services

- Design services around a single responsibility
- Use the `providedIn: 'root'` option for singleton services
- Prefer the `@Service` decorator over `@Injectable({providedIn: 'root'})` for new singleton services (Angular v22+)
- Use the `inject()` function instead of constructor injection

## Someday product boundaries

Someday: Save now. Deal with it someday. A tiny local-first inbox for text, links and images, initially Windows 10/11, mobile in later phases. See PRODUCT-BRIEF.md and FOUNDATION.md. Foundation, explicit selection capture, global panel show/hide (Ctrl + Shift + Space), item delete/archive/restore and single-category management/filtering are implemented. Search, open actions, mobile, sync, accounts, telemetry, AI and autostart remain out of scope.

Angular owns presentation; Rust owns native work and SQLite. All frontend Tauri imports/invoke calls stay under src/app/core/platform/tauri. Keep explicit command permissions in build.rs and capabilities/main.json in sync. Never expose arbitrary SQL, shell or filesystem access. Map native errors to safe user messages.

SQLite is stored under Tauri app_data_dir as someday.db, with independent com.someday.desktop identity. Append migrations, reject newer versions, preserve rows, and never reset on failure. Use short immediate migration transactions and blocking workers for storage. Read selected text/clipboard only during an explicit capture shortcut. Never log captured content. UI Automation comes first; automatic Ctrl+C is a guarded fallback and leaves the selected text in the clipboard. Reject known password/empty selections, stop on focus changes, and never reuse old clipboard text. Shortcut registration stays in Rust; do not expose input injection or raw capture commands to Angular.

Prefer the smallest complete implementation. Reuse installed Iconsax icons through ng-icons and CSS tokens; Tailwind v4 via PostCSS. No large UI libraries, generic frameworks or empty speculative layers. Default Angular 22 OnPush and standalone behavior applies. Use Signals and inject().

Settings includes custom global capture/panel shortcuts, persisted in SQLite schema v3. Rust validates and registers combinations before saving, retaining prior shortcuts on failure. Keep dynamic UI shortcut hints in sync. New settings require approved scope; do not add placeholders for future options.

Tests: npm test, npm run build, cargo test --manifest-path src-tauri/Cargo.toml, cargo fmt --manifest-path src-tauri/Cargo.toml --check, and npm run tauri:build. Changes to IPC update both Rust and TypeScript. Test important migration/constraint behavior. Report native behaviors as verified only when actually exercised. Tray lifecycle is a required desktop surface; future mobile integrations must remain separate from reusable data logic.

Saved-item copying: the Copy item icon in Inbox and Archive copies the full saved text or link, preserving whitespace and line endings. Rust looks up the saved item by ID and writes Windows Unicode clipboard data on a blocking worker. Only the main window has copy_item permission; no arbitrary clipboard read/write API is exposed. Success and safe failure feedback appear in the panel. Copy replaces the current clipboard contents without changing stored items.

Manual notes: New note in the panel header opens a labeled multiline editor. Save note uses main-only save_note IPC and the same Rust validation, exact deduplication and SQLite transaction as selection capture, without reading the clipboard. Content remains unchanged; a single HTTP(S) URL is detected as a link. Successful saves clear the draft and show the unfiltered Inbox. Empty/oversized notes are rejected; duplicate or storage failures retain the draft. Cancel or hiding the panel retains the draft in memory until saved or the app exits; drafts are not persisted across app restarts. No note editing, rich text or autosave is included.

Images: Add image offers a native single-file picker and an explicit Paste image action. Only paste_image may read clipboard image data; this approved action extends the capture-only clipboard rule above without adding polling or changing selection capture. Rust preserves original static PNG/JPEG/WebP file bytes, converts clipboard PNG/DIB to PNG, validates 20 MiB/40 megapixel/16,384px limits before pixel decoding, and caps raw clipboard input at 192 MiB. Schema v4 stores the original, a <=320px PNG thumbnail, hash and metadata in the item transaction. Exact original-byte deduplication includes the archive; hashes only narrow the comparison. Lists contain metadata, never image payloads. Main-only load_item_image accepts an item ID and a closed thumbnail/original variant and returns binary bytes; Blob URLs are revoked when discarded. Native clipboard access is serialized with text copy/selection fallback. Copy item dispatches text or image pixels by saved ID. Archive/category/delete behavior is shared. No general filesystem/clipboard API, remote image fetching, export, drag/drop, OCR, animation or extra global shortcut is included.


Automatic images (approved scope): Settings includes an immediately applied, persisted opt-in checkbox, off by default. Schema v5 adds a constrained auto_save_images boolean without changing existing rows/settings. While enabled and the app is running, Rust subscribes to Windows clipboard change events, reads only fresh image formats and reuses bounded image processing, exact deduplication and transactional storage. No text capture, polling, disk-folder watching or clipboard writes occur in the watcher. Starting/enabling establishes a baseline so old clipboard images are not imported. Disabling stops the listener and invalidates in-flight work before returning; failed setting writes retain the prior behavior. Ignore Someday image-copy sequences (arboard can leave a null owner) and own-process clipboard owners. Serialize native clipboard snapshots with selection/copy. Emit background capture-result updates without showing/focusing the panel or feedback popup. Only main has set_auto_save_images permission. Screenshots saved only to disk are outside this setting. This explicit opt-in supersedes earlier capture-only clipboard-reading rules while enabled.


Panel dismissal: Rust hides the main panel on sustained external focus loss after a short deferred recheck. Keep Windows owned popups/native picker usable and recheck focus when import returns. Only main auto-hides; never turn outside clicks into application exit or destroy the WebView/drafts. Tray, shortcuts and opt-in image monitoring must continue while hidden. No extra setting or frontend/global mouse listener is needed.


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

Panel show paths (shortcut, tray and single-instance activation) target the foreground window's monitor, falling back to the cursor monitor and then existing centering. Position uses the monitor work area with DPI-adjusted dimensions, clamps oversized panels and supports negative monitor coordinates.

Edit text is a plain multiline editor for saved text/link items in either view. Main-only update_note takes saved ID and new source, enforces the existing text size/empty rules and exact duplicate checks excluding itself in one immediate transaction. Content whitespace, item ID, date, category and archive state remain unchanged except the explicitly edited content/type; changed content clears stale saved HTML, unchanged content preserves it. The UI warns that changing text removes formatting. Failed saves retain the editor draft. New-note drafts remain separate and in memory. No rich-text editor or edit history.

Save image to file uses main-only export_image with a saved ID. Rust looks up original bytes and MIME and opens a native Save As dialog with a format-specific extension. Cancel changes nothing. Bytes are written and synced to a new temporary file beside the chosen destination, then atomically replace it using Windows MoveFileExW; failed writes retain the old destination and clean up the temporary file. Original PNG/JPEG/WebP bytes are preserved; no clipboard or general frontend filesystem API. Native picker ownership and dismissal recheck are retained. This supersedes previous exclusions of editing, image export and persistent theme/active-monitor positioning.


## Latest-item duplicate policy (approved 2026-10-31)

New text/link/image saves compare only with the most recently inserted existing item, across all types and including Archive. SQLite rowid order defines insertion order independently of wall-clock changes. A different intervening item allows recapturing identical content. Text remains exact (including whitespace/line endings); image hash plus exact original bytes are compared only against that one latest row. Checks and inserts remain in the same immediate transaction, including automatic image saves. Editing also checks only the latest item, excluding the edited ID without falling back to older rows; unchanged content stays a no-op. Deletion naturally exposes the previous latest existing row; archive/category/edit actions do not reorder insertion history. No schema change or existing-row cleanup. This supersedes earlier all-items duplicate policy descriptions.


## Local backup and restore (2026-10-31)

Settings > Backup & restore creates a manual `.someday-backup` file containing all Inbox/Archive items, exact text and saved HTML, original images/thumbnails, categories and settings. The file is an unencrypted standalone SQLite snapshot; keep it private. Use a destination outside Someday's app-data folder. There is no cloud upload, schedule, merge or encryption.

Restore backup opens a native picker, validates a private staged copy and shows its filename/item count before replacement. Confirming replaces the entire database after an automatic restart; save note drafts first, since in-memory drafts do not survive restart. The source backup stays unchanged. Supported older schemas migrate on the staged copy; unknown/newer schemas, unexpected schema objects, invalid constraints/foreign keys, oversized text, unsafe HTML or invalid image data are rejected.

Before replacement on startup, Someday saves the latest current database to a timestamped `backups/rescue-<timestamp>-<id>.someday-backup` under its app-data directory. A failed rescue write aborts replacement. The panel reports the result and rescue path. To undo a successful restore, choose that rescue file using Restore backup. Rescue files are retained until you remove them yourself and can contain private data. Shortcut conflicts and image-monitor startup failures use the existing Settings/tray recovery behavior.

Rust owns the dialogs and file operations on blocking workers. `create_backup`, `prepare_restore` and `restore_backup` are main-only commands; Angular receives an opaque temporary token, never arbitrary filesystem access. SQLite's backup API creates a consistent snapshot, including committed WAL data. Snapshot publication and startup replacement use synced temporary files plus Windows atomic replacement. A durable pending marker and staged-file hash recognize interruption after replacement without importing the same backup again. Startup restore runs before IPC storage work, shortcut registration and automatic image monitoring. No new database migration is introduced.
