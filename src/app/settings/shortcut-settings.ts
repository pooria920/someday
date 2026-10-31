import { Component, computed, inject, input, linkedSignal, model, output, signal } from '@angular/core';
import { Native, ShortcutSettings } from '../core/platform/tauri/native';

@Component({
  selector: 'app-shortcut-settings',
  template: `
    <section aria-labelledby="shortcuts-title">
      <h3 id="shortcuts-title">Keyboard shortcuts</h3>
      <p id="shortcut-help">Choose a combination for each action. Shortcuts work while Someday is running.</p>
      @for (action of actions; track action.key) {
        <fieldset [disabled]="busy()" aria-describedby="shortcut-help">
          <legend>{{ action.label }}</legend>
          <div class="shortcut-controls">
            <label [for]="action.key + '-modifiers'">Modifiers
              <select #modifier [id]="action.key + '-modifiers'" [value]="modifiers(action.key)" (change)="change(action.key, modifier.value, key(action.key))">
                @for (value of modifierOptions; track value) { <option [value]="value" [selected]="modifiers(action.key) === value">{{ label(value) }}</option> }
              </select>
            </label>
            <label [for]="action.key + '-key'">Key
              <select #keySelect [id]="action.key + '-key'" [value]="key(action.key)" (change)="change(action.key, modifiers(action.key), keySelect.value)">
                @for (value of keys; track value) { <option [value]="value" [selected]="key(action.key) === value">{{ value }}</option> }
              </select>
            </label>
          </div>
        </fieldset>
      }
      @if (duplicate()) { <p role="alert" class="error">Choose a different shortcut for each action.</p> }
      @if (error()) { <p role="alert" class="error">{{ error() }}</p> }
      <p role="status" class="saved">{{ message() }}</p>
      <div class="actions">
        <button type="button" class="secondary" [disabled]="busy()" (click)="restoreDefaults()">Reset to defaults</button>
        <button type="button" class="primary" [disabled]="busy() || duplicate()" (click)="save()">{{ busy() ? 'Saving…' : 'Save changes' }}</button>
      </div>
    </section>
  `,
  styles: `
    :host { display: block; }
    h3 { font-size: 14px; margin: 20px 0 6px; }
    p { color: var(--muted); font-size: 12px; line-height: 1.65; margin: 0 0 16px; }
    fieldset { padding: 0; border: 0; margin: 18px 0; min-width: 0; }
    legend { color: var(--fg); font-size: 13px; font-weight: 600; margin-bottom: 8px; }
    .shortcut-controls { display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 1fr); gap: 10px; }
    label { display: grid; gap: 5px; font-size: 12px; color: var(--muted); }
    select { width: 100%; min-width: 0; height: 38px; border-radius: 8px; border: 1px solid var(--control-border); background: var(--surface); color: var(--fg); padding: 6px; font: inherit; cursor: pointer; }
    button { min-height: 36px; padding: 8px 10px; border-radius: 8px; font-size: 12px; cursor: pointer; }
    button:focus-visible, select:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
    .actions { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 8px; }
    .secondary { border: 1px solid var(--control-border); color: var(--fg); }
    .primary { background: var(--accent); color: var(--bg); }
    button:disabled, fieldset:disabled { opacity: .55; cursor: wait; }
    .error { color: var(--danger); }
    .saved { color: var(--accent); }
    .saved:empty { display: none; }
  `,
})
export class ShortcutSettingsPanel {
  private readonly native = inject(Native);
  readonly settings = input.required<ShortcutSettings>();
  readonly busy = model(false);
  readonly saved = output<ShortcutSettings>();
  protected readonly draft = linkedSignal(() => this.settings());
  protected readonly duplicate = computed(() => this.draft().capture === this.draft().toggle);
  protected readonly error = signal('');
  protected readonly message = signal('');
  protected readonly actions = [{ key: 'capture', label: 'Save selected text' }, { key: 'toggle', label: 'Show / hide panel' }] as const;
  protected readonly modifierOptions = ['Ctrl', 'Alt', 'Ctrl+Shift', 'Alt+Shift', 'Ctrl+Alt', 'Ctrl+Alt+Shift'];
  protected readonly keys = [...'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789', 'Space', ...Array.from({ length: 12 }, (_, i) => `F${i + 1}`)];

  protected modifiers(action: keyof ShortcutSettings) { return this.draft()[action].split('+').slice(0, -1).join('+'); }
  protected key(action: keyof ShortcutSettings) { return this.draft()[action].split('+').at(-1)!; }
  protected label(shortcut: string) { return shortcut.replaceAll('+', ' + '); }
  protected change(action: keyof ShortcutSettings, modifiers: string, key: string) {
    this.draft.update(settings => ({ ...settings, [action]: `${modifiers}+${key}` }));
    this.error.set(''); this.message.set('');
  }
  protected restoreDefaults() {
    this.draft.set({ capture: 'Ctrl+Shift+S', toggle: 'Ctrl+Shift+Space' });
    this.error.set(''); this.message.set('Defaults selected. Save changes to apply.');
  }
  protected async save() {
    if (this.busy() || this.duplicate()) { return; }
    this.busy.set(true); this.error.set(''); this.message.set('');
    try {
      const settings = await this.native.setShortcuts(this.draft());
      this.saved.emit(settings);
      this.message.set('Shortcuts saved.');
    } catch (error: unknown) {
      this.error.set(error === 'shortcut_unavailable' ? 'A shortcut is unavailable. Try another combination. Your previous shortcuts are unchanged.'
        : error === 'duplicate_shortcut' ? 'Choose a different shortcut for each action.'
        : error === 'invalid_shortcut' ? 'Choose a supported shortcut combination.'
        : 'Could not save shortcuts. Your previous shortcuts are unchanged. Please try again.');
    } finally { this.busy.set(false); }
  }
}
