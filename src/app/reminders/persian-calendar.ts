export type CalendarType = 'persian' | 'gregory';

const partsFormatter = new Intl.DateTimeFormat('en-US-u-ca-persian', { year: 'numeric', month: 'numeric', day: 'numeric' });
export const persianDate = (value: number | Date) => new Intl.DateTimeFormat('fa-IR-u-ca-persian', {
  year: 'numeric', month: 'long', day: 'numeric', hour: '2-digit', minute: '2-digit', hourCycle: 'h23',
}).format(value);
export const persianNumber = (value: number) => new Intl.NumberFormat('fa-IR', { useGrouping: false }).format(value);

export function persianParts(date: Date) {
  const parts = partsFormatter.formatToParts(date);
  const read = (type: Intl.DateTimeFormatPartTypes) => Number(parts.find(part => part.type === type)?.value);
  return { year: read('year'), month: read('month'), day: read('day') };
}

export function addDays(date: Date, count: number): Date {
  const next = new Date(date);
  next.setHours(12, 0, 0, 0);
  next.setDate(next.getDate() + count);
  return next;
}

export function calendarParts(date: Date, calendar: CalendarType) {
  return calendar === 'persian' ? persianParts(date) : { year: date.getFullYear(), month: date.getMonth() + 1, day: date.getDate() };
}

export function reminderDate(value: number, calendar: CalendarType): string {
  return new Intl.DateTimeFormat(`en-US-u-ca-${calendar}`, { year: 'numeric', month: 'long', day: 'numeric', hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }).format(value);
}

export function monthDays(anchor: Date, calendar: CalendarType = 'persian'): Date[] {
  const first = addDays(anchor, 1 - calendarParts(anchor, calendar).day);
  const month = calendarParts(first, calendar).month;
  const days: Date[] = [];
  for (let day = first; calendarParts(day, calendar).month === month; day = addDays(day, 1)) { days.push(day); }
  return days;
}

export function shiftMonth(date: Date, direction: number, calendar: CalendarType = 'persian'): Date {
  let result = date;
  const wanted = calendarParts(date, calendar).day;
  for (let count = 0; count < Math.abs(direction); count++) {
    const days = monthDays(result, calendar);
    const adjacent = monthDays(addDays(direction > 0 ? days[days.length - 1] : days[0], direction > 0 ? 1 : -1), calendar);
    result = adjacent[Math.min(wanted, adjacent.length) - 1];
  }
  return result;
}

export function localReminderTime(day: Date, source: string): number | null {
  const normalized = source.replace(/[۰-۹]/g, digit => String(digit.charCodeAt(0) - 0x6f0));
  if (!/^\d{2}:\d{2}$/.test(normalized)) { return null; }
  const [hour, minute] = normalized.split(':').map(Number);
  if (hour > 23 || minute > 59) { return null; }
  const date = new Date(day);
  date.setHours(hour, minute, 0, 0);
  if (date.getFullYear() !== day.getFullYear() || date.getMonth() !== day.getMonth() || date.getDate() !== day.getDate() || date.getHours() !== hour || date.getMinutes() !== minute) { return null; }
  return date.getTime();
}
