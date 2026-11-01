import { BackupSettings } from './settings/backup-settings';
import { ImageCaptureSettings } from './settings/image-capture-settings';
import { afterRenderEffect, Component, computed, DestroyRef, ElementRef, inject, signal, untracked, viewChild } from '@angular/core';
import { DatePipe } from '@angular/common';
import { NgIcon, provideIcons } from '@ng-icons/core';
import { saxEdit2Outline, saxDocumentDownloadOutline, saxExport3Outline, saxImageOutline, saxCopyOutline, saxDirectInboxOutline, saxCloseCircleOutline, saxTickCircleOutline, saxMoonOutline, saxSun1Outline, saxLink1Outline, saxDocumentTextOutline, saxTrashOutline, saxArchiveOutline, saxUndoOutline, saxFolderOutline, saxArrowDown1Outline } from '@ng-icons/iconsax/outline';
import { CaptureUpdate, Category, ItemView, Native, SavedItem, ShortcutSettings } from './core/platform/tauri/native';
import { saxSetting2Outline } from '@ng-icons/iconsax/outline';
import { ShortcutSettingsPanel } from './settings/shortcut-settings';
import { SavedText } from './items/saved-text/saved-text';
import { SavedImage } from './items/saved-image/saved-image';

type DialogState = { kind: 'edit'; item: SavedItem } | { kind: 'delete'; item: SavedItem } | { kind: 'categories'; itemId: string | null } | { kind: 'settings' } | { kind: 'note' } | { kind: 'image' };

@Component({
  selector: 'app-root',
  imports: [BackupSettings, ImageCaptureSettings, NgIcon, DatePipe, ShortcutSettingsPanel, SavedImage, SavedText],
  providers: [provideIcons({ saxEdit2Outline, saxDocumentDownloadOutline, saxExport3Outline, saxImageOutline, saxSetting2Outline, saxCopyOutline, saxDirectInboxOutline, saxCloseCircleOutline, saxTickCircleOutline, saxMoonOutline, saxSun1Outline, saxLink1Outline, saxDocumentTextOutline, saxTrashOutline, saxArchiveOutline, saxUndoOutline, saxFolderOutline, saxArrowDown1Outline })],
  templateUrl: './app.html',
  styleUrl: './app.css',
  host: { '(document:keydown.escape)': 'onEscape($event)', '(document:keydown)': 'onItemKey($event)' },
})
export class App {
  private readonly native = inject(Native);
  private readonly destroyRef = inject(DestroyRef);
  private unlisten?: () => void;
  private sequence = 0;
  private listRequest = 0;
  private refreshPending = false;
  protected readonly items = signal<SavedItem[]>([]);
  protected readonly itemsLoaded = signal(false);
  protected readonly itemsError = signal('');
  protected readonly feedbackSurface = this.native.isFeedback();
  protected readonly captureMessage = signal('');
  protected readonly shortcutAvailable = signal(true);
  protected readonly toggleShortcutAvailable = signal(true);
  protected readonly autoSaveImages = signal(false);
  protected readonly autoSaveImagesAvailable = signal(true);
  protected readonly shortcuts = signal<ShortcutSettings>({ capture: 'Ctrl+Shift+S', toggle: 'Ctrl+Shift+Space' });
  protected shortcutLabel(shortcut: string) { return shortcut.replaceAll('+', ' + '); }
  protected readonly restoreReport = signal<{ message: string; rescuePath: string | null } | null>(null);
  protected readonly ready = signal(false);
  protected readonly pendingCount = signal(0);
  protected readonly error = signal('');
  protected readonly light = signal(globalThis.matchMedia?.('(prefers-color-scheme: light)').matches ?? false);

  protected readonly view = signal<ItemView>('inbox');
  protected readonly categories = signal<Category[]>([]);
  protected readonly categoryFilter = signal('all');
  protected readonly search = signal('');
  private searchText(value: string) {
    return value.normalize('NFKC').toLocaleLowerCase().replace(/ي/g, 'ی').replace(/ك/g, 'ک').replace(/[\u200c\u200d\u0640\u064b-\u065f]/g, '').replace(/\s+/g, '');
  }
  protected readonly visibleItems = computed(() => this.items().filter(item =>
    (this.categoryFilter() === 'all' || (item.categoryId ?? 'none') === this.categoryFilter()) &&
    this.searchText(item.itemType === 'image' ? item.image.name : item.content).includes(this.searchText(this.search()))));
  protected readonly busyItems = signal<ReadonlySet<string>>(new Set());
  protected readonly actionError = signal('');
  protected readonly copyingItem = signal<string | null>(null);
  protected readonly expandedItems = signal<ReadonlySet<string>>(new Set());
  protected readonly dialogState = signal<DialogState | null>(null);
  protected readonly dialogBusy = signal(false);
  protected readonly dialogError = signal('');
  protected readonly categoryName = signal('');
  protected readonly themeBusy = signal(false);
  protected readonly exportingImage = signal<string | null>(null);
  protected readonly editContent = signal('');
  protected readonly editorContent = computed(() => this.dialogState()?.kind === 'edit' ? this.editContent() : this.noteContent());
  protected setEditorContent(value: string) { if (this.dialogState()?.kind === 'edit') { this.editContent.set(value); } else { this.noteContent.set(value); } }
  protected openEdit(item: SavedItem) { if (item.itemType === 'image') { return; } this.editContent.set(item.content); this.openDialog({ kind: 'edit', item }); }
  protected async toggleTheme() {
    if (this.themeBusy()) { return; }
    this.themeBusy.set(true); this.actionError.set('');
    const light = !this.light();
    try { await this.native.setTheme(light); this.light.set(light); }
    catch { this.actionError.set('Could not save your theme. Please try again.'); }
    finally { this.themeBusy.set(false); }
  }
  protected async exportImage(item: SavedItem) {
    if (this.exportingImage()) { return; }
    this.exportingImage.set(item.id); this.actionError.set('');
    try { if (await this.native.exportImage(item.id)) { this.captureMessage.set('Image saved to file'); } }
    catch { this.actionError.set('Could not save this image to file. Please try again.'); }
    finally { this.exportingImage.set(null); }
  }
  protected readonly noteContent = signal('');
  protected readonly editingCategory = signal<string | null>(null);
  private createdCategoryId: string | null = null;
  protected readonly deletingCategory = signal<Category | null>(null);
  private readonly searchInput = viewChild<ElementRef<HTMLInputElement>>('searchInput');
  private readonly dialog = viewChild<ElementRef<HTMLDialogElement>>('dialog');
  private returnFocus: HTMLElement | null = null;

  constructor() {
    afterRenderEffect(() => {
      const state = this.dialogState();
      this.deletingCategory();
      this.editingCategory();
      this.dialogBusy();
      this.busyItems();
      if (!this.dialogBusy() && !this.busyItems().size && this.refreshPending) {
        untracked(() => { void this.refreshItems(); });
      }
      const dialog = this.dialog()?.nativeElement;
      if (state && dialog && !dialog.open) {
        dialog.showModal();
        dialog.querySelector<HTMLElement>('[data-initial-focus]')?.focus();
      } else if (state && dialog?.open && !dialog.contains(document.activeElement)) {
        dialog.querySelector<HTMLElement>('[data-initial-focus]')?.focus();
      } else if (!state && this.returnFocus) {
        const target = this.returnFocus;
        this.returnFocus = null;
        if (target.isConnected) { target.focus(); }
        else { document.querySelector<HTMLElement>('.view-button[aria-pressed="true"]')?.focus(); }
      }
    });
    this.destroyRef.onDestroy(() => this.unlisten?.());
    void this.initialize();
  }

  private async initialize() {
    try {
      const unlisten = await this.native.listenCapture(update => this.applyCapture(update));
      if (this.destroyRef.destroyed) { unlisten(); return; }
      this.unlisten = unlisten;
      const status = await this.native.status();
      if (this.destroyRef.destroyed) { return; }
      if (this.sequence <= (status.captureResult?.sequence ?? 0)) {
        this.pendingCount.set(status.pendingCount);
        if (status.captureResult) { this.applyCapture(status.captureResult, false); }
      }
      this.shortcutAvailable.set(status.shortcutAvailable);
      this.toggleShortcutAvailable.set(status.toggleShortcutAvailable);
      this.shortcuts.set(status.shortcuts);
      this.autoSaveImages.set(status.autoSaveImages ?? false);
      this.autoSaveImagesAvailable.set(status.autoSaveImagesAvailable ?? true);
      if (status.theme === 'light' || status.theme === 'dark') { this.light.set(status.theme === 'light'); }
      this.restoreReport.set(status.restoreReport ?? null);
      this.ready.set(true);
      if (!this.feedbackSurface) { await this.refreshItems(); }
    }
    catch { this.error.set('Local storage could not be opened. Please restart Someday.'); }
  }

  private applyCapture(update: CaptureUpdate, refresh = true) {
    if (this.destroyRef.destroyed || update.sequence <= this.sequence) { return; }
    this.sequence = update.sequence;
    if (update.pendingCount !== null) { this.pendingCount.set(update.pendingCount); }
    this.captureMessage.set(update.message);
    if (refresh && !this.feedbackSurface && (update.outcome === 'saved' || update.outcome === 'duplicate')) {
      void this.refreshItems();
    }
  }

  protected async refreshItems(afterDialogAction = false) {
    if (this.busyItems().size || (this.dialogBusy() && !afterDialogAction)) { this.refreshPending = true; return; }
    this.refreshPending = false;
    const request = ++this.listRequest;
    this.itemsError.set('');
    try {
      const snapshot = await this.native.listItems(this.view());
      if (this.destroyRef.destroyed || request !== this.listRequest) { return; }
      this.items.set(snapshot.items);
      this.expandedItems.update(ids => new Set([...ids].filter(id => snapshot.items.some(item => item.id === id))));
      this.categories.set(snapshot.categories);
      this.pendingCount.set(snapshot.pendingCount);
      if (!['all', 'none', ...snapshot.categories.map(category => category.id)].includes(this.categoryFilter())) {
        this.categoryFilter.set('all');
      }
      this.itemsLoaded.set(true);
    } catch {
      if (!this.destroyRef.destroyed && request === this.listRequest) {
        this.itemsError.set('Could not load your saved items. Please try again.');
      }
    }
  }

  protected switchView(view: ItemView) {
    if (view === this.view()) { return; }
    this.view.set(view);
    this.items.set([]);
    this.expandedItems.set(new Set());
    this.itemsLoaded.set(false);
    ++this.listRequest;
    void this.refreshItems();
  }

  protected categoryLabel(id: string | null): string {
    return this.categories().find(category => category.id === id)?.name ?? 'Uncategorized';
  }

  protected async changeCategory(item: SavedItem, value: string) {
    if (value === 'new') { this.openCategories(item.id); return; }
    await this.performItem(item.id, () => this.native.setItemCategory(item.id, value || null));
  }

  protected async copy(item: SavedItem) {
    if (this.copyingItem() !== null) { return; }
    this.copyingItem.set(item.id);
    this.actionError.set('');
    this.captureMessage.set('');
    try {
      await this.native.copyItem(item.id);
      this.captureMessage.set('Copied to clipboard');
    } catch {
      this.actionError.set('Could not copy this item. Please try again.');
    } finally {
      this.copyingItem.set(null);
    }
  }

  protected async openLink(item: SavedItem) {
    this.actionError.set('');
    try { await this.native.openItem(item.id); }
    catch { this.actionError.set('Could not open this link. Please try again.'); }
  }

  protected onItemKey(event: KeyboardEvent) {
    if (this.feedbackSurface || this.dialogState() || event.defaultPrevented || event.isComposing || event.altKey || event.metaKey) { return; }
    if (event.ctrlKey && !event.shiftKey && event.key.toLowerCase() === 'f') {
      const input = this.searchInput()?.nativeElement;
      if (input) { event.preventDefault(); input.focus(); input.select(); }
      return;
    }
    const target = event.target instanceof HTMLElement ? event.target : null;
    const search = target?.matches('input[type="search"]');
    if (target?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])') && !(search && event.key === 'ArrowDown')) { return; }
    const summaries = [...document.querySelectorAll<HTMLElement>('.items summary')];
    const index = summaries.findIndex(summary => summary === document.activeElement);
    if (!event.ctrlKey && ['ArrowDown', 'ArrowUp'].includes(event.key)) {
      event.preventDefault();
      summaries[Math.max(0, Math.min(summaries.length - 1, index < 0 ? 0 : index + (event.key === 'ArrowDown' ? 1 : -1)))]?.focus();
    } else if (!event.ctrlKey && !event.shiftKey && event.key === 'Enter' && index >= 0) {
      event.preventDefault();
      const item = this.visibleItems()[index];
      if (item) { this.itemExpanded(item, !this.expandedItems().has(item.id)); }
    } else if (event.ctrlKey && !event.shiftKey && event.key.toLowerCase() === 'c' && index >= 0 && !window.getSelection()?.toString()) {
      const item = this.visibleItems()[index];
      if (item && !this.busyItems().has(item.id)) { event.preventDefault(); void this.copy(item); }
    }
  }

  protected async archive(item: SavedItem) {
    await this.performItem(item.id, () => this.native.setItemArchived(item.id, this.view() === 'inbox'));
  }

  private async performItem(id: string, action: () => Promise<void>) {
    if (this.busyItems().has(id)) { return; }
    ++this.listRequest;
    this.busyItems.update(ids => new Set([...ids, id]));
    this.actionError.set('');
    try { await action(); }
    catch { this.actionError.set('Could not update this item. Please try again.'); }
    finally {
      this.busyItems.update(ids => new Set([...ids].filter(value => value !== id)));
      if (!this.destroyRef.destroyed) { await this.refreshItems(); }
    }
  }

  private openDialog(state: DialogState) {
    this.returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    this.dialogError.set('');
    this.categoryName.set('');
    this.editingCategory.set(null);
    this.createdCategoryId = null;
    this.deletingCategory.set(null);
    this.dialogState.set(state);
  }

  protected openDelete(item: SavedItem) { this.openDialog({ kind: 'delete', item }); }
  protected openNote() { this.openDialog({ kind: 'note' }); }
  protected openImage() { this.openDialog({ kind: 'image' }); }

  protected itemExpanded(item: SavedItem, open: boolean) {
    this.expandedItems.update(ids => {
      const next = new Set(ids);
      if (open) { next.add(item.id); } else { next.delete(item.id); }
      return next;
    });
  }

  protected imageSize(bytes: number): string {
    return bytes < 1_048_576 ? `${Math.max(1, Math.round(bytes / 1024))} KiB` : `${(bytes / 1_048_576).toFixed(1)} MiB`;
  }

  protected async saveImage(source: 'file' | 'clipboard') {
    if (this.dialogBusy() || this.busyItems().size) { return; }
    this.dialogBusy.set(true);
    this.dialogError.set('');
    let saved = false;
    try {
      const result = await (source === 'file' ? this.native.importImage() : this.native.pasteImage());
      if (!result || this.destroyRef.destroyed) { return; }
      const [outcome, count] = result;
      if (outcome === 'duplicate') {
        this.dialogError.set('This image is already saved in your inbox or archive.');
        return;
      }
      this.pendingCount.set(count);
      this.view.set('inbox');
      this.categoryFilter.set('all');
      this.search.set('');
      this.captureMessage.set('Image saved to Someday');
      saved = true;
      await this.refreshItems(true);
    } catch (error: unknown) {
      this.dialogError.set(error === 'image_too_large' ? 'This image is too large. Use an image up to 20 MiB, 40 megapixels and 16,384 pixels per side.'
        : error === 'unsupported_image' ? 'Choose a static PNG, JPEG or WebP image.'
        : error === 'invalid_image' ? 'This image could not be read. Try another image.'
        : error === 'no_image' ? 'Copy an image or screenshot first, then try again.'
        : error === 'clipboard_unavailable' ? 'The clipboard is busy or unavailable. Please try again.'
        : 'Could not save this image. Please try again.');
    } finally {
      this.dialogBusy.set(false);
      if (!this.destroyRef.destroyed && saved) { this.closeDialog(); }
    }
  }

  protected async saveNote(event: Event) {
    event.preventDefault();
    const content = this.editorContent();
    const state = this.dialogState();
    if (state?.kind === 'edit') {
      if (!content.trim() || content.length > 1_048_576) { return; }
      await this.dialogAction(() => this.native.updateNote(state.item.id, content), true);
      return;
    }
    if (!content.trim() || content.length > 1_048_576) { return; }
    await this.dialogAction(async () => {
      const [outcome, count] = await this.native.saveNote(content);
      if (outcome === 'duplicate') { throw 'duplicate_content'; }
      this.pendingCount.set(count);
      this.view.set('inbox');
      this.categoryFilter.set('all');
      this.search.set('');
      this.captureMessage.set('Note saved to Someday');
      this.noteContent.set('');
    }, true);
  }

  protected openSettings() { this.openDialog({ kind: 'settings' }); }
  protected shortcutsSaved(settings: ShortcutSettings) {
    this.shortcuts.set(settings);
    this.shortcutAvailable.set(true);
    this.toggleShortcutAvailable.set(true);
  }
  protected openCategories(itemId: string | null = null) { this.openDialog({ kind: 'categories', itemId }); }

  protected closeDialog() {
    if (this.dialogBusy()) { return; }
    this.dialog()?.nativeElement.close();
    this.dialogState.set(null);
  }

  protected cancelDialog(event: Event) { event.preventDefault(); this.closeDialog(); }

  protected onEscape(event: Event) {
    if (this.dialogState()) { event.preventDefault(); this.closeDialog(); }
    else if (!event.defaultPrevented) { void this.hide(); }
  }

  protected async confirmDelete() {
    const state = this.dialogState();
    if (state?.kind !== 'delete' || this.dialogBusy()) { return; }
    await this.dialogAction(async () => this.native.deleteItem(state.item.id), true);
  }

  protected editCategory(category: Category) {
    this.editingCategory.set(category.id);
    this.createdCategoryId = null;
    this.categoryName.set(category.name);
    this.deletingCategory.set(null);
    this.dialogError.set('');
  }

  protected async saveCategory(event: Event) {
    event.preventDefault();
    const name = this.categoryName().trim();
    if (!name) { this.dialogError.set('Enter a category name.'); return; }
    const state = this.dialogState();
    if (state?.kind !== 'categories') { return; }
    const editing = this.editingCategory();
    await this.dialogAction(async () => {
      if (editing) { await this.native.renameCategory(editing, name); }
      else {
        let id = this.createdCategoryId;
        if (id) { await this.native.renameCategory(id, name); }
        else {
          const category = await this.native.createCategory(name);
          id = category.id;
          // Retain the new ID if assignment fails, so retry does not create a duplicate.
          this.createdCategoryId = id;
        }
        if (state.itemId) { await this.native.setItemCategory(state.itemId, id); }
      }
      this.editingCategory.set(null);
      this.createdCategoryId = null;
      this.categoryName.set('');
    }, !!state.itemId && !editing);
  }

  protected async selectCreatedCategory(category: Category) {
    const state = this.dialogState();
    if (state?.kind !== 'categories' || !state.itemId) { return; }
    await this.dialogAction(() => this.native.setItemCategory(state.itemId!, category.id), true);
  }

  protected async confirmCategoryDelete() {
    const category = this.deletingCategory();
    if (!category) { return; }
    await this.dialogAction(async () => {
      await this.native.deleteCategory(category.id);
      this.deletingCategory.set(null);
      this.editingCategory.set(null);
      this.categoryName.set('');
    });
  }

  private async dialogAction(action: () => Promise<unknown>, close = false) {
    if (this.dialogBusy() || this.busyItems().size) { return; }
    ++this.listRequest;
    this.dialogBusy.set(true);
    this.dialogError.set('');
    let succeeded = false;
    try { await action(); succeeded = true; }
    catch (error: unknown) {
      this.dialogError.set(error === 'duplicate_name' ? 'A category with this name already exists.'
        : error === 'duplicate_content' ? 'This text is already saved in your inbox or archive.'
        : error === 'too_large' ? 'This note is too large.'
        : error === 'empty_selection' ? 'Write a note first.'
        : error === 'invalid_name' ? 'Enter a category name.'
        : 'Could not save your changes. Please try again.');
    } finally {
      if (!this.destroyRef.destroyed) { await this.refreshItems(true); }
      this.dialogBusy.set(false);
      if (!this.destroyRef.destroyed && succeeded && close) { this.closeDialog(); }
    }
  }

  protected async dragPanel(event: MouseEvent) {
    if (event.button !== 0 || event.detail > 1 || !(event.target instanceof Element)
      || event.target.closest('button')) { return; }
    event.preventDefault();
    try { await this.native.dragPanel(); }
    catch { this.error.set('The panel could not be moved. Please try again.'); }
  }

  protected async hide() {
    try { await this.native.hide(); }
    catch { this.error.set('The panel could not be hidden. Please try again.'); }
  }
}
