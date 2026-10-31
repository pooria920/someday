# Someday

Save now. Deal with it someday.

A small local-first Windows utility for saving links, text and images for later. Select text and press Ctrl + Shift + S, write a note, or add an image from the panel. Items are displayed newest first and support copying, categories, archive/restore and confirmed permanent deletion.

## Prerequisites

- Node.js compatible with Angular 22 (this workspace uses Node 22+), npm
- Rust 1.93+ with the MSVC toolchain
- Microsoft C++ Build Tools with Desktop development with C++
- Microsoft Edge WebView2 Runtime
- Windows 10/11 for the initial desktop target

See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Tailwind uses the official [Angular PostCSS integration](https://tailwindcss.com/docs/installation/framework-guides/angular).

## Run and build

```powershell
cd D:\projects\someday
npm ci
npm start
```

`npm start` launches Angular at 127.0.0.1:4201 and the native Tauri app. Daystash uses a different port and app identity.

```powershell
npm run web:serve
npm run build
npm test
npm run rust:test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
npm run tauri:build
```

The browser-only preview has no native storage bridge and displays a safe storage error. Use `npm start` for the working native foundation.

Production installers go under `src-tauri/target/release/bundle/nsis`. They are unsigned development builds.

## Current behavior

- Compact 420px frameless panel; dark/light theme toggle for the current process
- Clicking outside the panel or switching to another application hides it; Escape and the close button also hide it
- Tray menu: Open Someday and Quit
- Single instance; opening the executable again brings back the existing panel
- Real SQLite readiness round trip, no seeded/fake items
- Closing/hiding the panel keeps the process and capture shortcut alive; Quit ends them
- Shortcut conflicts are shown in the panel and do not prevent startup

## Shortcuts

Escape hides the panel. Both shortcuts are registered globally in Rust while Someday is running:

| Default shortcut | Action |
| --- | --- |
| Ctrl + Shift + S | Save selected text (implemented) |
| Ctrl + Shift + Space | Show/hide the panel |

Holding the panel shortcut toggles only once per press. Showing the panel focuses it; hiding keeps Someday running in the tray. Each shortcut registers independently. If another app reserves one, Someday shows a warning and keeps the other shortcut available.

Open **Settings** using the header gear to change either global shortcut. Choose Ctrl and/or Alt, optionally Shift, plus a letter, digit, Space or F1–F12. Each action needs a different combination. **Save changes** applies immediately and stores both shortcuts locally; **Reset to defaults** selects the original combinations, which are applied when saved. Closing Settings without saving discards edits. If registration or storage fails, previous settings and shortcuts remain active. Existing unavailable shortcuts can be retried by saving again. The native copy fallback waits for all supported shortcut keys to be released before sending Ctrl+C.

Selection capture first uses Windows UI Automation, with a guarded one-shot Ctrl+C fallback for unsupported non-password controls and the editor cases below. The fallback waits for shortcut keys to be released and replaces the clipboard with the copied text. Someday does not restore the previous clipboard and never continuously monitors it.

Someday checks a bounded raw UI Automation ancestor chain for `native-edit-context` or the combined Monaco classes `inputarea monaco-mouse-cursor-text`; neither a lone `inputarea` class nor the generic `monaco-editor` container is enough. Recognized proxies always use guarded copying because their accessibility text can omit part of a long or multicursor selection. In `Code.exe` and `Code - Insiders.exe`, an empty/whitespace-only UI Automation selection or a text-pattern read failure also uses this fallback; ordinary nonempty fields still use direct selection. Fresh Chromium clipboard metadata must explicitly confirm a nonempty editor selection. Whole-line copies from an empty selection and missing/malformed metadata are rejected.

Empty/password selections are rejected. Focus changes, inaccessible controls, copy failures and timeouts produce safe feedback without saving old clipboard content. Clipboard fallback requires both a changed sequence number and an owner from the source process. Other applications are best-effort; Ctrl+C semantics vary. Someday never elevates itself or bypasses Windows integrity restrictions.

A small non-focusable window shows the result for two seconds without opening the main panel. The panel count updates through a typed event. Before every save, Rust checks all stored items, including the archive, for exactly identical content in the same immediate transaction as insertion. A match returns "Already saved to Someday" without changing the item or count. This applies across restarts and intervening captures. Case, whitespace and line endings remain significant; similarity matching is not used. Permanently deleting all matching items allows saving the content again. Existing duplicates are preserved. A stalled UI Automation provider retains a single worker slot instead of accumulating threads. Selections are limited to 1,048,576 UTF-16 units; up to 32 disjoint ranges are joined with a newline. Native provider line endings are preserved (Notepad UI Automation may return CR instead of the file's LF).

## Storage and architecture

Angular standalone components and Signals handle the UI. Tailwind CSS v4 and local CSS tokens provide styling; Iconsax Outline icons come from ng-icons; bundled Vazirmatn supplies Persian glyphs offline. Rust owns bundled SQLite via rusqlite: the webview never receives SQL/filesystem access. Storage work runs on a blocking worker through narrowly permitted Tauri commands.

Windows data location: `%APPDATA%\com.someday.desktop\someday.db`, resolved through Tauri app_data_dir. Daystash data is untouched. Data is local, unencrypted SQLite; there are no network services, accounts or telemetry.

Schema v5 retains all text, links, images, categories and shortcuts and adds the optional automatic-image setting. Migrations run transactionally using user_version and reject future schemas without resetting data. Future schema changes must append migrations.

## Images

Image verification (2026-10-31): all 52 Angular tests and 43 Rust tests passed, as did rustfmt, the production frontend/CSP build and Tauri NSIS packaging (3.28 MiB). Twelve mocked-browser states covering both themes, 360px/420px layouts, collapsed/expanded images and the Add image error dialog had zero Axe violations and no overflow. An isolated native QA identity exercised the actual file picker, a schema-v3 upgrade preserving a Persian note/category/custom shortcuts, exact imported bytes, binary thumbnails, image copy/paste with alpha, duplicate suppression, Windows Alt+Print screenshot capture, archive/category/restore, process-restart persistence and permanent deletion. Native Axe had zero violations; feedback was denied all three image commands. The QA database reopened with integrity_check ok after deletion. The original production app and its database were left running/untouched; its executable was retained as target/release/someday-before-images.exe so the rebuilt executable could be produced without stopping it. Installer installation, actual browser context-menu Copy image, Windows 10 and forced OS clipboard races were not verified. The existing component CSS budget warning remains nonblocking.

Use **Add image** in the panel header, then **Choose file** or **Paste image**. Each action saves one image. File selection accepts static PNG, JPEG and WebP and keeps the original bytes and format. Clipboard PNG/DIB images become lossless PNG. Copying a file in Explorer or copying an image URL is not the same as copying image pixels; use the file picker or the browser's **Copy image** action.

Images are limited to 20 MiB, 40 megapixels and 16,384 pixels per side. Raw clipboard input is bounded to 192 MiB before decoding. Invalid, animated, unsupported and oversized images produce safe errors without storing an item. Canceling the picker changes nothing. A successful save shows the unfiltered Inbox; identical original bytes are suppressed across Inbox and Archive. Different encodings of the same picture remain separate items.

The original image and a PNG thumbnail (at most 320 pixels per side) are stored in the same SQLite item, independently of the source file. Removing or moving that source does not affect Someday. Listing returns only metadata; thumbnails and expanded images load separately by item ID. Expanding an image shows the full saved image scaled to the panel, including orientation and transparency. **Copy item** copies image pixels for pasting into another app and replaces the clipboard. Categories, archive/restore and confirmed permanent deletion work as they do for text.

By default only **Paste image** reads clipboard images. You can opt into automatic image saving in Settings as described below. The text-selection shortcut is unchanged; no extra global shortcut is registered. File selection, validation, image processing, clipboard access and SQLite remain in Rust, with no arbitrary file/clipboard API exposed to the webview. Image data is local and unencrypted, and increases the database and backup size. File export, drag and drop, OCR, animated images and mobile remain outside this feature.

## Files

- `src/app/app.*`: foundation panel and component tests
- `src/app/core/platform/tauri/native.ts`: typed native boundary
- `src-tauri/src/lib.rs`: IPC, composition and desktop lifecycle
- `src-tauri/src/database.rs`: SQLite migration and storage tests
- `src-tauri/migrations/0001_items.sql`: initial schema
- `src-tauri/capabilities/main.json`: narrow panel permissions
- `public/someday.svg`: source icon
- `PRODUCT-BRIEF.md`, `FOUNDATION.md`, `AGENTS.md`: product, scope and contributor guidance

Mobile is a later phase. Desktop tray/window integration will need separate mobile adapters; this foundation is not a mobile build.

## Limits and next phase

Panel sizing is fixed initially (resizable), and centering uses the existing window monitor. Active-monitor targeting, persisted theme, search and open actions are later work.

Next: search, open actions and expanded keyboard navigation.


## Foundation verification (2026-10-31; before selection capture)

- Production Angular build passed (134.73 kB initial raw JS/CSS).
- Angular tests: 3 passed (empty state/readiness, safe error mapping, Escape dispatch).
- Rust tests: 3 passed (constraints/idempotent migration, future-schema rejection, disk reopen persistence).
- cargo fmt --check passed.
- npm start compiled and launched the native Windows app. The panel displayed Local storage ready; the separate app-data database was created. Dark/light rendering was inspected in the native WebView.
- A second executable launch retained one Someday process and returned the existing panel. The hide button made the panel unavailable for window input while the process remained alive.
- npm run tauri:build produced src-tauri/target/release/bundle/nsis/Someday_0.1.0_x64-setup.exe (2.59 MiB).
- Installer installation, tray menu clicks, native Escape dismissal, Windows 10, multiple monitors and mobile were not verified. Escape command dispatch has an Angular test. At that checkpoint, global shortcuts and clipboard capture were not implemented.

## Selection-capture verification (2026-10-31)

- Angular tests: 8 passed, including capture updates, safe errors, shortcut conflicts, cleanup, feedback-only rendering and a capture arriving during the startup snapshot.
- Rust tests: 13 passed, covering Persian/multiline preservation, URL detection, empty/password rejection, copy failure/timeout, focus changes, fresh clipboard evidence, size limits, held/concurrent shortcuts, two-second deduplication (including intervening different content), SQLite failures, migration safety and disk persistence.
- cargo clippy --all-targets -- -D warnings and cargo fmt --check passed.
- Production Angular build and Tauri NSIS bundle passed. No new migration was added; user_version stayed 1 and SQLite integrity_check returned ok.
- Native Chrome and Edge captured the exact browser fixture text. Windows Notepad captured the selection with its provider's CR line endings. Notepad3 exercised automatic-copy compatibility and captured the exact LF file contents.
- Native empty Notepad selection and the synthetic Edge password field created no records. The live panel count and result messages updated. Feedback uses a non-focusable window; source selections remained in place.
- A controlled shortcut-holder process reserved Ctrl+Shift+S before Someday launched. Someday stayed running and displayed the shortcut-conflict message. The holder was then released.
- Six controlled fixture records survived app restarts. Only those fixture records were removed after testing; the database was not reset or migrated.
- Native focus-switch/clipboard-race/timeout conditions were covered through policy tests rather than forced OS races. Windows 10, multi-monitor behavior, installer installation and all possible third-party text providers remain unverified. There is an unavoidable foreground-change race between the final check and Windows input injection; no input is retried automatically.

To reproduce native capture checks, open tests/fixtures/selection.html in Chrome/Edge or tests/fixtures/selection.txt in Notepad, select text and press Ctrl+Shift+S. To reproduce registration conflicts, quit Someday, run cargo run --manifest-path src-tauri/Cargo.toml --example shortcut_holder, then start Someday. Press Enter in the fixture process to release the shortcut, and restart Someday to register it normally. The fixture is not bundled with the app.

## Packaged CSS / CSP

Production builds disable Angular's optimization.styles.inlineCritical so global CSS loads as a normal external stylesheet. Critical-CSS inlining creates an inline loading script and can add CSP nonces that interfere with runtime component styles in Tauri. Keep the existing CSP; do not enable unsafe-inline scripts. npm run build now runs scripts/check-frontend-build.mjs and rejects inline styles/scripts, CSS load handlers, script-dependent media attributes and missing global CSS before packaging.

Packaged CSS fix verified: the old generated HTML failed the CSS/CSP check; the rebuilt HTML passed with a normal external stylesheet and no inline loader. npm run tauri:build produced a fresh NSIS installer, and the rebuilt release executable was launched. Visual inspection could not be completed in this run because the desktop automation connection returned Transport closed. The installer itself was not installed during this check.

## Pending inbox list

The main panel now reads existing pending items through the restricted list_pending_items command on a blocking worker. Rows are ordered by saved time, newest first. Expand an item to read its full plain text and saved time. Successful captures refresh the list; older overlapping reads cannot replace a newer result. Load errors have a Retry action. The feedback window has no permission to read item contents. Existing rows/schema are unchanged; no migration or sample records are needed. Search, opening URLs, copying, completion and deletion remain future work.

Verification: 11 Angular tests and 14 Rust tests passed, including persisted-row listing, ordering/filtering, exact Persian/multiline text, text-only rendering, refresh after capture, load retries and overlapping request handling. Production frontend CSS/CSP checks passed.

The updated production Tauri/NSIS build passed, along with Clippy and rustfmt checks. The rebuilt release executable was started. Installer installation and native visual inspection were not repeated for this change; UI behavior is covered by component tests.

Toast overflow fix: removed the page-wide 320px minimum width, which exceeded the 300px feedback window and created horizontal/vertical scrollbars. Feedback now constrains its width/overflow and wraps message text inside the flex layout. Native window sizing and the main inbox scrolling behavior are unchanged.


## Item actions and categories

Each saved item has a category selector, archive/restore button and delete button, available while the text is collapsed. Delete permanently removes an item after confirmation. Switch between Inbox and Archive above the list; archived items can be restored. The counter always shows the total inbox size, independent of view/category filtering.

Use Manage categories to create, rename or delete categories. Each item has zero or one category; deleting a category preserves its items as uncategorized. The item selector also offers New category to create and select in one flow. Filter either view by all categories, uncategorized or a specific category. Category names support Persian and English and are trimmed, nonempty and unique (exact text). All changes persist locally in SQLite schema v2 through an additive migration; no database reset or seeded categories. Confirmed item deletion has no undo/trash.


Item-actions verification (2026-10-31): 21 Angular tests and 18 Rust tests passed, along with rustfmt, production Angular/CSP checks and the Tauri NSIS build. Browser interaction checks at 360px and default Axe checks passed across both themes, Inbox/Archive and dialogs. Native WebView2 checks exercised real IPC against an isolated QA identity: v1 migration, category create/assign/rename, archive/restore, category deletion preserving its item and confirmed item deletion. Archive/category state survived one process restart; deletion survived another. Feedback mutation permissions were denied. The final production identity launched with schema v2 and its capture shortcut registered; native default Axe checks, local Persian font and category-dialog focus/Escape passed. The app was restarted without the temporary debugging flag. Installer installation was not exercised. Angular reports a nonblocking component CSS budget warning (5.06 kB against a 4 kB warning threshold).


Panel shortcut verification (2026-10-31): Ctrl + Shift + Space now registers globally and toggles the main panel on the native UI thread. Both registrations are attempted independently, with separate conflict warnings. The repeat guards have a Rust test proving capture and toggle stay independent. All 24 Angular and 19 Rust tests, rustfmt, frontend/CSP build and Tauri NSIS build passed. The updated release executable was launched; the shortcut hid its visible window and showed it again while File Explorer was active. Local storage remained ready and no shortcut conflicts were reported. Holding the shortcut and native toggle-registration conflicts were not manually exercised. The existing component CSS size warning remains.

Settings verification (2026-10-31): 29 Angular tests, 26 Rust tests, rustfmt, frontend/CSP build and Tauri NSIS build passed. Chromium Settings checks at 360px and 420px in both themes had zero default Axe violations and no horizontal overflow; the short window scrolls the dialog. Actual WebView2 IPC registered and saved custom shortcuts, retained them across a process restart, rejected a real conflict held by the existing shortcut_holder fixture without replacing prior settings, and denied settings changes from the feedback window. Native Settings Axe had zero violations. Ctrl+Alt+F9 hid and showed the native panel from File Explorer. Original default shortcuts were restored and Someday was restarted without the debugging flag. Capturing real selected text with a custom shortcut and installer installation were not exercised. The existing component CSS warning remains (8 kB against a 4 kB warning threshold).

VS Code selection compatibility (2026-10-31): 20 Rust tests and 24 Angular tests passed, along with rustfmt, production frontend/CSP checks and the Tauri NSIS build. The release executable was rebuilt. Live VS Code capture could not be exercised because desktop automation rejected attaching to its window; native selection behavior remains to be confirmed. The existing component CSS budget warning remains.

Proxy/ancestor compatibility validation (2026-10-31): 31 Rust tests, 34 Angular tests, rustfmt, the production frontend/CSP build and the Tauri NSIS build passed. Installer installation was not exercised. Live VS Code confirmation remains pending: desktop automation reports a window-owner mismatch when attaching to VS Code. Capture remains best-effort across third-party applications, subject to their copy behavior and Windows integrity restrictions. The existing component CSS budget warning remains (8 kB against a 4 kB warning threshold).

Saved-item copying: the Copy item icon in Inbox and Archive copies the full saved text or link, preserving whitespace and line endings. Rust looks up the saved item by ID and writes Windows Unicode clipboard data on a blocking worker. Only the main window has copy_item permission; no arbitrary clipboard read/write API is exposed. Success and safe failure feedback appear in the panel. Copy replaces the current clipboard contents without changing stored items.

Manual notes: New note in the panel header opens a labeled multiline editor. Save note uses main-only save_note IPC and the same Rust validation, exact deduplication and SQLite transaction as selection capture, without reading the clipboard. Content remains unchanged; a single HTTP(S) URL is detected as a link. Successful saves clear the draft and show the unfiltered Inbox. Empty/oversized notes are rejected; duplicate or storage failures retain the draft. Cancel or hiding the panel retains the draft in memory until saved or the app exits; drafts are not persisted across app restarts. No note editing, rich text or autosave is included.


## Automatically save copied images and screenshots

In **Settings → Images**, enable **Automatically save copied images and screenshots**. The checkbox applies immediately and survives restarts; it is off by default. While Someday is running, new clipboard PNG/DIB images are saved locally using the same image limits and duplicate prevention as Paste image. Screenshots must reach the clipboard; a screenshot saved only as a file is not detected. Copied image URLs or Explorer file paths are not image pixels. Private images can be saved while this option is enabled, so disable it when you do not want clipboard images retained.

Enabling or restarting skips the image already on the clipboard. Turning the checkbox off stops monitoring and prevents outstanding captures from saving after the change returns. The watcher receives Windows change notifications rather than polling. It never reads clipboard text, writes to the clipboard or opens/focuses the panel. Copying an image from Someday is ignored. Errors are shown as safe panel status; invalid/oversized images are not stored. Extremely rapid clipboard replacements can supersede an image before it is snapshotted; there is no clipboard history recovery.

Automatic-image verification (2026-10-31): 55 Angular tests and 44 Rust tests passed, including v4-to-v5 data preservation/default-off persistence, rejected non-boolean storage values, setting failure rollback in the UI and a capture arriving while settings are busy. Actual native WebView2 checks with an isolated QA identity exercised checkbox enable/disable, external synthetic DIB events, exact duplicate suppression, no import of existing clipboard content on enable/startup, persisted opt-in and resumed watching after restart, own-image-copy suppression even with a null OS owner, feedback permission denial and zero Axe violations in Settings. The test watcher was disabled and its QA process stopped afterward; production data and its running app were not changed. Production frontend/CSP checks, rustfmt and NSIS packaging passed. Installer installation, Windows 10, screenshot-tool variations and extreme clipboard contention remain unverified; the existing component CSS budget warning is unchanged.


## Dismiss on outside focus

The main panel hides when it loses focus to another application, including an outside click or Alt+Tab. A short deferred check ignores transient WebView focus transfers and does not hide a refocused panel. Windows owned popups and the native image picker remain usable; after the picker returns, focus is checked again. The hidden panel remains alive, retaining note drafts and dialog state, with tray, global shortcuts and opt-in automatic image saving still running. Show it again using the configured panel shortcut, tray Open Someday or another executable launch. This is not application exit; Quit remains in the tray. No new setting, IPC, permission or database migration is added.

Outside-focus dismissal verification (2026-10-31): 55 Angular tests, 44 Rust tests, rustfmt and the production frontend/CSP check passed. This change is native-only and introduces no new permissions or data migration. Live outside-click/picker/draft-retention tests could not be completed: Computer Use stopped because it could not determine the browser URL confidently enough to enforce its policy. No native focus behavior is claimed as exercised. The isolated QA process was stopped, and the running production executable was retained as target/release/someday-before-dismiss.exe so the build can replace the release path without stopping the app. Installer installation remains unverified.


## Formatted text and Markdown (approved scope, schema v6)

Explicit selection capture now checks UI Automation first, then performs one guarded Ctrl+C to obtain fresh Unicode text and optional Windows HTML Format from the same clipboard snapshot. This replaces the current clipboard even when direct selection reading succeeds. Password, focus, size, ownership and VS Code selection-metadata checks remain mandatory. A failed copy may retain a still-valid direct selection as plain text; old clipboard text is never used. There is no background text monitoring.

Schema v6 appends nullable formatted_html, with a 4 MiB UTF-8 limit and text/link-only constraint. Rust validates CF_HTML byte offsets and sanitizes its fragment using ammonia before storage. Allowed structure includes headings, emphasis, lists, quotations, tables and code. Only bounded bold/italic/underline/strikethrough inline styles survive; source colors, fonts, layout, images, scripts and external resources are removed. Original content, whitespace and line endings remain unchanged. Exact-text deduplication still includes the archive and never changes existing formatting.

Expanded text items prefer saved HTML, otherwise render Markdown (including existing items and manual notes). Raw Markdown HTML is displayed literally. DOMPurify sanitizes rendered output, limited inline styles become semantic elements, and normal Angular sanitization remains enabled. Links do not navigate, remote images do not load, and CSP stays unchanged. Show original text / Show formatted text is an in-memory per-item toggle. Plain links remain plain text. Persian paragraphs use automatic direction; code blocks use LTR and wide code/tables scroll locally.

Copy item looks up content by saved ID in Rust and writes Unicode text plus sanitized CF_HTML when saved formatting exists. Markdown-only items copy their original source. IPC command signatures and main-only permissions are unchanged; SavedItem adds formattedHtml (nullable; null for images). Clipboard operations remain serialized with image copy, image monitoring and selection capture.

File import for .md, RTF, rich-text editing, source-color/font fidelity and syntax highlighting are outside this feature. This section supersedes earlier plain-text-only rendering and copy-fallback descriptions; historical verification sections describe their original checkpoints.


Formatted-text verification (2026-10-31): 59 Angular tests and 50 Rust tests passed. Coverage includes Markdown and semantic HTML display, original-text toggling across collapse, active-content/resource removal, fresh-selection matching and fallback policy, CF_HTML Unicode byte offsets and bounds, v5-to-v6 row/settings preservation, byte limits, and duplicate-format preservation across archive/copy. The saved-text component passed axe-core DOM checks in jsdom with color-contrast explicitly disabled because jsdom has no layout. Production Angular/CSP checks, rustfmt and the production Tauri NSIS build passed; the existing app.css warning threshold is still exceeded.

Live capture/paste in Chrome and VS Code was not verified: Windows Computer Use stopped browser interaction because it could not confidently determine the current URL for policy enforcement. Word was absent from the available app inventory. Real-browser contrast/full-panel AXE, installer installation and live native clipboard behavior remain unverified. Native fixture pages are tests/fixtures/formatted-text.html and tests/fixtures/formatted-paste.html. No QA application was launched against the user's database.


Long-selection capture: after the guarded copy passes clipboard freshness/ownership, focus and editor-metadata checks, its full text takes precedence over a differing UIA result. Some UIA providers return only visible text. Preserve UIA line endings only when both complete texts match after newline normalization. Copy failure may still use the valid direct-text fallback. Never truncate at the size limit; reject oversized text.


Long-selection regression verification (2026-10-31): 51 Rust tests passed, including a truncated UIA selection with a long full clipboard result and preservation of source line endings when normalized texts match. The existing app process was left running to preserve drafts; its executable was renamed to someday-running-before-text-fix.exe under target/release so the replacement could be built. Restart the app to use the fixed binary and recapture previously incomplete items. No saved user items were edited.


## Search, link opening and panel keyboard (2026-10-31)

Approved and implemented: search filters the current Inbox/Archive view together with its category, matching text, URLs and image names. Matching normalizes Arabic/Persian yeh and kaf, case, diacritics and spacing without changing saved content. Image pixels are not searched. Arrow keys focus item summaries (Down also enters from search); Enter toggles details; Ctrl+C copies the focused summary's saved item unless browser text is selected. Editable fields and dialogs retain their keyboard behavior. Escape keeps its existing dialog/panel behavior.

Open link explicitly opens a saved link in the default browser. Main-only open_item accepts only an item ID; Rust retrieves a link row and validates HTTP(S) plus hostname before invoking the Windows URL handler. No arbitrary URL, shell command or filesystem API is exposed. Saved formatted-text links remain inert. No database migration. This supersedes earlier statements excluding search and open actions.

Search/open verification (2026-10-31): 62 Angular tests and 52 Rust tests passed, together with production frontend/CSP checks, rustfmt check and Tauri NSIS packaging. Tests cover normalized Persian search, links/image names, combined category filtering, Archive search, safe open failures, summary keyboard behavior and stored-link validation. AXE in jsdom found no violations with color contrast disabled (no layout). Actual browser launching, native keyboard/focus behavior, full visual/contrast QA and installer installation were not exercised. Existing CSS warning remains. The running executable was retained as target/release/someday-running-before-search.exe to preserve drafts; restart Someday after saving any draft to use the newly built binary.

Expanded-item preview fix (2026-10-31): text/link summaries show their preview only while collapsed; expanding shows the full content once, and collapsing restores the preview. Image names retain their existing behavior. 64 Angular tests and 52 Rust tests passed, including text/link expansion, collapse and focus retention; frontend/CSP build and rustfmt check passed. Native visual verification was not performed.

Link action layout (2026-10-31): Open link is now an Iconsax icon button in the item action row beside Copy, with an accessible name and tooltip. Browser visual QA used the production Angular bundle with mocked native data at 360px (both themes) and 420px, including expanded content; the action row fits without overflow. 64 Angular tests (including SVG rendering/action-row placement and existing AXE checks), 52 Rust tests, rustfmt and frontend/CSP build passed. Native browser launch and installer installation were not repeated.

Search placement and keyboard focus (2026-10-31): search sits below the view heading and above the view/category controls with aligned gutters and fixed toolbar spacing. Ctrl+F focuses search and selects its current query while the main panel has no modal open; dialogs keep focus. 65 Angular tests, 52 Rust tests, rustfmt and production frontend/CSP checks passed. Production-bundle browser QA with mocked native data at 360px confirmed the new layout and actual Ctrl+F focus. Native WebView shortcut behavior and installer installation were not exercised.

Compact header (2026-10-31): reduced header/title/search/filter spacing and removed the decorative inbox subtitle. The view heading remains a semantic h2; header action targets remain 32x32px. Production-bundle browser review with mocked native data at 360px in both themes showed the item list starting at 179.5px (previously about 264px), with no panel overflow. 65 Angular tests, 52 Rust tests, rustfmt and frontend/CSP checks passed. Native visual QA and installer installation were not performed.


## Everyday details (approved 2026-10-31, schema v7)

Theme selection is saved in SQLite as a constrained nullable light/dark preference; null initially follows the OS. Main-only set_theme applies the new choice only after a successful write, retaining the old theme on failure. Migration v7 appends theme without changing existing items, categories, shortcuts or image settings.

Panel show paths (shortcut, tray and single-instance activation) target the foreground window's monitor, falling back to the cursor monitor and then existing centering. Position uses the monitor work area with DPI-adjusted dimensions, clamps oversized panels and supports negative monitor coordinates.

Edit text is a plain multiline editor for saved text/link items in either view. Main-only update_note takes saved ID and new source, enforces the existing text size/empty rules and exact duplicate checks excluding itself in one immediate transaction. Content whitespace, item ID, date, category and archive state remain unchanged except the explicitly edited content/type; changed content clears stale saved HTML, unchanged content preserves it. The UI warns that changing text removes formatting. Failed saves retain the editor draft. New-note drafts remain separate and in memory. No rich-text editor or edit history.

Save image to file uses main-only export_image with a saved ID. Rust looks up original bytes and MIME and opens a native Save As dialog with a format-specific extension. Cancel changes nothing. Bytes are written and synced to a new temporary file beside the chosen destination, then atomically replace it using Windows MoveFileExW; failed writes retain the old destination and clean up the temporary file. Original PNG/JPEG/WebP bytes are preserved; no clipboard or general frontend filesystem API. Native picker ownership and dismissal recheck are retained. This supersedes previous exclusions of editing, image export and persistent theme/active-monitor positioning.

Everyday-details verification (2026-10-31): 68 Angular tests and 56 Rust tests passed, along with rustfmt, production frontend/CSP validation and NSIS packaging. Coverage includes theme restoration/failure rollback, edit failure retention and saved-ID updates, export cancel/success/failure, v6-to-v7 preservation/default theme/constraints, unchanged-format preservation, duplicate/oversize rejection, exact original-byte file replacement and failure cleanup, and negative-coordinate/DPI/work-area panel geometry. Production-bundle browser QA at 360px with mocked native data exercised edit/open/save/return-focus and action-row layout. Real multiple-monitor/DPI movement, native Save As interaction, app-restart theme selection through real IPC and installer installation were not exercised. The existing app and in-memory drafts were retained; save drafts and restart to load the new executable. Existing CSS budget warning remains.


## Latest-item duplicate policy (approved 2026-10-31)

New text/link/image saves compare only with the most recently inserted existing item, across all types and including Archive. SQLite rowid order defines insertion order independently of wall-clock changes. A different intervening item allows recapturing identical content. Text remains exact (including whitespace/line endings); image hash plus exact original bytes are compared only against that one latest row. Checks and inserts remain in the same immediate transaction, including automatic image saves. Editing also checks only the latest item, excluding the edited ID without falling back to older rows; unchanged content stays a no-op. Deletion naturally exposes the previous latest existing row; archive/category/edit actions do not reorder insertion history. No schema change or existing-row cleanup. This supersedes earlier all-items duplicate policy descriptions.

Latest-item deduplication verification (2026-10-31): 57 Rust tests and 68 Angular tests passed, along with rustfmt, frontend/CSP validation and Tauri NSIS packaging. Coverage includes immediate duplicate suppression across reopen, A/B/A recapture, cross-type saves, latest archived rows, clock changes, concurrent saves and original-byte image comparison. Live shortcut and clipboard-image watcher scenarios were not manually repeated. Save any draft and restart Someday to use the updated executable; the running process was preserved.


## Local backup and restore (2026-10-31)

Settings > Backup & restore creates a manual `.someday-backup` file containing all Inbox/Archive items, exact text and saved HTML, original images/thumbnails, categories and settings. The file is an unencrypted standalone SQLite snapshot; keep it private. Use a destination outside Someday's app-data folder. There is no cloud upload, schedule, merge or encryption.

Restore backup opens a native picker, validates a private staged copy and shows its filename/item count before replacement. Confirming replaces the entire database after an automatic restart; save note drafts first, since in-memory drafts do not survive restart. The source backup stays unchanged. Supported older schemas migrate on the staged copy; unknown/newer schemas, unexpected schema objects, invalid constraints/foreign keys, oversized text, unsafe HTML or invalid image data are rejected.

Before replacement on startup, Someday saves the latest current database to a timestamped `backups/rescue-<timestamp>-<id>.someday-backup` under its app-data directory. A failed rescue write aborts replacement. The panel reports the result and rescue path. To undo a successful restore, choose that rescue file using Restore backup. Rescue files are retained until you remove them yourself and can contain private data. Shortcut conflicts and image-monitor startup failures use the existing Settings/tray recovery behavior.

Rust owns the dialogs and file operations on blocking workers. `create_backup`, `prepare_restore` and `restore_backup` are main-only commands; Angular receives an opaque temporary token, never arbitrary filesystem access. SQLite's backup API creates a consistent snapshot, including committed WAL data. Snapshot publication and startup replacement use synced temporary files plus Windows atomic replacement. A durable pending marker and staged-file hash recognize interruption after replacement without importing the same backup again. Startup restore runs before IPC storage work, shortcut registration and automatic image monitoring. No new database migration is introduced.


Backup verification (2026-10-31): 72 Angular tests and 64 Rust tests passed, including AXE structure checks in jsdom (contrast disabled), byte/format/settings round-trip, rowid preservation through old-schema migration, committed WAL snapshots, rejected schema/constraint/image corruption, stale tokens, rescue-write failure and interrupted-replacement recovery. Rustfmt and frontend/CSP checks passed. An isolated `com.someday.backup-qa` native run opened Settings and its owned Save As dialog without dismissing the panel; the Windows UI helper could not reliably control the filename input, so end-to-end picker/automatic-restart/tray behavior remains unverified. Actual disk exhaustion, full visual contrast checks and installer installation were not exercised. The existing app.css budget warning remains.

Backup UI token contrast was checked numerically in both themes: text contrast is at least 5.54:1, and control borders/focus accents are at least 3.28:1. This is a token-level check, not a full browser contrast audit. The running release executable was retained as `src-tauri/target/release/someday-running-before-backup-20261031.exe` so existing drafts were not closed during packaging; save drafts and open the newly built executable/installer to use this feature.

The startup restore itself also runs on a dedicated worker, joined before native listeners and storage IPC become available. A restore worker panic reports a safe recovery message rather than resetting the database.

Final backup-feature packaging passed: `npm run tauri:build` produced `src-tauri/target/release/bundle/nsis/Someday_0.1.0_x64-setup.exe` (3.64 MiB). The installer was built, not installed.
