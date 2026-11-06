import { afterNextRender, Component, DestroyRef, ElementRef, inject, Injector, signal } from '@angular/core';
import { DueReminder, Native } from '../core/platform/tauri/native';
import { NgIcon, provideIcons } from '@ng-icons/core';
import { saxNotificationOutline, saxClockOutline, saxExport3Outline, saxTickCircleOutline } from '@ng-icons/iconsax/outline';
import { persianNumber, persianDate } from './persian-calendar';

@Component({
  selector: 'app-reminder-window',
  imports: [NgIcon],
  providers: [provideIcons({ saxNotificationOutline, saxClockOutline, saxExport3Outline, saxTickCircleOutline })],
  template: `
    <main class="panel" [class.light]="light" dir="rtl" lang="fa" aria-labelledby="reminder-heading">
      <header class="toast-header">
        <span class="reminder-mark"><ng-icon name="saxNotificationOutline" size="20" aria-hidden="true" /></span>
        <h1 id="reminder-heading">یادآورها</h1>
        @if (items().length > 1) { <span class="reminder-count" aria-label="تعداد یادآورها">{{ number(items().length) }}</span> }
        <span class="toast-brand"><bdi dir="ltr" lang="en">someday.</bdi></span>
      </header>
      <div class="reminder-list" aria-live="polite">
        @for (item of items(); track item.id) {
          <article class="reminder-card" [attr.aria-busy]="busy() === item.id">
            <p class="preview" dir="auto">{{ item.preview }}</p>
            <p class="due-date"><ng-icon name="saxClockOutline" size="14" aria-hidden="true" /><span>{{ format(item.dueAt) }}</span></p>
            <div class="toast-actions">
              <button type="button" class="primary-action" [disabled]="busy() !== null" (click)="act(item, 'open')">باز کردن آیتم<ng-icon name="saxExport3Outline" size="16" aria-hidden="true" /></button>
              <button type="button" class="secondary-action" [disabled]="busy() !== null" (click)="act(item, 'snooze')">۱۰ دقیقه بعد</button>
              <button type="button" class="text-action acknowledge-action" [disabled]="busy() !== null" (click)="act(item, 'acknowledge')"><ng-icon name="saxTickCircleOutline" size="16" aria-hidden="true" />تأیید</button>
            </div>
          </article>
        } @empty { <p>یادآوری برای رسیدگی نیست.</p> }
        @if (error()) { <p class="error" role="alert">{{ error() }} <button type="button" [disabled]="busy() !== null" (click)="refresh()">تلاش مجدد</button></p> }
      </div>
    </main>
  `,
  styleUrls: ['./reminders.css', './reminder-window.css'],
})
export class ReminderWindow {
  private readonly native = inject(Native);
  private readonly destroyRef = inject(DestroyRef);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly injector = inject(Injector);
  readonly items = signal<DueReminder[]>([]);
  readonly error = signal('');
  readonly busy = signal<string | null>(null);
  readonly light = globalThis.matchMedia?.('(prefers-color-scheme: light)').matches ?? false;
  readonly format = persianDate;
  readonly number = persianNumber;
  private revision = 0;

  constructor() { void this.initialize(); }
  private async initialize() {
    try {
      const unlisten = await this.native.listenDueReminders(items => {
        ++this.revision;
        this.items.set(items);
      });
      if (this.destroyRef.destroyed) { unlisten(); return; }
      this.destroyRef.onDestroy(unlisten);
      await this.refresh();
    } catch { this.error.set('یادآورها بارگذاری نشدند. دوباره تلاش کن.'); }
  }
  async refresh() {
    const revision = ++this.revision;
    try {
      const items = await this.native.listDueReminders();
      if (!this.destroyRef.destroyed && revision === this.revision) { this.items.set(items); this.error.set(''); }
    } catch { if (!this.destroyRef.destroyed) { this.error.set('یادآورها بارگذاری نشدند. دوباره تلاش کن.'); } }
  }
  async act(item: DueReminder, action: 'open' | 'snooze' | 'acknowledge') {
    if (this.busy() !== null) { return; }
    this.busy.set(item.id);
    this.error.set('');
    try {
      if (action === 'open') { await this.native.openReminderItem(item.id, item.dueAt); }
      else if (action === 'snooze') { await this.native.snoozeReminder(item.id, item.dueAt); }
      else { await this.native.acknowledgeReminder(item.id, item.dueAt); }
      await this.refresh();
    } catch (error: unknown) {
      if (error === 'stale_reminder') { await this.refresh(); }
      else { this.error.set('تغییر ذخیره نشد. دوباره تلاش کن.'); }
    } finally {
      this.busy.set(null);
      if (action !== 'open' && !this.destroyRef.destroyed) {
        afterNextRender(() => this.host.nativeElement.querySelector<HTMLButtonElement>('.reminder-card button')?.focus(), { injector: this.injector });
      }
    }
  }
}
