> Current approved extension (2026-10-31): item delete with confirmation, archive/restore, optional single-category assignment, category create/rename/delete and filtering in both Inbox and Archive are in scope and implemented. Settings now includes locally persisted custom capture and panel shortcuts with conflict handling and default restoration. Categories are optional; deleting one keeps its items uncategorized. This supersedes older phase/scope descriptions below for those features. Search, open/copy, sync, mobile and automatic organization are not part of this extension.

> Current capture flow: select text and press Ctrl + Shift + S; no manual copy is required. UI Automation is primary, with an approved automatic-copy fallback that replaces the clipboard. See README.md and FOUNDATION.md. The original brief below is retained for reference.

# Someday — A Minimal Desktop App for Saving Things for Later

You are a senior software engineer specializing in Angular, Tauri 2, Rust, Windows desktop applications, and UI/UX design.

Your task is to build **Someday**, a lightweight, local-first Windows desktop application.

**Tagline:** Save now. Deal with it someday.

## 1. Product Vision

Someday is a tiny desktop utility that helps users quickly save things they want to deal with later.

Throughout the day, people encounter interesting links, useful text, ideas, and information they don't have time to process immediately.

Someday lets them save these things instantly using a global keyboard shortcut without interrupting their workflow.

Later, users can open a small floating panel, review their saved items, and mark them as completed or delete them.

Think of Someday as a combination of:

- CopyQ's quick-access floating panel
- A minimal personal inbox
- A lightweight task queue

Someday is NOT a traditional to-do app, a clipboard history manager, or a complex bookmarking system.

**Core philosophy: Capture instantly. Review later.**

The application must be extremely simple, fast, lightweight, and unobtrusive.

## 2. Technology Stack

Use:

- Angular with standalone components
- Angular Signals for state management
- Tauri 2
- Rust for native functionality when necessary
- SQLite for local persistent storage
- Tailwind CSS v4
- TypeScript with strict typing
- Lucide icons

Use the latest stable compatible versions.

Do not use large UI component libraries such as Angular Material or Spartan.

Create a small custom design system.

Follow modern Angular best practices, including Signals, `inject()`, modern template control flow, and OnPush change detection.

## 3. Target Platform

The initial release targets **Windows 10 and Windows 11**.

Prioritize native desktop behavior and excellent Windows integration.

Keep the architecture reasonably portable to macOS and Linux, but do not sacrifice Windows functionality for premature cross-platform support.

Android and cloud synchronization are out of scope for the MVP.

## 4. Core User Flow

### Flow A — Save Something

1. User selects and copies a text or URL in any application.
2. User presses `Ctrl + Shift + S`.
3. Someday reads the current clipboard content.
4. Someday saves the content into SQLite.
5. A subtle confirmation appears.
6. The user continues working without opening the main application.

Saving should be nearly instantaneous.

The application must not steal focus during this operation.

The original clipboard content must remain unchanged.

Do not continuously monitor clipboard changes.

Someday must read the clipboard only when explicitly requested by the user.

### Flow B — Open Someday

1. User presses `Ctrl + Shift + Space`.
2. A small floating panel appears.
3. The panel displays pending saved items.
4. User can search, open, copy, complete, or delete items.
5. Pressing `Escape` closes the panel.

The panel must open quickly and feel like a native desktop utility.

### Flow C — Review Saved Items

Each item supports:

- View content
- Copy content
- Open URL in the default browser, if applicable
- Mark as completed
- Restore to pending
- Delete

No complex workflows.

No required tags, categories, projects, or due dates.

## 5. Floating Panel

The floating panel is the heart of Someday.

Its UX should resemble lightweight desktop utilities such as CopyQ, Raycast, and Spotlight.

### Window Behavior

- Small floating window
- Approximately 420px wide
- Compact height that adapts to content, within reasonable limits
- No traditional application title bar
- Rounded corners
- Subtle shadow
- Appears near the center of the screen
- Opens on the currently active monitor when possible
- Can be closed with Escape
- Should not appear in the taskbar as a normal application window
- Should not create multiple instances

Support closing the panel when clicking outside it, where reliable and practical.

Use appropriate Tauri window APIs.

Avoid relying on fragile hacks.

### Panel Layout

Header:

- Someday logo/name
- Number of pending items
- Settings icon

Search:

- Search input at the top
- Automatically focused when the panel opens
- Instantly filters saved items

Item List:

- Compact rows
- Content-type icon
- Item title or text preview
- Secondary information
- Save date
- Quick actions on hover or focus

Footer:

- Keyboard shortcut hints
- Minimal status information

### Empty State

When there are no saved items, display:

"Nothing waiting for you."

Include a short hint explaining how to save the first item.

## 6. Keyboard Shortcuts

Implement the following global shortcuts:

| Shortcut | Action |
|---|---|
| Ctrl + Shift + S | Save clipboard content |
| Ctrl + Shift + Space | Toggle Someday panel |

Inside the panel:

| Shortcut | Action |
|---|---|
| Escape | Close panel |
| Arrow Up / Down | Navigate items |
| Enter | Open selected item |
| Ctrl + C | Copy selected item when focus is not in an editable field |
| Delete | Delete selected item with appropriate safeguards |

Make global shortcuts configurable in a future-friendly way, but settings UI for remapping is not required in the MVP.

Handle shortcut registration failures gracefully, including conflicts with other applications.

## 7. Content Detection

Automatically detect basic clipboard content types.

### URLs

If the clipboard contains a valid HTTP or HTTPS URL:

- Save it as a link
- Display a link icon
- Show a clean URL preview
- Allow opening it in the default browser

### Plain Text

For all other text:

- Save it as a text item
- Generate a short preview
- Preserve the original content

### Duplicate Handling

Prevent accidental duplicate captures.

For example, if the user presses the save shortcut twice consecutively with identical clipboard content, do not create duplicate entries.

Use a simple, predictable duplicate-detection policy and document it.

Do not prevent users from intentionally saving the same content again later.

Image and file capture are not required in the MVP.

## 8. Data Model

Use SQLite.

Create a minimal `items` table with fields equivalent to:

```text
id
type
content
title
status
created_at
completed_at
```

Item types:

- link
- text

Item statuses:

- pending
- completed

Use appropriate database constraints and indexes.

Support database migrations.

Store the database in the appropriate Tauri application data directory.

Data must survive application restarts and updates.

## 9. Application Architecture

Separate responsibilities clearly.

Suggested structure:

```text
src/
  app/
    core/
      models/
      services/
      repositories/
    features/
      panel/
      items/
      settings/
    shared/
      components/
      utils/
    app.config.ts
    app.routes.ts

src-tauri/
  src/
    commands/
    shortcuts/
    database/
    lib.rs
```

Adapt this structure if a simpler organization is more suitable.

Architecture principles:

- Angular handles presentation and UI state.
- Rust/Tauri handles native integration.
- SQLite handles persistence.
- Native functionality must have a clear interface.
- Avoid excessive abstraction.
- Use Angular Signals for reactive state.
- Keep the application easy to maintain.

Implement database access using either a maintained Tauri SQLite plugin or Rust-side commands.

Choose one approach based on security, simplicity, and compatibility, and explain the choice.

Use minimal Tauri permissions and capabilities.

Do not expose unrestricted filesystem, shell, or SQL access unnecessarily.

## 10. Background Behavior

Someday should behave like a proper desktop utility.

Requirements:

- Runs in the background
- Provides a system tray icon
- Can open the panel from the tray
- Closing the panel does not terminate the application
- Tray menu includes Open Someday and Quit
- Keeps global shortcuts registered while running
- Does not continuously poll the clipboard

Optional:

- Launch at Windows startup, with user-controlled opt-in

Avoid unnecessary background processes, timers, and network requests.

## 11. UI Design

Create a visually polished but minimal interface.

Design direction:

- Modern Windows desktop utility
- Clean typography
- Compact spacing
- Rounded corners
- Subtle borders
- Minimal animations
- Excellent dark theme
- Light theme support
- Keyboard-first interactions

Use Tailwind CSS v4 and custom CSS variables for theme tokens.

Avoid:

- Large dashboards
- Sidebars
- Complex navigation
- Excessive gradients
- Decorative illustrations
- Unnecessary onboarding screens

This should look and feel like a small utility, not a SaaS dashboard.

## 12. Performance

Performance is a core product requirement.

Optimize for:

- Fast application startup
- Fast floating panel opening
- Minimal memory consumption
- Immediate search feedback
- Non-blocking database operations
- Reliable global shortcuts

Keep the panel initialized or hidden instead of repeatedly destroying and recreating it, if that improves responsiveness.

Measure performance where practical rather than claiming arbitrary performance guarantees.

## 13. Privacy and Security

Someday is local-first and privacy-focused.

Rules:

- No account required
- No cloud backend
- No analytics
- No telemetry
- No external AI APIs
- No clipboard monitoring without user action
- No unnecessary network requests

All saved data remains on the user's device.

Do not log clipboard content.

Handle sensitive data carefully.

## 14. Testing

Create tests for important behavior:

- URL detection
- Content normalization
- Duplicate handling
- Item creation
- Item completion
- Item deletion
- Search and filtering

Verify the following native behaviors on Windows where the environment permits:

- Shortcut registration
- Clipboard capture
- Floating window toggle
- Escape handling
- System tray behavior
- Persistence after restart

If the development environment cannot execute Windows-native behavior, explicitly report what remains unverified.

Never claim a native feature has been tested unless it was actually tested.

## 15. Project Documentation

Create:

### AGENTS.md

Document:

- Project purpose
- Technology stack
- Architecture
- Coding conventions
- Angular guidelines
- Tauri/Rust guidelines
- Database rules
- Testing requirements
- Scope boundaries

### README.md

Document:

- Prerequisites
- Installation
- Development commands
- Build commands
- Keyboard shortcuts
- Database location
- Application behavior

## 16. Implementation Plan

Implement the project incrementally.

### Phase 1 — Foundation

- Initialize Angular and Tauri 2
- Configure Tailwind CSS v4
- Create application architecture
- Configure SQLite
- Create initial migrations
- Build the basic floating panel
- Configure application window behavior
- Verify development and production builds

### Phase 2 — Capture

- Implement clipboard reading
- Implement URL/text detection
- Implement global save shortcut
- Implement SQLite persistence
- Add capture confirmation
- Add duplicate protection

### Phase 3 — Panel Interactions

- Implement item list
- Implement search
- Implement keyboard navigation
- Implement open/copy/complete/delete actions
- Implement panel toggle shortcut
- Implement Escape and focus behavior

### Phase 4 — Desktop Integration

- Implement system tray
- Implement background lifecycle
- Handle shortcut conflicts
- Test native Windows behavior
- Optimize window opening

### Phase 5 — Polish

- Refine visual design
- Implement light/dark theme
- Add useful empty states
- Add error handling
- Improve accessibility
- Complete tests and documentation

## 17. Start Implementation

First, inspect the existing repository.

If it is empty, initialize the project from scratch.

Before coding:

1. Check the latest stable compatible versions of Angular, Tauri 2, Tailwind CSS, and required Tauri plugins.
2. Identify Windows-specific technical constraints.
3. Decide on a simple, maintainable architecture.
4. Create a concise implementation plan.

Then begin implementation.

**Implement Phase 1 completely.**

Do not stop after creating a plan or scaffolding.

Ensure the application can be launched and built.

Do not implement future phases prematurely.

At the end, provide:

- Summary of implementation
- Files created or modified
- Architectural decisions
- Commands to run the application
- Test/build results
- Known limitations
- Next implementation phase

## Final Product Principle

Someday should feel almost invisible.

Users should not have to organize, categorize, or manage everything they save.

They should be able to capture something in one second, forget about it, and return to it later.

**Capture it. Forget it. Someday.**


## Formatted text and Markdown (approved scope, schema v6)

Explicit selection capture now checks UI Automation first, then performs one guarded Ctrl+C to obtain fresh Unicode text and optional Windows HTML Format from the same clipboard snapshot. This replaces the current clipboard even when direct selection reading succeeds. Password, focus, size, ownership and VS Code selection-metadata checks remain mandatory. A failed copy may retain a still-valid direct selection as plain text; old clipboard text is never used. There is no background text monitoring.

Schema v6 appends nullable formatted_html, with a 4 MiB UTF-8 limit and text/link-only constraint. Rust validates CF_HTML byte offsets and sanitizes its fragment using ammonia before storage. Allowed structure includes headings, emphasis, lists, quotations, tables and code. Only bounded bold/italic/underline/strikethrough inline styles survive; source colors, fonts, layout, images, scripts and external resources are removed. Original content, whitespace and line endings remain unchanged. Exact-text deduplication still includes the archive and never changes existing formatting.

Expanded text items prefer saved HTML, otherwise render Markdown (including existing items and manual notes). Raw Markdown HTML is displayed literally. DOMPurify sanitizes rendered output, limited inline styles become semantic elements, and normal Angular sanitization remains enabled. Links do not navigate, remote images do not load, and CSP stays unchanged. Show original text / Show formatted text is an in-memory per-item toggle. Plain links remain plain text. Persian paragraphs use automatic direction; code blocks use LTR and wide code/tables scroll locally.

Copy item looks up content by saved ID in Rust and writes Unicode text plus sanitized CF_HTML when saved formatting exists. Markdown-only items copy their original source. IPC command signatures and main-only permissions are unchanged; SavedItem adds formattedHtml (nullable; null for images). Clipboard operations remain serialized with image copy, image monitoring and selection capture.

File import for .md, RTF, rich-text editing, source-color/font fidelity and syntax highlighting are outside this feature. This section supersedes earlier plain-text-only rendering and copy-fallback descriptions; historical verification sections describe their original checkpoints.


> Approved extension (2026-10-31): local search in Inbox/Archive with category filters, explicit saved HTTP(S) link opening in the default browser and item keyboard navigation are implemented. See README.md for current behavior. This supersedes historical exclusions for search and opening links.


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
