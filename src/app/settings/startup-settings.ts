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
  styleUrl: './toggle-settings.css',
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
