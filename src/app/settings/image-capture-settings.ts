import { Component, inject, input, model, output, signal } from '@angular/core';
import { Native } from '../core/platform/tauri/native';

@Component({
  selector: 'app-image-capture-settings',
  template: `
    <section aria-labelledby="auto-images-title">
      <h3 id="auto-images-title">Images</h3>
      <label><input #checkbox type="checkbox" [checked]="enabled()" [disabled]="busy()" aria-describedby="auto-images-help" (change)="save(checkbox.checked); checkbox.checked = enabled()" /> Automatically save copied images and screenshots</label>
      <p id="auto-images-help">While Someday is running, new images on your clipboard are saved locally. Screenshots must be copied to the clipboard. This can include private images. Changes apply immediately.</p>
      @if (!available()) { <p role="alert">Automatic saving could not start. Turn it off and on to retry.</p> }
      @if (error()) { <p role="alert">{{ error() }}</p> }
      <p role="status">{{ message() }}</p>
    </section>
  `,
  styles: `:host { display:block; } h3 { font-size:14px; margin:20px 0 10px; } label { display:flex; align-items:start; gap:8px; font-size:13px; color:var(--fg); } input { flex-shrink:0; width:16px; height:16px; accent-color:var(--accent); } input:focus-visible { outline:2px solid var(--accent); outline-offset:3px; } p { color:var(--muted); font-size:12px; line-height:1.65; margin-top:10px; } [role=alert] { color:var(--danger); } [role=status]:empty { display:none; }`,
})
export class ImageCaptureSettings {
  private readonly native = inject(Native);
  readonly enabled = input(false);
  readonly available = input(true);
  readonly busy = model(false);
  readonly saved = output<boolean>();
  protected readonly error = signal('');
  protected readonly message = signal('');
  protected async save(enabled: boolean) {
    if (this.busy()) { return; }
    this.busy.set(true); this.error.set(''); this.message.set('');
    try {
      const value = await this.native.setAutoSaveImages(enabled);
      this.saved.emit(value);
      this.message.set(value ? 'Automatic image saving enabled.' : 'Automatic image saving disabled.');
    } catch {
      this.error.set('Could not change automatic image saving. Your previous setting is unchanged. Please try again.');
    } finally { this.busy.set(false); }
  }
}
