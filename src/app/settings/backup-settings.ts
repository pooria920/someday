import { afterRenderEffect, Component, ElementRef, inject, model, signal, viewChild } from '@angular/core';
import { Native, PreparedRestore } from '../core/platform/tauri/native';

@Component({
  selector: 'app-backup-settings',
  template: `
    <section aria-labelledby="backup-title" [attr.aria-busy]="busy()">
      <h3 id="backup-title">Backup &amp; restore</h3>
      <p id="backup-help">Save all items, original images, categories and settings to a local file. Backups are not encrypted. Keep them somewhere private.</p>
      @if (prepared(); as backup) {
        <div role="group" aria-labelledby="restore-title" aria-describedby="restore-warning">
          <h4 id="restore-title">Replace all current data?</h4>
          <p><bdi>{{ backup.name }}</bdi> contains {{ backup.itemCount }} items.</p>
          <p id="restore-warning">All current data and settings will be replaced. A rescue backup will be kept locally. Someday will restart, and unsaved note drafts will be lost.</p>
          <div class="actions">
            <button #cancelButton type="button" [disabled]="busy()" (click)="cancel()">Cancel</button>
            <button type="button" class="destructive" [disabled]="busy()" (click)="restore()">Replace data and restart</button>
          </div>
        </div>
      } @else {
        <div class="actions">
          <button type="button" [disabled]="busy()" aria-describedby="backup-help" (click)="create()">Create backup</button>
          <button #restoreButton type="button" [disabled]="busy()" (click)="choose()">Restore backup…</button>
        </div>
      }
      @if (error()) { <p role="alert">{{ error() }}</p> }
      <p role="status">{{ message() }}</p>
    </section>
  `,
  styles: `:host{display:block} h3{font-size:14px;margin:20px 0 10px} h4{font-size:13px;margin:12px 0} p{color:var(--muted);font-size:12px;line-height:1.65;margin:10px 0;overflow-wrap:anywhere} .actions{display:flex;flex-wrap:wrap;gap:8px} button{font:inherit;font-size:12px;border:1px solid var(--control-border);border-radius:8px;padding:8px 12px;background:var(--surface);color:var(--fg);cursor:pointer} button:focus-visible{outline:2px solid var(--accent);outline-offset:3px} button:disabled{opacity:.55;cursor:wait} .destructive,[role=alert]{color:var(--danger)} [role=status]:empty{display:none}`,
})
export class BackupSettings {
  private readonly native = inject(Native);
  readonly busy = model(false);
  protected readonly prepared = signal<PreparedRestore | null>(null);
  protected readonly error = signal('');
  protected readonly message = signal('');
  private readonly cancelButton = viewChild<ElementRef<HTMLButtonElement>>('cancelButton');
  private readonly restoreButton = viewChild<ElementRef<HTMLButtonElement>>('restoreButton');
  private returnFocus = false;

  constructor() {
    afterRenderEffect(() => {
      if (this.prepared()) { this.cancelButton()?.nativeElement.focus(); }
      else if (this.returnFocus) { this.restoreButton()?.nativeElement.focus(); this.returnFocus = false; }
    });
  }
  protected cancel() { this.returnFocus = true; this.prepared.set(null); this.error.set(''); this.message.set(''); }
  protected async create() {
    if (this.busy()) { return; }
    this.busy.set(true); this.error.set(''); this.message.set('Creating backup…');
    try { this.message.set(await this.native.createBackup() ? 'Backup saved.' : ''); }
    catch (error) { this.message.set(''); this.error.set(error === 'backup_internal_path' ? 'Choose a location outside Someday’s app data folder.' : 'Could not save the backup. Check disk space and folder access. Your existing file was kept.'); }
    finally { this.busy.set(false); }
  }
  protected async choose() {
    if (this.busy()) { return; }
    this.busy.set(true); this.error.set(''); this.message.set('Opening and checking backup…');
    try { this.prepared.set(await this.native.prepareRestore()); this.message.set(''); }
    catch (error) { this.message.set(''); this.error.set(error === 'backup_newer_version' ? 'This backup needs a newer version of Someday.' : 'Could not read this backup. Choose a valid Someday backup. Your current data is unchanged.'); }
    finally { this.busy.set(false); }
  }
  protected async restore() {
    const prepared = this.prepared();
    if (this.busy() || !prepared) { return; }
    this.busy.set(true); this.error.set(''); this.message.set('Preparing to restart…');
    try { await this.native.restoreBackup(prepared.token); }
    catch { this.message.set(''); this.error.set('Could not prepare the restore. Your current data is unchanged. Choose the backup again and retry.'); this.busy.set(false); }
  }
}
