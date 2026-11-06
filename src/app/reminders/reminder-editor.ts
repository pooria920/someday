import { afterNextRender, Component, computed, ElementRef, inject, Injector, input, output, signal } from '@angular/core';
import { NgIcon, provideIcons } from '@ng-icons/core';
import { saxArrowLeftOutline, saxArrowRight1Outline, saxTrashOutline } from '@ng-icons/iconsax/outline';
import { addDays, localReminderTime, monthDays, CalendarType, calendarParts, shiftMonth } from './persian-calendar';

@Component({
  selector: 'app-reminder-editor',
  imports: [NgIcon],
  providers: [provideIcons({ saxArrowLeftOutline, saxArrowRight1Outline, saxTrashOutline })],
  template: `
    <section class="reminder-editor" dir="ltr" lang="en" aria-label="Reminder date and time">
      <div class="calendar-navigation">
        <div class="month-controls">
          <button type="button" class="icon-action year-control" [disabled]="busy()" (click)="move(-12)" aria-label="Previous year" title="Previous year"><ng-icon name="saxArrowLeftOutline" size="15" aria-hidden="true" /><ng-icon name="saxArrowLeftOutline" size="15" aria-hidden="true" /></button>
          <button type="button" class="icon-action" [disabled]="busy()" (click)="move(-1)" aria-label="Previous month" title="Previous month"><ng-icon name="saxArrowLeftOutline" size="15" aria-hidden="true" /></button>
        </div>
        <strong class="month-title" aria-live="polite">{{ monthLabel() }}</strong>
        <div class="month-controls">
          <button type="button" class="icon-action" [disabled]="busy()" (click)="move(1)" aria-label="Next month" title="Next month"><ng-icon name="saxArrowRight1Outline" size="15" aria-hidden="true" /></button>
          <button type="button" class="icon-action year-control" [disabled]="busy()" (click)="move(12)" aria-label="Next year" title="Next year"><ng-icon name="saxArrowRight1Outline" size="15" aria-hidden="true" /><ng-icon name="saxArrowRight1Outline" size="15" aria-hidden="true" /></button>
        </div>
      </div>
      <div class="calendar" role="group" aria-label="Days of the month" (keydown)="onKey($event)">
        @for (name of weekdays(); track name) { <span aria-hidden="true">{{ name }}</span> }
        @for (space of blanks(); track space) { <span aria-hidden="true"></span> }
        @for (day of days(); track day.getTime()) {
          <button type="button" [disabled]="busy()" [attr.data-day]="day.getTime()" [tabIndex]="sameDay(day) ? 0 : -1"
            [attr.aria-pressed]="sameDay(day)" [attr.aria-label]="dayLabel(day)" [attr.aria-current]="isToday(day) ? 'date' : null"
            (click)="selected.set(day)">{{ number(parts(day).day) }}</button>
        }
      </div>
      <div class="calendar-bottom"><span><span class="today-dot" aria-hidden="true"></span>Today</span><button type="button" class="text-action" [disabled]="busy()" (click)="today()">Go to today</button></div>
      <div class="schedule-fields">
        <div class="selected-date"><span class="field-label">Selected date</span><strong>{{ selectedLabel() }}</strong></div>
        <div class="time-field">
          <span class="field-label" id="reminder-time-label">Time <span>(24-hour)</span></span>
          <div class="time-input" dir="ltr" role="group" aria-labelledby="reminder-time-label" aria-describedby="reminder-help">
            <select #hourSelect aria-label="Hour" [value]="hour()" [disabled]="busy()" (change)="chooseTime('hour', hourSelect.value)">
              @for (option of hourOptions; track option.value) { <option [value]="option.value" [selected]="hour() === option.value">{{ option.label }}</option> }
            </select>
            <span class="time-separator" aria-hidden="true">:</span>
            <select #minuteSelect aria-label="Minute" [value]="minute()" [disabled]="busy()" (change)="chooseTime('minute', minuteSelect.value)">
              @for (option of minuteOptions; track option.value) { <option [value]="option.value" [selected]="minute() === option.value">{{ option.label }}</option> }
            </select>
          </div>
        </div>
      </div>
      <div class="editor-feedback">
        @if (error() || saveError()) { <div id="reminder-help" role="alert" class="error">{{ error() || saveError() }}</div> }
        @else { <div id="reminder-help" class="editor-help">Windows local time · If Someday is closed, reminders appear on the next launch.</div> }
      </div>
      <div class="editor-actions">
        <button type="button" class="primary-action" [disabled]="busy()" (click)="submit()">{{ busy() ? 'Saving…' : 'Save reminder' }}</button>
        <button type="button" class="text-action cancel-action" [disabled]="busy()" (click)="cancelled.emit()">Cancel</button>
        @if (initial() !== null) { <button type="button" class="icon-action remove-action" [disabled]="busy()" (click)="save.emit(null)" aria-label="Remove reminder" title="Remove reminder"><ng-icon name="saxTrashOutline" size="18" aria-hidden="true" /></button> }
      </div>
    </section>
  `,
  styleUrls: ['./reminders.css', './reminder-editor.css'],
})
export class ReminderEditor {
  readonly calendar = input<CalendarType>('persian');
  readonly initial = input<number | null>(null);
  readonly busy = input(false);
  readonly saveError = input('');
  readonly save = output<number | null>();
  readonly cancelled = output<void>();
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly injector = inject(Injector);
  readonly selected = signal(addDays(new Date(), 0));
  readonly hour = signal(9);
  readonly minute = signal(0);
  readonly timeText = computed(() => [this.hour(), this.minute()].map(part => String(part).padStart(2, '0')).join(':'));
  readonly hourOptions = Array.from({ length: 24 }, (_, value) => ({ value, label: String(value).padStart(2, '0') }));
  readonly minuteOptions = Array.from({ length: 60 }, (_, value) => ({ value, label: String(value).padStart(2, '0') }));
  readonly error = signal('');
  readonly weekdays = computed(() => this.calendar() === 'persian' ? ['Sat', 'Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri'] : ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']);
  readonly number = String;
  readonly parts = (day: Date) => calendarParts(day, this.calendar());
  readonly days = computed(() => monthDays(this.selected(), this.calendar()));
  readonly blanks = computed(() => Array.from({ length: (this.days()[0].getDay() + (this.calendar() === 'persian' ? 1 : 0)) % 7 }, (_, index) => index));
  readonly locale = computed(() => `en-US-u-ca-${this.calendar()}`);
  readonly selectedLabel = computed(() => new Intl.DateTimeFormat(this.locale(), { year: 'numeric', month: 'long', day: 'numeric' }).format(this.selected()));
  readonly monthLabel = computed(() => new Intl.DateTimeFormat(this.locale(), { year: 'numeric', month: 'long' }).format(this.selected()));
  readonly dayLabel = (day: Date) => new Intl.DateTimeFormat(this.locale(), { dateStyle: 'full' }).format(day);

  constructor() {
    afterNextRender(() => {
      const initial = this.initial();
      const date = initial === null ? new Date(Date.now() + 3_600_000) : new Date(initial);
      this.selected.set(addDays(date, 0));
      this.hour.set(date.getHours());
      this.minute.set(date.getMinutes());
    });
  }
  sameDay(day: Date) { return day.toDateString() === this.selected().toDateString(); }
  isToday(day: Date) { return day.toDateString() === new Date().toDateString(); }
  move(months: number) { this.selected.set(shiftMonth(this.selected(), months, this.calendar())); }
  today() { this.selected.set(addDays(new Date(), 0)); }
  chooseTime(part: 'hour' | 'minute', value: string) {
    (part === 'hour' ? this.hour : this.minute).set(Number(value));
    this.error.set('');
  }
  onKey(event: KeyboardEvent) {
    if (!(event.target instanceof HTMLButtonElement) || !event.target.hasAttribute('data-day') || this.busy()) { return; }
    const shifts: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 };
    if (event.key in shifts) { this.selected.set(addDays(this.selected(), shifts[event.key])); }
    else if (event.key === 'PageUp' || event.key === 'PageDown') { this.move((event.key === 'PageUp' ? -1 : 1) * (event.ctrlKey ? 12 : 1)); }
    else if (event.key === 'Home') { this.selected.set(this.days()[0]); }
    else if (event.key === 'End') { this.selected.set(this.days().at(-1)!); }
    else { return; }
    event.preventDefault();
    afterNextRender(() => this.host.nativeElement.querySelector<HTMLButtonElement>('[data-day][tabindex="0"]')?.focus(), { injector: this.injector });
  }
  submit() {
    const value = localReminderTime(this.selected(), this.timeText());
    if (value === null || value <= Date.now()) { this.error.set('Choose a valid date and time in the future.'); return; }
    this.save.emit(value);
  }
}
