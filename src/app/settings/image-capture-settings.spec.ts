import { TestBed } from '@angular/core/testing';
import { ImageCaptureSettings } from './image-capture-settings';
import { Native } from '../core/platform/tauri/native';

describe('Automatic image settings', () => {
  const setAutoSaveImages = vi.fn();
  beforeEach(() => {
    setAutoSaveImages.mockReset().mockImplementation(async (enabled: boolean) => enabled);
    TestBed.configureTestingModule({ imports: [ImageCaptureSettings], providers: [{ provide: Native, useValue: { setAutoSaveImages } }] });
  });
  it('defaults off and applies both changes immediately', async () => {
    const fixture = TestBed.createComponent(ImageCaptureSettings);
    fixture.componentInstance.saved.subscribe(value => fixture.componentRef.setInput('enabled', value));
    await fixture.whenStable();
    const checkbox: HTMLInputElement = fixture.nativeElement.querySelector('input');
    expect(checkbox.checked).toBe(false);
    checkbox.click(); await fixture.whenStable();
    expect(setAutoSaveImages).toHaveBeenLastCalledWith(true);
    expect(checkbox.checked).toBe(true);
    checkbox.click(); await fixture.whenStable();
    expect(setAutoSaveImages).toHaveBeenLastCalledWith(false);
    expect(checkbox.checked).toBe(false);
  });
  it('keeps the previous value on failure and prevents repeated changes while saving', async () => {
    let fail!: (reason: string) => void;
    setAutoSaveImages.mockReturnValueOnce(new Promise((_, reject) => { fail = reject; }));
    const fixture = TestBed.createComponent(ImageCaptureSettings);
    fixture.componentRef.setInput('enabled', true);
    await fixture.whenStable();
    const checkbox: HTMLInputElement = fixture.nativeElement.querySelector('input');
    checkbox.click(); await fixture.whenStable();
    expect(checkbox.disabled).toBe(true);
    checkbox.click(); expect(setAutoSaveImages).toHaveBeenCalledOnce();
    fail('private path'); await Promise.resolve(); await fixture.whenStable();
    expect(checkbox.checked).toBe(true);
    expect(checkbox.disabled).toBe(false);
    expect(fixture.nativeElement.textContent).toContain('previous setting is unchanged');
    expect(fixture.nativeElement.textContent).not.toContain('private path');
  });
});
