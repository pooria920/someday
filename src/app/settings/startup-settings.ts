import { Component, inject, input, model, output, signal } from '@angular/core';
import { Native } from '../core/platform/tauri/native';

@Component({
  selector: 'app-startup-settings',
  template: `
    <section aria-labelledby="startup-title" [attr.aria-busy]="busy()">
      <h3 id="startup-title">Startup</h3>
      <label>
        <input #checkbox type="checkbox" [checked]="enabled() === true"
          [disabled]="busy() || enabled() === null" aria-describedby="startup-help"
          (change)="save(checkbox.checked); checkbox.checked = enabled() === true" />
        Launch Someday when I sign in to Windows
      </label>
      <p id="startup-help">Starts quietly in the system tray for your Windows account. Changes apply immediately. Windows Startup settings can also prevent automatic launch.</p>
      @if (enabled() === null) { <p role="alert">Startup settings are unavailable. Reopen Someday to try again.</p> }
      @if (error()) { <p role="alert">{{ error() }}</p> }
      <p role="status">{{ message() }}</p>
    </section>
  `,
  styles: `
    :host { display:block; }
    h3 { font-size:14px; margin:20px 0 10px; }
    label { display:flex; align-items:start; gap:8px; font-size:13px; color:var(--fg); cursor:pointer; }
    input { flex-shrink:0; width:16px; height:16px; accent-color:var(--accent); }
    input:focus-visible { outline:2px solid var(--accent); outline-offset:3px; }
    p { color:var(--muted); font-size:12px; line-height:1.65; margin-top:10px; }
    [role=alert] { color:var(--danger); }
    [role=status]:empty { display:none; }
  `,
})
export class StartupSettings {
  private readonly native = inject(Native);
  readonly enabled = input<boolean | null>(null);
  readonly busy = model(false);
  readonly saved = output<boolean>();
  protected readonly error = signal('');
  protected readonly message = signal('');

  protected async save(enabled: boolean) {
    if (this.busy() || this.enabled() === null) { return; }
    this.busy.set(true);
    this.error.set('');
    this.message.set('');
    try {
      const value = await this.native.setLaunchAtLogin(enabled);
      this.saved.emit(value);
      this.message.set(value ? 'Launch at sign-in enabled.' : 'Launch at sign-in disabled.');
    } catch {
      this.error.set('Could not change startup settings. Your previous setting is unchanged. Please try again.');
    } finally {
      this.busy.set(false);
    }
  }
}
