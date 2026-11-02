import axe from 'axe-core';
import { TestBed } from '@angular/core/testing';
import { Native } from '../core/platform/tauri/native';
import { StartupSettings } from './startup-settings';

describe('Startup settings', () => {
  const setLaunchAtLogin = vi.fn();
  beforeEach(() => {
    setLaunchAtLogin.mockReset().mockImplementation(async (enabled: boolean) => enabled);
    TestBed.configureTestingModule({ imports: [StartupSettings], providers: [{ provide: Native, useValue: { setLaunchAtLogin } }] });
  });

  it('shows native registration and applies enabling and disabling immediately', async () => {
    const fixture = TestBed.createComponent(StartupSettings);
    fixture.componentRef.setInput('enabled', false);
    fixture.componentInstance.saved.subscribe(value => fixture.componentRef.setInput('enabled', value));
    await fixture.whenStable();
    const checkbox: HTMLInputElement = fixture.nativeElement.querySelector('input');
    expect(checkbox.checked).toBe(false);
    checkbox.click(); await fixture.whenStable();
    expect(setLaunchAtLogin).toHaveBeenLastCalledWith(true);
    expect(checkbox.checked).toBe(true);
    expect(fixture.nativeElement.textContent).toContain('Launch at sign-in enabled.');
    checkbox.click(); await fixture.whenStable();
    expect(setLaunchAtLogin).toHaveBeenLastCalledWith(false);
    expect(checkbox.checked).toBe(false);
  });

  it('retains prior state on failure and blocks repeated changes while saving', async () => {
    let fail!: (reason: string) => void;
    setLaunchAtLogin.mockReturnValueOnce(new Promise((_, reject) => { fail = reject; }));
    const fixture = TestBed.createComponent(StartupSettings);
    fixture.componentRef.setInput('enabled', true);
    await fixture.whenStable();
    const checkbox: HTMLInputElement = fixture.nativeElement.querySelector('input');
    checkbox.click(); await fixture.whenStable();
    expect(checkbox.disabled).toBe(true);
    checkbox.click(); expect(setLaunchAtLogin).toHaveBeenCalledOnce();
    fail('private registry information'); await Promise.resolve(); await fixture.whenStable();
    expect(checkbox.checked).toBe(true);
    expect(checkbox.disabled).toBe(false);
    expect(fixture.nativeElement.textContent).toContain('previous setting is unchanged');
    expect(fixture.nativeElement.textContent).not.toContain('private registry information');
  });

  it('disables unavailable settings and exposes labeled accessible controls', async () => {
    const fixture = TestBed.createComponent(StartupSettings);
    await fixture.whenStable();
    const checkbox: HTMLInputElement = fixture.nativeElement.querySelector('input');
    expect(checkbox.disabled).toBe(true);
    checkbox.click(); expect(setLaunchAtLogin).not.toHaveBeenCalled();
    fixture.componentRef.setInput('enabled', true); await fixture.whenStable();
    expect(checkbox.checked).toBe(true);
    expect(checkbox.labels?.[0].textContent).toContain('Launch Someday');
    const result = await axe.run(fixture.nativeElement, { rules: { 'color-contrast': { enabled: false } } });
    expect(result.violations).toEqual([]);
  });
});
