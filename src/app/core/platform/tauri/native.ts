import { ColorTheme } from '../../color-theme';
import { Injectable } from '@angular/core';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

export type CaptureOutcome = 'saved' | 'duplicate' | 'empty_selection' | 'protected_field' |
  'focus_changed' | 'read_failed' | 'copy_failed' | 'too_large' | 'storage_unavailable';
export interface CaptureUpdate {
  sequence: number;
  outcome: CaptureOutcome;
  message: string;
  pendingCount: number | null;
}
export interface PreparedRestore { token: string; name: string; itemCount: number; }
export interface RestoreReport { message: string; rescuePath: string | null; }
export interface FoundationStatus {
  panelOnTop?: boolean;
  launchAtLogin: boolean | null;
  restoreReport?: RestoreReport;
  theme?: 'light' | 'dark' | null;
  colorPalette?: ColorTheme;
  autoSaveImages: boolean;
  autoSaveImagesAvailable: boolean;
  pendingCount: number;
  schemaVersion: number;
  shortcutAvailable: boolean;
  toggleShortcutAvailable: boolean;
  shortcuts: ShortcutSettings;
  captureResult: CaptureUpdate | null;
}
export interface ShortcutSettings { capture: string; toggle: string; }
export interface ImageMetadata {
  name: string;
  mimeType: string;
  width: number;
  height: number;
  byteSize: number;
}
interface ItemFields {
  pinned: boolean;
  id: string;
  createdAt: string;
  categoryId: string | null;
}
export type SavedItem = ItemFields & (
  { itemType: 'link' | 'text'; content: string; formattedHtml: string | null; image: null }
  | { itemType: 'image'; content: null; image: ImageMetadata }
);

export type ItemView = 'inbox' | 'archive';
export interface Category { id: string; name: string; }
export interface ItemSnapshot { items: SavedItem[]; categories: Category[]; pendingCount: number; }

@Injectable({ providedIn: 'root' })
export class Native {
  isFeedback(): boolean { return isTauri() && getCurrentWindow().label === 'feedback'; }
  listenCapture(callback: (update: CaptureUpdate) => void): Promise<UnlistenFn> {
    return listen<CaptureUpdate>('capture-result', event => callback(event.payload));
  }
  listenDraftFailure(callback: () => void): Promise<UnlistenFn> {
    return listen('note-draft-save-failed', callback);
  }
  listenPanelShown(callback: () => void): Promise<UnlistenFn> {
    return listen('panel-shown', callback);
  }
  createBackup(): Promise<boolean> { return invoke('create_backup'); }
  prepareRestore(): Promise<PreparedRestore | null> { return invoke('prepare_restore'); }
  restoreBackup(token: string): Promise<void> { return invoke('restore_backup', { token }); }
  status(): Promise<FoundationStatus> { return invoke('get_foundation_status'); }
  setLaunchAtLogin(enabled: boolean): Promise<boolean> { return invoke('set_launch_at_login', { enabled }); }
  setAutoSaveImages(enabled: boolean): Promise<boolean> { return invoke('set_auto_save_images', { enabled }); }
  setShortcuts(settings: ShortcutSettings): Promise<ShortcutSettings> { return invoke('set_shortcuts', { settings }); }
  listItems(view: ItemView): Promise<ItemSnapshot> { return invoke('list_items', { view }); }
  setTheme(light: boolean, palette?: ColorTheme): Promise<void> { return invoke('set_theme', { light, palette: palette ?? null }); }
  updateNote(id: string, content: string): Promise<void> { return invoke('update_note', { id, content }); }
  exportImage(id: string): Promise<boolean> { return invoke('export_image', { id }); }
  saveNote(content: string): Promise<['saved' | 'duplicate', number]> { return invoke('save_note', { content }); }
  loadNoteDraft(): Promise<{ content: string; revision: number }> { return invoke('load_note_draft'); }
  saveNoteDraft(content: string, revision: number): Promise<void> { return invoke('save_note_draft', { content, revision }); }
  setItemPinned(id: string, pinned: boolean): Promise<void> { return invoke('set_item_pinned', { id, pinned }); }
  importImage(): Promise<['saved' | 'duplicate', number] | null> { return invoke('import_image'); }
  pasteImage(): Promise<['saved' | 'duplicate', number]> { return invoke('paste_image'); }
  loadItemImage(id: string, variant: 'thumbnail' | 'original'): Promise<ArrayBuffer> { return invoke('load_item_image', { id, variant }); }
  openItem(id: string): Promise<void> { return invoke('open_item', { id }); }
  copyItem(id: string): Promise<void> { return invoke('copy_item', { id }); }
  deleteItem(id: string): Promise<void> { return invoke('delete_item', { id }); }
  setItemArchived(id: string, archived: boolean): Promise<void> { return invoke('set_item_archived', { id, archived }); }
  setItemCategory(id: string, categoryId: string | null): Promise<void> { return invoke('set_item_category', { id, categoryId }); }
  createCategory(name: string): Promise<Category> { return invoke('create_category', { name }); }
  renameCategory(id: string, name: string): Promise<void> { return invoke('rename_category', { id, name }); }
  deleteCategory(id: string): Promise<void> { return invoke('delete_category', { id }); }
  hide(): Promise<void> { return invoke('hide_panel'); }
  setPanelOnTop(enabled: boolean): Promise<void> { return invoke('set_panel_on_top', { enabled }); }
  dragPanel(): Promise<void> { return invoke('drag_panel'); }
}
