import { addDays, localReminderTime, monthDays, persianDate, persianParts, shiftMonth, reminderDate } from './persian-calendar';

describe('Persian calendar', () => {
  const day = (year: number, month: number, date: number) => new Date(year, month - 1, date, 12);
  it('handles Nowruz and leap Esfand using the platform calendar', () => {
    expect(persianParts(day(2025, 3, 20))).toEqual({ year: 1403, month: 12, day: 30 });
    expect(persianParts(addDays(day(2025, 3, 20), 1))).toEqual({ year: 1404, month: 1, day: 1 });
    expect(monthDays(day(2025, 3, 20))).toHaveLength(30);
    expect(monthDays(day(2026, 3, 20))).toHaveLength(29);
    expect(monthDays(day(2025, 4, 1))).toHaveLength(31);
    expect(monthDays(day(2025, 10, 1))).toHaveLength(30);
    expect(persianDate(day(2025, 3, 21))).toContain('فروردین');
    expect(persianDate(day(2025, 3, 21))).toContain('۱۴۰۴');
  });
  it('navigates months and years and clamps the day at short month boundaries', () => {
    expect(persianParts(shiftMonth(day(2025, 3, 20), 1))).toEqual({ year: 1404, month: 1, day: 30 });
    expect(persianParts(shiftMonth(day(2025, 3, 20), 12))).toEqual({ year: 1404, month: 12, day: 29 });
    expect(persianParts(shiftMonth(day(2025, 3, 21), -1))).toEqual({ year: 1403, month: 12, day: 1 });
    expect(persianParts(shiftMonth(day(2025, 3, 21), -12))).toEqual({ year: 1403, month: 1, day: 1 });
  });
  it('handles Gregorian leap months, year boundaries and English date formatting', () => {
    expect(monthDays(day(2024, 2, 10), 'gregory')).toHaveLength(29);
    expect(monthDays(day(2025, 2, 10), 'gregory')).toHaveLength(28);
    expect(shiftMonth(day(2024, 1, 31), 1, 'gregory')).toEqual(day(2024, 2, 29));
    expect(shiftMonth(day(2024, 2, 29), 12, 'gregory')).toEqual(day(2025, 2, 28));
    expect(shiftMonth(day(2025, 1, 31), -1, 'gregory')).toEqual(day(2024, 12, 31));
    expect(reminderDate(day(2025, 3, 21).getTime(), 'gregory')).toContain('March 21, 2025');
    expect(reminderDate(day(2025, 3, 21).getTime(), 'persian')).toContain('Farvardin 1, 1404');
  });
  it('accepts Persian or Latin 24-hour times and rejects invalid input without truncation', () => {
    const date = day(2030, 4, 1);
    const value = localReminderTime(date, '۱۴:۳۰');
    expect(value).toBe(new Date(2030, 3, 1, 14, 30).getTime());
    expect(localReminderTime(date, '14:30')).toBe(value);
    for (const time of ['', '24:00', '12:60', '1:00', '14:30junk', '-1:00']) { expect(localReminderTime(date, time)).toBeNull(); }
    expect(date.getHours()).toBe(12);
  });
});
