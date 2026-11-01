import axe from 'axe-core';
import { TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { NgIcon } from '@ng-icons/core';
import { App } from './app';
import { Native } from './core/platform/tauri/native';

describe('Someday foundation', () => {
  const defaultShortcuts = { capture: 'Ctrl+Shift+S', toggle: 'Ctrl+Shift+Space' };
  const native = { setTheme: vi.fn(), updateNote: vi.fn(), exportImage: vi.fn(), openItem: vi.fn(), setAutoSaveImages: vi.fn(), importImage: vi.fn(), pasteImage: vi.fn(), loadItemImage: vi.fn(), saveNote: vi.fn(), copyItem: vi.fn(), setShortcuts: vi.fn(), status: vi.fn(), hide: vi.fn(), isFeedback: vi.fn(), listenCapture: vi.fn(), listItems: vi.fn(), deleteItem: vi.fn(), setItemArchived: vi.fn(), setItemCategory: vi.fn(), createCategory: vi.fn(), renameCategory: vi.fn(), deleteCategory: vi.fn() };
  const savedItem = { id: 'one', itemType: 'text', formattedHtml: null, content: 'سلام\nSaved selection', image: null, createdAt: '2026-10-31T00:00:00Z', categoryId: null };
  const imageItem = { ...savedItem, id: 'image', itemType: 'image', content: null, image: { name: 'Screenshot.png', mimeType: 'image/png', width: 1280, height: 720, byteSize: 2048 } };
  const snapshot = (items: unknown[] = [], categories: unknown[] = [], pendingCount = items.length) => ({ items, categories, pendingCount });
  beforeEach(() => {
    native.status.mockReset().mockResolvedValue({ pendingCount: 0, schemaVersion: 1, shortcutAvailable: true, toggleShortcutAvailable: true, shortcuts: defaultShortcuts, captureResult: null });
    native.hide.mockReset().mockResolvedValue(undefined);
    native.isFeedback.mockReset().mockReturnValue(false);
    native.listenCapture.mockReset().mockResolvedValue(vi.fn());
    native.listItems.mockReset().mockResolvedValue(snapshot());
    for (const action of [native.setTheme, native.updateNote, native.exportImage, native.openItem, native.copyItem, native.deleteItem, native.setItemArchived, native.setItemCategory, native.renameCategory, native.deleteCategory]) { action.mockReset().mockResolvedValue(undefined); }
    native.saveNote.mockReset().mockResolvedValue(['saved', 1]);
    native.setAutoSaveImages.mockReset().mockImplementation(async (enabled: boolean) => enabled);
    native.importImage.mockReset().mockResolvedValue(['saved', 1]);
    native.pasteImage.mockReset().mockResolvedValue(['saved', 1]);
    native.loadItemImage.mockReset().mockResolvedValue(new ArrayBuffer(0));
    native.setShortcuts.mockReset().mockImplementation(async (settings) => settings);
    native.createCategory.mockReset().mockResolvedValue({ id: 'work', name: 'Work' });
    HTMLDialogElement.prototype.showModal = function () { this.open = true; };
    HTMLDialogElement.prototype.close = function () { this.open = false; };
    TestBed.configureTestingModule({ imports: [App], providers: [{ provide: Native, useValue: native }] });
  });

  it('searches Persian text, links and image names while keeping category filters', async () => {
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, content: 'مي‌رود كتاب', categoryId: 'work' }, { ...savedItem, id: 'link', itemType: 'link', content: 'https://example.com' }, imageItem], [{ id: 'work', name: 'Work' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const search = element.querySelector<HTMLInputElement>('input[type="search"]')!;
    const find = async (value: string) => { search.value = value; search.dispatchEvent(new Event('input')); await fixture.whenStable(); };
    await find('می رود کتاب'); expect(element.querySelectorAll('.item')).toHaveLength(1);
    await find('EXAMPLE'); expect(element.querySelectorAll('.item')).toHaveLength(1);
    element.querySelector<HTMLButtonElement>('[aria-label="Open link in default browser"]')!.click(); await fixture.whenStable();
    expect(native.openItem).toHaveBeenCalledWith('link');
    const openButton = element.querySelector<HTMLButtonElement>('[aria-label="Open link in default browser"]')!;
    expect(openButton.querySelector('svg')).not.toBeNull();
    expect(openButton.closest('.item-actions')).not.toBeNull();
    expect(openButton.classList.contains('icon-button')).toBe(true);
    expect(openButton.querySelector('ng-icon')?.getAttribute('aria-hidden')).toBe('true');
    native.openItem.mockRejectedValueOnce('open_failed');
    element.querySelector<HTMLButtonElement>('[aria-label="Open link in default browser"]')!.click(); await fixture.whenStable();
    expect(element.textContent).toContain('Could not open this link.');
    [...element.querySelectorAll<HTMLButtonElement>('.view-button')].find(button => button.textContent === 'Archive')!.click(); await fixture.whenStable();
    expect(native.listItems).toHaveBeenLastCalledWith('archive');
    expect(element.querySelectorAll('.item')).toHaveLength(1);
    await find('SCREENSHOT'); expect(element.querySelectorAll('.item')).toHaveLength(1);
    const filter = element.querySelector<HTMLSelectElement>('[aria-label="Filter by category"]')!;
    filter.value = 'work'; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
    expect(element.textContent).toContain('No matching items.');
  });

  it('navigates, expands and copies focused items without stealing editable input or selected text', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem, { ...savedItem, id: 'two' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const search = element.querySelector<HTMLInputElement>('input[type="search"]')!;
    const key = (target: HTMLElement, value: string, ctrlKey = false) => target.dispatchEvent(new KeyboardEvent('keydown', { key: value, ctrlKey, bubbles: true, cancelable: true }));
    search.focus(); key(search, 'c', true); expect(native.copyItem).not.toHaveBeenCalled();
    key(search, 'ArrowDown');
    const summaries = element.querySelectorAll<HTMLElement>('summary'); expect(document.activeElement).toBe(summaries[0]);
    key(summaries[0], 'ArrowDown'); expect(document.activeElement).toBe(summaries[1]);
    key(summaries[1], 'Enter'); await fixture.whenStable(); expect(element.querySelectorAll('details')[1].open).toBe(true);
    key(summaries[1], 'c', true); await fixture.whenStable(); expect(native.copyItem).toHaveBeenCalledWith('two');
  });

  it('keeps search and link controls accessible in both themes', async () => {
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, itemType: 'link', content: 'https://example.com' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    for (const theme of ['Use light theme', 'Use dark theme']) {
      const toggle = element.querySelector<HTMLButtonElement>(`[aria-label="${theme}"]`);
      toggle?.click(); await fixture.whenStable();
      const result = await axe.run(element, { rules: { 'color-contrast': { enabled: false } } });
      expect(result.violations).toEqual([]);
    }
    element.querySelector<HTMLButtonElement>('[aria-label="Edit text"]')!.click(); await fixture.whenStable();
    const editing = await axe.run(element, { rules: { 'color-contrast': { enabled: false } } });
    expect(editing.violations).toEqual([]);
  });

  it.each(['text', 'link'])('shows the %s preview only while collapsed', async (itemType) => {
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, itemType, content: 'https://example.com/' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const details = element.querySelector('details')!;
    const summary = element.querySelector<HTMLElement>('summary')!;
    expect(element.querySelector('.item-preview')?.textContent).toBe('https://example.com/');
    summary.focus();
    details.open = true; details.dispatchEvent(new Event('toggle')); await fixture.whenStable();
    expect(element.querySelector('.item-preview')).toBeNull();
    expect(details.textContent).toContain('https://example.com/');
    expect(document.activeElement).toBe(summary);
    details.open = false; details.dispatchEvent(new Event('toggle')); await fixture.whenStable();
    expect(element.querySelector('.item-preview')?.textContent).toBe('https://example.com/');
  });

  it('focuses and selects search with Ctrl+F and respects open dialogs', async () => {
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const search = element.querySelector<HTMLInputElement>('input[type="search"]')!;
    search.value = 'previous query'; search.dispatchEvent(new Event('input')); await fixture.whenStable();
    const key = new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, bubbles: true, cancelable: true });
    document.dispatchEvent(key); await fixture.whenStable();
    expect(document.activeElement).toBe(search); expect(key.defaultPrevented).toBe(true);
    expect(search.selectionStart).toBe(0); expect(search.selectionEnd).toBe(search.value.length);
    expect(search.closest('label')!.compareDocumentPosition(element.querySelector('.toolbar')!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    element.querySelector<HTMLButtonElement>('[aria-label="New note"]')!.click(); await fixture.whenStable();
    const note = element.querySelector<HTMLTextAreaElement>('textarea')!; note.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, cancelable: true }));
    expect(document.activeElement).toBe(note);
  });

  it('restores the saved theme and retains it when saving fails', async () => {
    native.status.mockResolvedValue({ pendingCount: 0, shortcutAvailable: true, toggleShortcutAvailable: true, shortcuts: defaultShortcuts, theme: 'light' });
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    expect(element.querySelector('.panel')!.classList.contains('light')).toBe(true);
    native.setTheme.mockRejectedValueOnce('storage_unavailable');
    element.querySelector<HTMLButtonElement>('[aria-label="Use dark theme"]')!.click(); await fixture.whenStable();
    expect(element.querySelector('.panel')!.classList.contains('light')).toBe(true);
    element.querySelector<HTMLButtonElement>('[aria-label="Use dark theme"]')!.click(); await fixture.whenStable();
    expect(native.setTheme).toHaveBeenLastCalledWith(false);
    expect(element.querySelector('.panel')!.classList.contains('light')).toBe(false);
  });

  it('edits inside the item, retains failed changes and saves by ID without a dialog', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('[aria-label="Edit text"]')!.click(); await fixture.whenStable();
    const editor = element.querySelector<HTMLTextAreaElement>('textarea')!;
    expect(editor.closest('.item')).not.toBeNull();
    expect(element.querySelector('details')!.open).toBe(true);
    expect(element.querySelector('dialog')!.open).toBe(false);
    expect(document.activeElement).toBe(editor);
    expect(editor.value).toBe(savedItem.content);
    editor.value = '  changed\r\ntext  '; editor.dispatchEvent(new Event('input')); await fixture.whenStable();
    native.updateNote.mockRejectedValueOnce('duplicate_content');
    element.querySelector('form')!.dispatchEvent(new Event('submit', {cancelable:true})); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(false);
    expect(editor.value).toContain('changed');
    expect(element.textContent).toContain('already saved');
    element.querySelector('form')!.dispatchEvent(new Event('submit', {cancelable:true})); await fixture.whenStable();
    expect(native.updateNote).toHaveBeenLastCalledWith(savedItem.id,editor.value);
    expect(element.querySelector('dialog')!.open).toBe(false);
    expect(element.querySelector('.inline-editor')).toBeNull();
    expect(document.activeElement).toBe(element.querySelector('[aria-label="Edit text"]'));
    expect(native.saveNote).not.toHaveBeenCalled();
  });

  it('keeps inline drafts when filtered out and cancels with Escape without hiding the panel', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('[aria-label="Edit text"]')!.click(); await fixture.whenStable();
    const editor = element.querySelector<HTMLTextAreaElement>('textarea')!;
    editor.value = 'unsaved changes'; editor.dispatchEvent(new Event('input')); await fixture.whenStable();
    const search = element.querySelector<HTMLInputElement>('input[type="search"]')!;
    search.value = 'no match'; search.dispatchEvent(new Event('input')); await fixture.whenStable();
    expect(element.querySelector('.inline-editor')).toBeNull();
    search.value = ''; search.dispatchEvent(new Event('input')); await fixture.whenStable();
    const restored = element.querySelector<HTMLTextAreaElement>('textarea')!;
    expect(restored.value).toBe('unsaved changes');
    restored.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })); await fixture.whenStable();
    expect(element.querySelector('.inline-editor')).toBeNull();
    expect(native.hide).not.toHaveBeenCalled();
    expect(native.updateNote).not.toHaveBeenCalled();
    element.querySelector<HTMLButtonElement>('[aria-label="Edit text"]')!.click(); await fixture.whenStable();
    expect(element.querySelector<HTMLTextAreaElement>('textarea')!.value).toBe(savedItem.content);
    element.querySelector<HTMLButtonElement>('.inline-editor .secondary')!.click(); await fixture.whenStable();
    expect(element.querySelector('.inline-editor')).toBeNull();
  });

  it('edits archived links inline and prevents duplicate saves while saving', async () => {
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, itemType: 'link', content: 'https://example.com' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelectorAll<HTMLButtonElement>('.view-button')[1].click(); await fixture.whenStable();
    element.querySelector<HTMLButtonElement>('[aria-label="Edit text"]')!.click(); await fixture.whenStable();
    const editor = element.querySelector<HTMLTextAreaElement>('textarea')!;
    editor.value = 'https://example.org'; editor.dispatchEvent(new Event('input')); await fixture.whenStable();
    let finish!: () => void;
    native.updateNote.mockImplementationOnce(() => new Promise<void>(resolve => { finish = resolve; }));
    const form = element.querySelector('.inline-editor')!;
    form.dispatchEvent(new Event('submit', { cancelable: true }));
    form.dispatchEvent(new Event('submit', { cancelable: true })); await fixture.whenStable();
    expect(native.updateNote).toHaveBeenCalledExactlyOnceWith(savedItem.id, 'https://example.org');
    expect(editor.disabled).toBe(true);
    expect(element.querySelector<HTMLButtonElement>('.inline-editor .secondary')!.disabled).toBe(true);
    finish(); await Promise.resolve(); await fixture.whenStable();
    expect(element.querySelector('.inline-editor')).toBeNull();
  });

  it('exports images by ID and distinguishes cancellation and failure', async () => {
    native.listItems.mockResolvedValue(snapshot([imageItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const button=element.querySelector<HTMLButtonElement>('[aria-label="Save image to file"]')!;
    native.exportImage.mockResolvedValueOnce(false); button.click(); await fixture.whenStable();
    expect(native.exportImage).toHaveBeenCalledWith(imageItem.id);
    expect(element.textContent).not.toContain('Image saved to file');
    native.exportImage.mockResolvedValueOnce(true); button.click(); await fixture.whenStable();
    expect(element.textContent).toContain('Image saved to file');
    native.exportImage.mockRejectedValueOnce('export_failed'); button.click(); await fixture.whenStable();
    expect(element.textContent).toContain('Could not save this image to file.');
  });

  it.each([true, false])('uses the system light preference (%s) and allows switching', async (matches) => {
    vi.stubGlobal('matchMedia', vi.fn().mockReturnValue({ matches }));
    try {
      const fixture = TestBed.createComponent(App);
      await fixture.whenStable();
      const panel: HTMLElement = fixture.nativeElement.querySelector('.panel');
      expect(panel.classList.contains('light')).toBe(matches);
      fixture.nativeElement.querySelector('[aria-label="' + (matches ? 'Use dark theme' : 'Use light theme') + '"]').click();
      await fixture.whenStable();
      expect(panel.classList.contains('light')).toBe(!matches);
    } finally { vi.unstubAllGlobals(); }
  });

  it('edits shortcuts in Settings, reports conflicts safely and updates all hints after saving', async () => {
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const settingsButton = element.querySelector<HTMLButtonElement>('[aria-label="Settings"]')!;
    settingsButton.focus(); settingsButton.click(); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    expect(document.activeElement?.textContent).toBe('Settings');
    const choose = async (id: string, value: string) => {
      const select = element.querySelector<HTMLSelectElement>(`#${id}`)!;
      select.value = value; select.dispatchEvent(new Event('change')); await fixture.whenStable();
    };
    await choose('capture-modifiers', 'Alt+Shift'); await choose('capture-key', 'F8');
    await choose('toggle-modifiers', 'Ctrl+Alt');
    const save = element.querySelector<HTMLButtonElement>('app-shortcut-settings .primary')!;
    native.setShortcuts.mockRejectedValueOnce('shortcut_unavailable');
    save.click(); await fixture.whenStable();
    expect(native.setShortcuts).toHaveBeenCalledWith({ capture: 'Alt+Shift+F8', toggle: 'Ctrl+Alt+Space' });
    expect(element.textContent).toContain('Your previous shortcuts are unchanged.');
    expect(element.querySelector<HTMLButtonElement>('[aria-label="Hide Someday"]')!.title).toBe('Show or hide: Ctrl + Shift + Space');
    save.click(); await fixture.whenStable();
    expect(element.textContent).toContain('Shortcuts saved.');
    expect(element.querySelector<HTMLButtonElement>('[aria-label="Hide Someday"]')!.title).toBe('Show or hide: Ctrl + Alt + Space');
    expect(element.querySelector('.phase-note')!.textContent).toContain('Alt + Shift + F8');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true })); await fixture.whenStable();
    expect(document.activeElement).toBe(settingsButton);
    expect(native.hide).not.toHaveBeenCalled();
  });

  it('loads saved shortcuts, rejects duplicates and applies defaults only when saved', async () => {
    native.status.mockResolvedValue({ pendingCount: 0, schemaVersion: 3, shortcutAvailable: true, toggleShortcutAvailable: true, shortcuts: { capture: 'Alt+F8', toggle: 'Alt+F9' }, captureResult: null });
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('[aria-label="Settings"]')!.click(); await fixture.whenStable();
    const key = element.querySelector<HTMLSelectElement>('#toggle-key')!;
    expect(key.value).toBe('F9');
    key.value = 'F8'; key.dispatchEvent(new Event('change')); await fixture.whenStable();
    const save = element.querySelector<HTMLButtonElement>('app-shortcut-settings .primary')!;
    expect(save.disabled).toBe(true); save.click();
    expect(native.setShortcuts).not.toHaveBeenCalled();
    expect(element.textContent).toContain('Choose a different shortcut for each action.');
    element.querySelector<HTMLButtonElement>('app-shortcut-settings .secondary')!.click(); await fixture.whenStable();
    expect(element.querySelector<HTMLSelectElement>('#capture-key')!.value).toBe('S');
    expect(native.setShortcuts).not.toHaveBeenCalled();
    save.click(); await fixture.whenStable();
    expect(native.setShortcuts).toHaveBeenCalledExactlyOnceWith(defaultShortcuts);
  });

  it('keeps Settings open and blocks repeat saves until native registration finishes', async () => {
    let finish!: (value: unknown) => void;
    native.setShortcuts.mockReturnValue(new Promise(resolve => finish = resolve));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('[aria-label="Settings"]')!.click(); await fixture.whenStable();
    const save = element.querySelector<HTMLButtonElement>('app-shortcut-settings .primary')!;
    save.click(); save.click(); await fixture.whenStable();
    expect(native.setShortcuts).toHaveBeenCalledOnce();
    expect(save.disabled).toBe(true);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true })); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    finish(defaultShortcuts);
    await vi.waitFor(() => expect(save.disabled).toBe(false));
    await fixture.whenStable();
    expect(save.disabled).toBe(false);
  });

  it('shows the empty panel and real storage readiness', async () => {
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(fixture.nativeElement.textContent).toContain('Nothing waiting for you.');
    expect(fixture.nativeElement.textContent).toContain('Local storage ready');
  });

  it.each(['file', 'clipboard'])('saves an image from %s into the unfiltered inbox', async source => {
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('.view-button:nth-child(2)')!.click(); await fixture.whenStable();
    const filter = element.querySelector<HTMLSelectElement>('[aria-label="Filter by category"]')!;
    filter.value = 'none'; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
    const add = element.querySelector<HTMLButtonElement>('[aria-label="Add image"]')!;
    add.focus(); add.click(); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    expect(document.activeElement?.textContent).toBe('Choose file');
    native.listItems.mockResolvedValue(snapshot([imageItem]));
    const buttons = [...element.querySelectorAll<HTMLButtonElement>('dialog .image-sources button')];
    buttons[source === 'file' ? 0 : 1].click(); await fixture.whenStable();
    expect(source === 'file' ? native.importImage : native.pasteImage).toHaveBeenCalledOnce();
    expect(element.querySelector('dialog')!.open).toBe(false);
    expect(element.querySelector('.view-button[aria-pressed="true"]')!.textContent).toBe('Inbox');
    expect(filter.value).toBe('all');
    expect(element.textContent).toContain('Screenshot.png');
    expect(element.textContent).toContain('1280 × 720 · 2 KiB');
    expect(element.textContent).toContain('Image saved to Someday');
    expect(document.activeElement).toBe(add);
  });

  it('leaves the current view and items alone when file selection is cancelled or an image is duplicate', async () => {
    native.importImage.mockResolvedValueOnce(null).mockResolvedValueOnce(['duplicate', 1]);
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('.view-button:nth-child(2)')!.click(); await fixture.whenStable();
    element.querySelector<HTMLButtonElement>('[aria-label="Add image"]')!.click(); await fixture.whenStable();
    const reads = native.listItems.mock.calls.length;
    const choose = element.querySelector<HTMLButtonElement>('dialog .image-sources button')!;
    choose.click(); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    expect(element.querySelector('[role="alert"]')).toBeNull();
    expect(native.listItems.mock.calls.length).toBe(reads);
    choose.click(); await fixture.whenStable();
    expect(element.textContent).toContain('This image is already saved in your inbox or archive.');
    expect(element.querySelector('.view-button[aria-pressed="true"]')!.textContent).toBe('Archive');
    expect(element.textContent).toContain(savedItem.content);
    expect(native.listItems.mock.calls.length).toBe(reads);
  });

  it.each([
    ['no_image', 'Copy an image or screenshot first'],
    ['unsupported_image', 'Choose a static PNG, JPEG or WebP'],
    ['invalid_image', 'This image could not be read'],
    ['image_too_large', 'This image is too large'],
    ['clipboard_unavailable', 'The clipboard is busy or unavailable'],
    ['private filesystem path', 'Could not save this image'],
  ])('reports a safe image error for %s and allows retry', async (error, message) => {
    native.pasteImage.mockRejectedValueOnce(error);
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('[aria-label="Add image"]')!.click(); await fixture.whenStable();
    const paste = element.querySelectorAll<HTMLButtonElement>('dialog .image-sources button')[1];
    paste.click(); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    expect(element.querySelector('[role="alert"]')!.textContent).toContain(message);
    expect(element.textContent).not.toContain('private filesystem path');
    expect(paste.disabled).toBe(false);
    paste.click(); await fixture.whenStable();
    expect(native.pasteImage).toHaveBeenCalledTimes(2);
    expect(element.querySelector('dialog')!.open).toBe(false);
  });

  it('blocks repeat image saves and Escape while an image operation is in progress', async () => {
    let finish!: (value: ['saved', number]) => void;
    native.pasteImage.mockReturnValueOnce(new Promise(resolve => { finish = resolve; }));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('[aria-label="Add image"]')!.click(); await fixture.whenStable();
    const paste = element.querySelectorAll<HTMLButtonElement>('dialog .image-sources button')[1];
    paste.click(); paste.click(); await fixture.whenStable();
    expect(native.pasteImage).toHaveBeenCalledOnce();
    expect(paste.disabled).toBe(true);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true })); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    expect(native.hide).not.toHaveBeenCalled();
    finish(['saved', 1]); await fixture.whenStable();
  });

  it('loads originals only when image details are opened, and reuses item copy/archive/delete controls', async () => {
    native.listItems.mockResolvedValue(snapshot([imageItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    expect(native.loadItemImage).toHaveBeenCalledExactlyOnceWith(imageItem.id, 'thumbnail');
    const details = element.querySelector('details')!;
    details.open = true; details.dispatchEvent(new Event('toggle')); await fixture.whenStable();
    expect(native.loadItemImage).toHaveBeenLastCalledWith(imageItem.id, 'original');
    expect(element.querySelector('app-saved-image.image-content')).not.toBeNull();
    details.open = false; details.dispatchEvent(new Event('toggle')); await fixture.whenStable();
    expect(element.querySelector('app-saved-image.image-content')).toBeNull();
    element.querySelector<HTMLButtonElement>('[aria-label="Copy item"]')!.click(); await fixture.whenStable();
    expect(native.copyItem).toHaveBeenCalledExactlyOnceWith(imageItem.id);
    element.querySelector<HTMLButtonElement>('[aria-label="Archive item"]')!.click(); await fixture.whenStable();
    expect(native.setItemArchived).toHaveBeenCalledExactlyOnceWith(imageItem.id, true);
    element.querySelector<HTMLButtonElement>('[aria-label="Delete item"]')!.click(); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    expect(native.deleteItem).not.toHaveBeenCalled();
    element.querySelector<HTMLButtonElement>('dialog .primary')!.click(); await fixture.whenStable();
    expect(native.deleteItem).toHaveBeenCalledExactlyOnceWith(imageItem.id);
  });

  it('keeps restored image disclosure and original loading synchronized after category filtering', async () => {
    const category = { id: 'photos', name: 'Photos' };
    native.listItems.mockResolvedValue(snapshot([{ ...imageItem, categoryId: category.id }], [category]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const details = element.querySelector('details')!;
    details.open = true; details.dispatchEvent(new Event('toggle')); await fixture.whenStable();
    const filter = element.querySelector<HTMLSelectElement>('[aria-label="Filter by category"]')!;
    filter.value = 'none'; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
    expect(element.querySelector('details')).toBeNull();
    filter.value = 'all'; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
    const restored = element.querySelector('details')!;
    expect(restored.open).toBe(true);
    expect(element.querySelector('app-saved-image.image-content')).not.toBeNull();
    restored.open = false; restored.dispatchEvent(new Event('toggle')); await fixture.whenStable();
    expect(element.querySelector('app-saved-image.image-content')).toBeNull();
  });

  it('keeps raw native errors out of the UI', async () => {
    native.status.mockRejectedValue('secret filesystem path');
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(fixture.nativeElement.textContent).toContain('Local storage could not be opened');
    expect(fixture.nativeElement.textContent).not.toContain('secret');
  });

  it('hides the panel on Escape', async () => {
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(native.hide).toHaveBeenCalledOnce();
  });

  it('updates the count after capture without opening the panel', async () => {
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const listener = native.listenCapture.mock.calls[0][0];
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    listener({ sequence: 1, outcome: 'saved', pendingCount: 1, message: 'Saved to Someday' });
    await fixture.whenStable();
    expect(fixture.nativeElement.textContent).toContain('Saved to Someday');
    expect(fixture.nativeElement.textContent).toContain(savedItem.content);
    expect(fixture.nativeElement.querySelector('[aria-label="Saved items"]')).not.toBeNull();
  });

  it('shows a shortcut conflict without hiding the panel', async () => {
    native.status.mockResolvedValue({ pendingCount: 0, schemaVersion: 1, shortcutAvailable: false, toggleShortcutAvailable: true, shortcuts: defaultShortcuts, captureResult: null });
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(fixture.nativeElement.textContent).toContain('could not be registered');
  });

  it('reports a window shortcut conflict independently of capture', async () => {
    native.status.mockResolvedValue({ pendingCount: 0, schemaVersion: 2, shortcutAvailable: true, toggleShortcutAvailable: false, shortcuts: defaultShortcuts, captureResult: null });
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(fixture.nativeElement.textContent).toContain('Ctrl + Shift + Space could not be registered.');
    expect(fixture.nativeElement.textContent).not.toContain('Ctrl + Shift + S could not be registered.');
    expect(fixture.nativeElement.textContent).toContain('open Someday from the tray');
  });

  it('does not overwrite a new capture with an older startup snapshot', async () => {
    let complete!: (value: unknown) => void;
    native.status.mockReturnValue(new Promise(resolve => complete = resolve));
    native.listItems.mockResolvedValue(snapshot([], [], 2));
    const fixture = TestBed.createComponent(App);
    await Promise.resolve();
    native.listenCapture.mock.calls[0][0]({ sequence: 2, outcome: 'saved', pendingCount: 2, message: 'Saved to Someday' });
    complete({ pendingCount: 0, schemaVersion: 1, shortcutAvailable: true, toggleShortcutAvailable: true, shortcuts: defaultShortcuts, captureResult: null });
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('[aria-label="Pending items"]').textContent).toContain('2');
  });

  it('cleans up the event subscription when destroyed', async () => {
    const unlisten = vi.fn();
    native.listenCapture.mockResolvedValue(unlisten);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    fixture.destroy();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('renders feedback without panel controls', async () => {
    native.isFeedback.mockReturnValue(true);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('button')).toBeNull();
    expect(fixture.nativeElement.querySelector('.feedback')).not.toBeNull();
    expect(native.listItems).not.toHaveBeenCalled();
  });

  it('renders Markdown on expansion and retains the original-text choice across collapse', async () => {
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, content: '# Heading\n\n**Bold**' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const details = element.querySelector('details')!;
    expect(element.querySelector('.text-formatted')).toBeNull();
    const expand = async (open: boolean) => {
      details.open = open; details.dispatchEvent(new Event('toggle')); await fixture.whenStable();
    };
    await expand(true);
    expect(element.querySelector('.text-formatted strong')?.textContent).toBe('Bold');
    element.querySelector<HTMLButtonElement>('.text-view-toggle')!.click(); await fixture.whenStable();
    await expand(false); await expand(true);
    expect(element.querySelector('.text-original')?.textContent).toBe('# Heading\n\n**Bold**');
  });

  it('loads existing items on startup and renders their content as text', async () => {
    const content = '<script>not executable</script>\nسلام';
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, content }]));
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    fixture.nativeElement.querySelector('.text-view-toggle').click();
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('.item-content').textContent).toBe(content);
    expect(fixture.nativeElement.querySelector('.items script')).toBeNull();
    expect(fixture.nativeElement.textContent).not.toContain('Nothing waiting for you.');
  });

  it('isolates direction for each saved item and renders Iconsax icons', async () => {
    native.listItems.mockResolvedValue(snapshot([
      savedItem,
      { ...savedItem, id: 'two', itemType: 'link', content: 'https://example.com/سلام' },
    ]));
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    for (const selector of ['summary', '.item-content']) {
      const blocks = element.querySelectorAll(selector);
      expect(blocks).toHaveLength(2);
      for (const block of blocks) { expect(block.getAttribute('dir')).toBe('auto'); }
    }
    const icons = () => fixture.debugElement.queryAll(By.directive(NgIcon))
      .map(node => node.injector.get(NgIcon).name());
    expect(icons()).toContain('saxDocumentTextOutline');
    expect(icons()).toContain('saxLink1Outline');
    expect(element.querySelectorAll('.item summary .item-type svg')).toHaveLength(2);
    expect([...element.querySelectorAll('.item-meta > span')].map(node => node.textContent)).toEqual(['Text', 'Link']);
    expect(element.querySelectorAll('summary time[datetime]')).toHaveLength(2);
    expect(element.querySelectorAll('.category-chip select[aria-label="Item category"]')).toHaveLength(2);
    element.querySelector<HTMLButtonElement>('[aria-label="Use light theme"]')!.click();
    await fixture.whenStable();
    expect(icons()).toContain('saxMoonOutline');
  });

  it('copies an item from inbox and archive without changing saved items', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const element = fixture.nativeElement as HTMLElement;
    for (const view of ['inbox', 'archive']) {
      if (view === 'archive') {
        element.querySelectorAll<HTMLButtonElement>('.view-button')[1].click();
        await fixture.whenStable();
      }
      const reads = native.listItems.mock.calls.length;
      element.querySelector<HTMLButtonElement>('[aria-label="Copy item"]')!.click();
      await fixture.whenStable();
      expect(native.copyItem).toHaveBeenLastCalledWith(savedItem.id);
      expect(element.querySelector('[role="status"]')!.textContent).toContain('Copied to clipboard');
      expect(element.querySelector('details')!.open).toBe(false);
      expect(native.listItems.mock.calls.length).toBe(reads);
      expect(element.textContent).toContain(savedItem.content);
    }
    expect(native.deleteItem).not.toHaveBeenCalled();
    expect(native.setItemArchived).not.toHaveBeenCalled();
  });

  it('disables duplicate copy clicks, reports safe errors and allows retry', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    let rejectCopy!: (reason: unknown) => void;
    native.copyItem.mockReturnValueOnce(new Promise<void>((_, reject) => { rejectCopy = reject; }));
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const element = fixture.nativeElement as HTMLElement;
    const button = element.querySelector<HTMLButtonElement>('[aria-label="Copy item"]')!;
    button.click();
    fixture.detectChanges();
    expect(button.disabled).toBe(true);
    button.click();
    expect(native.copyItem).toHaveBeenCalledOnce();
    rejectCopy('private native error');
    await Promise.resolve();
    await fixture.whenStable();
    expect(button.disabled).toBe(false);
    expect(element.textContent).toContain('Could not copy this item. Please try again.');
    expect(element.textContent).not.toContain('private native error');
    expect(element.textContent).not.toContain('Copied to clipboard');
    button.click();
    await fixture.whenStable();
    expect(element.textContent).toContain('Copied to clipboard');
    expect(element.textContent).not.toContain('Could not copy this item');
  });

  it('saves a multiline note, switches to the unfiltered inbox and restores focus', async () => {
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const element = fixture.nativeElement as HTMLElement;
    element.querySelectorAll<HTMLButtonElement>('.view-button')[1].click();
    await fixture.whenStable();
    const filter = element.querySelector<HTMLSelectElement>('[aria-label="Filter by category"]')!;
    filter.value = 'none'; filter.dispatchEvent(new Event('change'));
    const open = element.querySelector<HTMLButtonElement>('[aria-label="New note"]')!;
    open.focus(); open.click(); await fixture.whenStable();
    const textarea = element.querySelector<HTMLTextAreaElement>('#note-content')!;
    expect(document.activeElement).toBe(textarea);
    const save = element.querySelector<HTMLButtonElement>('dialog [type="submit"]')!;
    expect(save.disabled).toBe(true);
    textarea.value = ' \n\t'; textarea.dispatchEvent(new Event('input')); await fixture.whenStable();
    expect(save.disabled).toBe(true);
    const content = '  سلام\nMy new note  ';
    textarea.value = content; textarea.dispatchEvent(new Event('input')); await fixture.whenStable();
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, content }]));
    element.querySelector('dialog form')!.dispatchEvent(new Event('submit', { cancelable: true }));
    await fixture.whenStable();
    expect(native.saveNote).toHaveBeenCalledExactlyOnceWith(content);
    expect(native.listItems).toHaveBeenLastCalledWith('inbox');
    expect(filter.value).toBe('all');
    expect(element.querySelector('dialog')!.open).toBe(false);
    expect(element.textContent).toContain(content);
    expect(element.textContent).toContain('Note saved to Someday');
    expect(document.activeElement).toBe(open);
    open.click(); await fixture.whenStable();
    expect(element.querySelector<HTMLTextAreaElement>('#note-content')!.value).toBe('');
  });

  it('retains a note after duplicate or storage errors and while hiding the panel', async () => {
    native.saveNote.mockResolvedValueOnce(['duplicate', 1]).mockRejectedValueOnce('private filesystem path');
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element = fixture.nativeElement as HTMLElement;
    const open = element.querySelector<HTMLButtonElement>('[aria-label="New note"]')!;
    open.click(); await fixture.whenStable();
    const textarea = element.querySelector<HTMLTextAreaElement>('#note-content')!;
    textarea.value = 'Keep this draft'; textarea.dispatchEvent(new Event('input')); await fixture.whenStable();
    for (const message of ['This text is already saved', 'Could not save your changes']) {
      element.querySelector('dialog form')!.dispatchEvent(new Event('submit', { cancelable: true }));
      await fixture.whenStable();
      expect(element.querySelector('dialog')!.open).toBe(true);
      expect(textarea.value).toBe('Keep this draft');
      expect(element.textContent).toContain(message);
      expect(element.textContent).not.toContain('private filesystem');
    }
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
    await fixture.whenStable();
    expect(native.hide).not.toHaveBeenCalled();
    open.click(); await fixture.whenStable();
    expect(element.querySelector<HTMLTextAreaElement>('#note-content')!.value).toBe('Keep this draft');
  });

  it('keeps the draft and prevents duplicate submits while a note is saving', async () => {
    let complete!: (result: ['saved', number]) => void;
    native.saveNote.mockReturnValueOnce(new Promise<['saved', number]>(resolve => { complete = resolve; }));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element = fixture.nativeElement as HTMLElement;
    element.querySelector<HTMLButtonElement>('[aria-label="New note"]')!.click(); await fixture.whenStable();
    const textarea = element.querySelector<HTMLTextAreaElement>('#note-content')!;
    textarea.value = 'Pending'; textarea.dispatchEvent(new Event('input')); await fixture.whenStable();
    const form = element.querySelector('dialog form')!;
    form.dispatchEvent(new Event('submit', { cancelable: true }));
    fixture.detectChanges();
    expect(textarea.disabled).toBe(true);
    form.dispatchEvent(new Event('submit', { cancelable: true }));
    expect(native.saveNote).toHaveBeenCalledOnce();
    complete(['saved', 1]); await Promise.resolve(); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(false);
  });

  it('shows a safe list error and allows retry', async () => {
    native.listItems.mockRejectedValueOnce('private filesystem path');
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(fixture.nativeElement.textContent).toContain('Could not load your saved items');
    expect(fixture.nativeElement.textContent).not.toContain('private filesystem');
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    fixture.nativeElement.querySelector('.retry').click();
    await fixture.whenStable();
    expect(fixture.nativeElement.textContent).toContain(savedItem.content);
    expect(fixture.nativeElement.querySelector('.retry')).toBeNull();
  });

  it('keeps a newer capture list when an older request finishes late', async () => {
    let finish!: (value: unknown) => void;
    native.listItems.mockImplementationOnce(() => new Promise(resolve => finish = resolve));
    const fixture = TestBed.createComponent(App);
    await vi.waitFor(() => expect(native.listItems).toHaveBeenCalledOnce());
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    native.listenCapture.mock.calls[0][0]({ sequence: 1, outcome: 'saved', pendingCount: 1, message: 'Saved to Someday' });
    finish(snapshot());
    await fixture.whenStable();
    expect(fixture.nativeElement.textContent).toContain(savedItem.content);
  });

  it('confirms deletion, focuses Cancel and restores focus after Escape', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const remove = element.querySelector<HTMLButtonElement>('[aria-label="Delete item"]')!;
    expect(remove.closest('details')).toBeNull();
    remove.focus(); remove.click(); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    expect(document.activeElement?.textContent).toBe('Cancel');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
    await fixture.whenStable();
    expect(native.hide).not.toHaveBeenCalled();
    expect(native.deleteItem).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(remove);
    remove.click(); await fixture.whenStable();
    native.listItems.mockResolvedValue(snapshot());
    [...element.querySelectorAll<HTMLButtonElement>('dialog button')].find(button => button.textContent === 'Delete permanently')!.click();
    await fixture.whenStable();
    expect(native.deleteItem).toHaveBeenCalledExactlyOnceWith('one');
    expect(element.querySelector('.item')).toBeNull();
    expect(element.querySelector('dialog')!.open).toBe(false);
    expect(document.activeElement?.textContent).toBe('Inbox');
  });

  it('archives, restores and keeps the global count when filtering', async () => {
    const work = { id: 'work', name: 'Work' };
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, categoryId: 'work' }], [work], 4));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const filter = element.querySelector<HTMLSelectElement>('[aria-label="Filter by category"]')!;
    filter.value = 'none'; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
    expect(element.querySelector('.item')).toBeNull();
    expect(element.querySelector('[aria-label="Pending items"]')!.textContent).toContain('4');
    filter.value = 'all'; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
    native.listItems.mockResolvedValue(snapshot([], [work], 3));
    element.querySelector<HTMLButtonElement>('[aria-label="Archive item"]')!.click(); await fixture.whenStable();
    expect(native.setItemArchived).toHaveBeenCalledWith('one', true);
    native.listItems.mockResolvedValue(snapshot([savedItem], [work], 3));
    [...element.querySelectorAll<HTMLButtonElement>('.view-button')].find(button => button.textContent === 'Archive')!.click(); await fixture.whenStable();
    expect(native.listItems).toHaveBeenLastCalledWith('archive');
    element.querySelector<HTMLButtonElement>('[aria-label="Restore to inbox"]')!.click(); await fixture.whenStable();
    expect(native.setItemArchived).toHaveBeenLastCalledWith('one', false);
  });

  it('filters by category in both views and can show all items again', async () => {
    const categories = [{ id: 'work', name: 'Work' }, { id: 'personal', name: 'Personal' }, { id: 'empty', name: 'Empty' }];
    const items = [savedItem, { ...savedItem, id: 'two', categoryId: 'work' }, { ...savedItem, id: 'three', categoryId: 'personal' }];
    native.listItems.mockResolvedValue(snapshot(items, categories));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const filter = element.querySelector<HTMLSelectElement>('[aria-label="Filter by category"]')!;
    const renderedCategories = () => [...element.querySelectorAll<HTMLSelectElement>('[aria-label="Item category"]')].map(select => select.value);
    for (const view of ['Inbox', 'Archive']) {
      [...element.querySelectorAll<HTMLButtonElement>('.view-button')].find(button => button.textContent === view)!.click();
      await fixture.whenStable();
      for (const [category, expected] of [['work', ['work']], ['personal', ['personal']], ['none', ['']], ['empty', []], ['all', ['', 'work', 'personal']]] as const) {
        const requests = native.listItems.mock.calls.length;
        filter.value = category; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
        expect(renderedCategories()).toEqual(expected);
        expect(element.textContent?.includes('No items in this category.')).toBe(category === 'empty');
        expect(native.listItems).toHaveBeenCalledTimes(requests);
      }
      filter.value = 'work'; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
    }
    expect(filter.value).toBe('work');
    expect(renderedCategories()).toEqual(['work']);
  });

  it('keeps the category filter when switching views and resets it after category deletion', async () => {
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, categoryId: 'work' }], [{ id: 'work', name: 'Work' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const filter = element.querySelector<HTMLSelectElement>('[aria-label="Filter by category"]')!;
    filter.value = 'work'; filter.dispatchEvent(new Event('change')); await fixture.whenStable();
    [...element.querySelectorAll<HTMLButtonElement>('.view-button')].find(button => button.textContent === 'Archive')!.click();
    await fixture.whenStable();
    expect(filter.value).toBe('work');
    element.querySelector<HTMLButtonElement>('[aria-label="Manage categories"]')!.click(); await fixture.whenStable();
    element.querySelector<HTMLButtonElement>('[aria-label="Delete category Work"]')!.click(); await fixture.whenStable();
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    [...element.querySelectorAll<HTMLButtonElement>('dialog button')].find(button => button.textContent === 'Delete category')!.click();
    await fixture.whenStable();
    expect(filter.value).toBe('all');
    expect(element.querySelectorAll('.item')).toHaveLength(1);
    expect(native.deleteItem).not.toHaveBeenCalled();
  });

  it('keeps items on action failure and blocks repeat requests while pending', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    let reject!: (error: unknown) => void;
    native.setItemArchived.mockReturnValue(new Promise((_, fail) => reject = fail));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const button = element.querySelector<HTMLButtonElement>('[aria-label="Archive item"]')!;
    button.click(); button.click(); fixture.detectChanges();
    expect(button.disabled).toBe(true);
    expect(native.setItemArchived).toHaveBeenCalledOnce();
    reject('private path');
    await vi.waitFor(() => expect(element.textContent).toContain('Could not update this item'));
    await fixture.whenStable();
    expect(element.textContent).toContain('Could not update this item');
    expect(element.textContent).not.toContain('private path');
    expect(element.querySelector('.item')).not.toBeNull();
  });

  it('selects a category and creates a category for an item', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem], [{ id: 'work', name: 'Work' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const select = element.querySelector<HTMLSelectElement>('[aria-label="Item category"]')!;
    select.value = 'work'; select.dispatchEvent(new Event('change')); await fixture.whenStable();
    expect(native.setItemCategory).toHaveBeenCalledWith('one', 'work');
    select.value = 'new'; select.dispatchEvent(new Event('change')); await fixture.whenStable();
    const input = element.querySelector<HTMLInputElement>('#category-name')!;
    input.value = ' New category '; input.dispatchEvent(new Event('input')); fixture.detectChanges();
    element.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true })); await fixture.whenStable();
    expect(native.createCategory).toHaveBeenCalledWith('New category');
    expect(native.setItemCategory).toHaveBeenLastCalledWith('one', 'work');
  });

  it('renames and deletes categories without deleting items', async () => {
    const work = { id: 'work', name: 'Work' };
    native.listItems.mockResolvedValue(snapshot([savedItem], [work]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('[aria-label="Manage categories"]')!.click(); await fixture.whenStable();
    element.querySelector<HTMLButtonElement>('.category-list .secondary')!.click(); await fixture.whenStable();
    const input = element.querySelector<HTMLInputElement>('#category-name')!;
    input.value = ' Personal '; input.dispatchEvent(new Event('input')); fixture.detectChanges();
    element.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true })); await fixture.whenStable();
    expect(native.renameCategory).toHaveBeenCalledWith('work', 'Personal');
    element.querySelector<HTMLButtonElement>('[aria-label="Delete category Work"]')!.click(); await fixture.whenStable();
    expect(native.deleteCategory).not.toHaveBeenCalled();
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    [...element.querySelectorAll<HTMLButtonElement>('dialog button')].find(button => button.textContent === 'Delete category')!.click(); await fixture.whenStable();
    expect(native.deleteCategory).toHaveBeenCalledWith('work');
    expect(native.deleteItem).not.toHaveBeenCalled();
    expect(element.querySelector('.item')).not.toBeNull();
  });

  it('discards an old view response after a capture during mutation', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    let finish!: () => void;
    native.setItemArchived.mockReturnValue(new Promise<void>(resolve => finish = resolve));
    element.querySelector<HTMLButtonElement>('[aria-label="Archive item"]')!.click();
    native.listenCapture.mock.calls[0][0]({ sequence: 2, outcome: 'saved', pendingCount: 2, message: 'Saved to Someday' });
    let oldView!: (value: unknown) => void;
    native.listItems.mockImplementationOnce(() => new Promise(resolve => oldView = resolve));
    finish();
    await vi.waitFor(() => expect(native.listItems).toHaveBeenCalledTimes(2));
    native.listItems.mockResolvedValue(snapshot([{ ...savedItem, id: 'archived', content: 'Archived content' }], [], 1));
    [...element.querySelectorAll<HTMLButtonElement>('.view-button')].find(button => button.textContent === 'Archive')!.click();
    oldView(snapshot([savedItem], [], 99)); await fixture.whenStable();
    expect(element.textContent).toContain('Archived content');
    expect(element.querySelector('[aria-label="Pending items"]')!.textContent).toContain('1');
  });

  it('reports duplicate names and retries assignment without creating duplicate categories', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const select = element.querySelector<HTMLSelectElement>('[aria-label="Item category"]')!;
    select.value = 'new'; select.dispatchEvent(new Event('change')); await fixture.whenStable();
    const input = element.querySelector<HTMLInputElement>('#category-name')!;
    input.value = 'Work'; input.dispatchEvent(new Event('input')); fixture.detectChanges();
    native.createCategory.mockRejectedValueOnce('duplicate_name');
    element.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true })); await fixture.whenStable();
    expect(element.textContent).toContain('A category with this name already exists.');
    native.setItemCategory.mockRejectedValueOnce('storage_unavailable');
    element.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true })); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(true);
    element.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true })); await fixture.whenStable();
    expect(native.createCategory).toHaveBeenCalledTimes(2);
    expect(native.setItemCategory).toHaveBeenCalledTimes(2);
    expect(element.querySelector('dialog')!.open).toBe(false);
  });


  it('keeps confirmation locked until its refreshed snapshot finishes', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    element.querySelector<HTMLButtonElement>('[aria-label="Delete item"]')!.click(); await fixture.whenStable();
    let finish!: (value: unknown) => void;
    native.listItems.mockImplementationOnce(() => new Promise(resolve => finish = resolve));
    const confirm = [...element.querySelectorAll<HTMLButtonElement>('dialog button')].find(button => button.textContent === 'Delete permanently')!;
    confirm.click();
    await vi.waitFor(() => expect(native.listItems).toHaveBeenCalledTimes(2));
    fixture.detectChanges();
    expect(confirm.disabled).toBe(true);
    confirm.click();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
    expect(element.querySelector('dialog')!.open).toBe(true);
    expect(native.deleteItem).toHaveBeenCalledOnce();
    expect(native.hide).not.toHaveBeenCalled();
    finish(snapshot()); await fixture.whenStable();
    expect(element.querySelector('dialog')!.open).toBe(false);
  });

  it('renaming from the item category dialog does not assign the category', async () => {
    native.listItems.mockResolvedValue(snapshot([savedItem], [{ id: 'work', name: 'Work' }]));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const select = element.querySelector<HTMLSelectElement>('[aria-label="Item category"]')!;
    select.value = 'new'; select.dispatchEvent(new Event('change')); await fixture.whenStable();
    element.querySelector<HTMLButtonElement>('.category-list .secondary')!.click(); await fixture.whenStable();
    const input = element.querySelector<HTMLInputElement>('#category-name')!;
    input.value = 'Personal'; input.dispatchEvent(new Event('input')); fixture.detectChanges();
    element.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true })); await fixture.whenStable();
    expect(native.renameCategory).toHaveBeenCalledWith('work', 'Personal');
    expect(native.setItemCategory).not.toHaveBeenCalled();
    expect(element.querySelector('dialog')!.open).toBe(true);
  });

  it('refreshes a background capture deferred while the image setting is saving', async () => {
    let finish!: (enabled: boolean) => void;
    native.setAutoSaveImages.mockReturnValueOnce(new Promise<boolean>(resolve => { finish = resolve; }));
    const fixture = TestBed.createComponent(App); await fixture.whenStable();
    fixture.nativeElement.querySelector('[aria-label="Settings"]').click(); await fixture.whenStable();
    fixture.nativeElement.querySelector('app-image-capture-settings input').click(); await fixture.whenStable();
    const reads = native.listItems.mock.calls.length;
    native.listItems.mockResolvedValue(snapshot([imageItem]));
    native.listenCapture.mock.calls[0][0]({ sequence: 3, outcome: 'saved', message: 'Image saved automatically', pendingCount: 1 });
    await fixture.whenStable(); expect(native.listItems.mock.calls.length).toBe(reads);
    finish(true); await Promise.resolve(); await fixture.whenStable();
    expect(native.listItems.mock.calls.length).toBeGreaterThan(reads);
    expect(fixture.nativeElement.querySelector('.item-preview').textContent).toContain('Screenshot.png');
  });

});
