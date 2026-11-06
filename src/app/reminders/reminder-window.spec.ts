import axe from 'axe-core';
import { TestBed } from '@angular/core/testing';
import { Native } from '../core/platform/tauri/native';
import { ReminderWindow } from './reminder-window';

describe('Reminder window', () => {
  const item = { id: 'one', dueAt: 1000, preview: '<script>private text</script>' };
  const native = { listenDueReminders: vi.fn(), listDueReminders: vi.fn(), acknowledgeReminder: vi.fn(), snoozeReminder: vi.fn(), openReminderItem: vi.fn() };
  beforeEach(() => {
    native.listenDueReminders.mockReset().mockResolvedValue(vi.fn());
    native.listDueReminders.mockReset().mockResolvedValue([item]);
    for (const action of [native.acknowledgeReminder, native.snoozeReminder, native.openReminderItem]) { action.mockReset().mockResolvedValue(undefined); }
    TestBed.configureTestingModule({ imports: [ReminderWindow], providers: [{ provide: Native, useValue: native }] });
  });
  it('renders bounded plain text and routes all actions with the original due time', async () => {
    const fixture = TestBed.createComponent(ReminderWindow); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    expect(element.querySelector('script')).toBeNull();
    expect(element.textContent).toContain(item.preview);
    await fixture.componentInstance.act(item, 'open');
    expect(native.openReminderItem).toHaveBeenCalledWith('one', 1000);
    await fixture.componentInstance.act(item, 'snooze');
    expect(native.snoozeReminder).toHaveBeenCalledWith('one', 1000);
    await fixture.componentInstance.act(item, 'acknowledge');
    expect(native.acknowledgeReminder).toHaveBeenCalledWith('one', 1000);
    expect((await axe.run(element, { rules: { 'color-contrast': { enabled: false } } })).violations).toEqual([]);
  });
  it('keeps failed reminders visible without exposing native error details and reconciles stale actions', async () => {
    const fixture = TestBed.createComponent(ReminderWindow); await fixture.whenStable();
    native.snoozeReminder.mockRejectedValueOnce('private path');
    await fixture.componentInstance.act(item, 'snooze'); await fixture.whenStable();
    expect(fixture.componentInstance.items()).toEqual([item]);
    expect(fixture.nativeElement.textContent).not.toContain('private path');
    expect(fixture.componentInstance.error()).not.toBe('');
    native.acknowledgeReminder.mockRejectedValueOnce('stale_reminder');
    native.listDueReminders.mockResolvedValueOnce([]);
    await fixture.componentInstance.act(item, 'acknowledge');
    expect(fixture.componentInstance.items()).toEqual([]);
  });
  it('uses newer events over an older initial read and removes its listener on destruction', async () => {
    let finish!: (items: typeof item[]) => void;
    native.listDueReminders.mockReturnValueOnce(new Promise(resolve => { finish = resolve; }));
    const unlisten = vi.fn(); native.listenDueReminders.mockResolvedValueOnce(unlisten);
    const fixture = TestBed.createComponent(ReminderWindow);
    await Promise.resolve(); await Promise.resolve();
    native.listenDueReminders.mock.calls[0][0]([]);
    finish([item]); await fixture.whenStable();
    expect(fixture.componentInstance.items()).toEqual([]);
    fixture.destroy(); expect(unlisten).toHaveBeenCalledOnce();
  });
});
