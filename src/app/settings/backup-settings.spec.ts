import axe from 'axe-core';
import { TestBed } from '@angular/core/testing';
import { BackupSettings } from './backup-settings';
import { Native } from '../core/platform/tauri/native';

describe('Backup settings', () => {
  const native = { createBackup: vi.fn(), prepareRestore: vi.fn(), restoreBackup: vi.fn() };
  const prepared = { token: 'opaque-token', name: 'my.someday-backup', itemCount: 12 };
  beforeEach(() => {
    native.createBackup.mockReset().mockResolvedValue(true);
    native.prepareRestore.mockReset().mockResolvedValue(prepared);
    native.restoreBackup.mockReset().mockResolvedValue(undefined);
    TestBed.configureTestingModule({ imports: [BackupSettings], providers: [{ provide: Native, useValue: native }] });
  });
  const button = (element: HTMLElement, text: string) => [...element.querySelectorAll('button')].find(b => b.textContent?.includes(text))!;
  it('reports success and cancellation without exposing native errors', async () => {
    const fixture = TestBed.createComponent(BackupSettings); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    button(element, 'Create backup').click(); await fixture.whenStable();
    expect(element.textContent).toContain('Backup saved.');
    native.createBackup.mockResolvedValueOnce(false);
    button(element, 'Create backup').click(); await fixture.whenStable();
    expect(element.querySelector('[role=status]')?.textContent).toBe('');
    native.createBackup.mockRejectedValueOnce('private path');
    button(element, 'Create backup').click(); await fixture.whenStable();
    expect(element.textContent).toContain('existing file was kept');
    expect(element.textContent).not.toContain('private path');
    native.prepareRestore.mockResolvedValueOnce(null);
    button(element, 'Restore backup').click(); await fixture.whenStable();
    expect(element.querySelector('[role=group]')).toBeNull();
  });
  it('requires explicit confirmation, shows counts and focuses cancel before destructive action', async () => {
    const fixture = TestBed.createComponent(BackupSettings); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    button(element, 'Restore backup').click(); await fixture.whenStable();
    expect(element.textContent).toContain('my.someday-backup');
    expect(element.textContent).toContain('12 items');
    expect(element.textContent).toContain('unsaved note drafts will be lost');
    expect(native.restoreBackup).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(button(element, 'Cancel'));
    button(element, 'Cancel').click(); await fixture.whenStable();
    expect(document.activeElement).toBe(button(element, 'Restore backup'));
    button(element, 'Restore backup').click(); await fixture.whenStable();
    button(element, 'Replace data and restart').click(); await fixture.whenStable();
    expect(native.restoreBackup).toHaveBeenCalledExactlyOnceWith('opaque-token');
    expect([...element.querySelectorAll('button')].every(b => b.disabled)).toBe(true);
  });
  it('prevents repeated operations and retains confirmation safely on failure', async () => {
    let reject!: (error: string) => void;
    native.prepareRestore.mockReturnValueOnce(new Promise((_, fail) => { reject = fail; }));
    const fixture = TestBed.createComponent(BackupSettings); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    button(element, 'Restore backup').click(); await fixture.whenStable();
    expect([...element.querySelectorAll('button')].every(b => b.disabled)).toBe(true);
    button(element, 'Restore backup').click(); expect(native.prepareRestore).toHaveBeenCalledOnce();
    reject('backup_newer_version'); await Promise.resolve(); await fixture.whenStable();
    expect(element.textContent).toContain('newer version');
    button(element, 'Restore backup').click(); await fixture.whenStable();
    native.restoreBackup.mockRejectedValueOnce('native detail');
    button(element, 'Replace data and restart').click(); await fixture.whenStable();
    expect(element.textContent).toContain('Your current data is unchanged');
    expect(element.textContent).not.toContain('native detail');
    expect(button(element, 'Cancel').disabled).toBe(false);
  });
  it('passes axe checks for controls, confirmation and errors', async () => {
    const fixture = TestBed.createComponent(BackupSettings); await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    const check = async () => expect((await axe.run(element, { rules: { 'color-contrast': { enabled: false } } })).violations).toEqual([]);
    await check();
    button(element, 'Restore backup').click(); await fixture.whenStable(); await check();
    native.restoreBackup.mockRejectedValueOnce('failure');
    button(element, 'Replace data and restart').click(); await fixture.whenStable(); await check();
  });
});
