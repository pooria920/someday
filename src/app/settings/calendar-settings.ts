import { Component, inject, input, model, output, signal } from '@angular/core';
import { Native } from '../core/platform/tauri/native';
import { CalendarType } from '../reminders/persian-calendar';

@Component({
  selector: 'app-calendar-settings',
  template: `
    <section aria-labelledby="calendar-title" [attr.aria-busy]="busy()">
      <h3 id="calendar-title">Calendar</h3>
      <label for="calendar-type">Calendar type</label>
      <select #choice id="calendar-type" [value]="calendar()" [disabled]="busy()" aria-describedby="calendar-help"
        (change)="save(choice.value); choice.value = calendar()">
        <option value="persian" [selected]="calendar() === 'persian'">Persian (Solar Hijri)</option>
        <option value="gregory" [selected]="calendar() === 'gregory'">Gregorian</option>
      </select>
      <p id="calendar-help">Used to choose and display reminder dates. Existing reminder times stay the same.</p>
      @if (error()) { <p role="alert">{{ error() }}</p> }
    </section>
  `,
  styleUrl: './toggle-settings.css',
  styles: `label { display:block; margin-bottom:8px; } select { width:100%; min-height:38px; padding:8px; border:1px solid var(--control-border); border-radius:var(--radius, 6px); background:var(--surface); color:var(--fg); font:inherit; } select:focus-visible { outline:2px solid var(--accent); outline-offset:3px; }`,
})
export class CalendarSettings {
  private readonly native = inject(Native);
  readonly calendar = input.required<CalendarType>();
  readonly busy = model(false);
  readonly saved = output<CalendarType>();
  protected readonly error = signal('');

  protected async save(value: string) {
    if ((value !== 'persian' && value !== 'gregory') || this.busy() || value === this.calendar()) { return; }
    this.busy.set(true); this.error.set('');
    try { await this.native.setCalendarType(value); this.saved.emit(value); }
    catch { this.error.set('Could not save your calendar. Please try again.'); }
    finally { this.busy.set(false); }
  }
}
