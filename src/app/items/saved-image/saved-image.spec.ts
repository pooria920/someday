import { TestBed } from '@angular/core/testing';
import { Native } from '../../core/platform/tauri/native';
import { SavedImage } from './saved-image';

describe('Saved image', () => {
  const image = { name: 'Photo.jpg', mimeType: 'image/jpeg', width: 640, height: 480, byteSize: 512 };
  const loadItemImage = vi.fn();
  const createObjectURL = vi.fn();
  const revokeObjectURL = vi.fn();
  beforeEach(() => {
    loadItemImage.mockReset().mockResolvedValue(new ArrayBuffer(4));
    createObjectURL.mockReset().mockReturnValue('blob:saved-image');
    revokeObjectURL.mockReset();
    vi.stubGlobal('URL', class extends URL {
      static override createObjectURL = createObjectURL;
      static override revokeObjectURL = revokeObjectURL;
    });
    TestBed.configureTestingModule({ imports: [SavedImage], providers: [{ provide: Native, useValue: { loadItemImage } }] });
  });
  afterEach(() => { TestBed.resetTestingModule(); vi.unstubAllGlobals(); });

  function create(variant: 'thumbnail' | 'original' = 'thumbnail') {
    const fixture = TestBed.createComponent(SavedImage);
    fixture.componentRef.setInput('itemId', 'photo');
    fixture.componentRef.setInput('image', image);
    fixture.componentRef.setInput('variant', variant);
    return fixture;
  }

  it.each(['thumbnail', 'original'] as const)('loads %s as a typed Blob and releases it when removed', async variant => {
    const fixture = create(variant); await fixture.whenStable();
    expect(loadItemImage).toHaveBeenCalledExactlyOnceWith('photo', variant);
    const blob: Blob = createObjectURL.mock.calls[0][0];
    expect(blob.type).toBe(variant === 'thumbnail' ? 'image/png' : 'image/jpeg');
    const img = fixture.nativeElement.querySelector('img');
    expect(img.src).toBe('blob:saved-image');
    expect(img.alt).toBe(variant === 'thumbnail' ? '' : 'Photo.jpg');
    fixture.destroy();
    expect(revokeObjectURL).toHaveBeenCalledExactlyOnceWith('blob:saved-image');
  });

  it('does not create or leak a Blob URL for stale loads after a changed item or destruction', async () => {
    let first!: (value: ArrayBuffer) => void;
    loadItemImage.mockReturnValueOnce(new Promise(resolve => { first = resolve; }));
    const fixture = create('original'); await fixture.whenStable();
    fixture.componentRef.setInput('itemId', 'new-photo'); await fixture.whenStable();
    expect(createObjectURL).toHaveBeenCalledOnce();
    first(new ArrayBuffer(2)); await fixture.whenStable();
    expect(createObjectURL).toHaveBeenCalledOnce();
    fixture.destroy();
    expect(revokeObjectURL).toHaveBeenCalledOnce();

    let late!: (value: ArrayBuffer) => void;
    loadItemImage.mockReturnValueOnce(new Promise(resolve => { late = resolve; }));
    const removed = create('original'); await removed.whenStable(); removed.destroy();
    late(new ArrayBuffer(2)); await Promise.resolve();
    expect(createObjectURL).toHaveBeenCalledOnce();
  });

  it('releases the previous URL when the input changes without reloading on identical MIME metadata', async () => {
    const fixture = create('original'); await fixture.whenStable();
    fixture.componentRef.setInput('image', { ...image, name: 'Renamed.jpg' }); await fixture.whenStable();
    expect(loadItemImage).toHaveBeenCalledOnce();
    expect(fixture.nativeElement.querySelector('img').alt).toBe('Renamed.jpg');
    fixture.componentRef.setInput('itemId', 'second'); await fixture.whenStable();
    expect(revokeObjectURL).toHaveBeenCalledOnce();
    expect(loadItemImage).toHaveBeenLastCalledWith('second', 'original');
  });

  it('shows a safe load error and retries without exposing a native error', async () => {
    loadItemImage.mockRejectedValueOnce('private path');
    const fixture = create('original'); await fixture.whenStable();
    await vi.waitFor(() => {
      fixture.detectChanges();
      expect(fixture.nativeElement.textContent).toContain('Could not load this image');
    });
    expect(fixture.nativeElement.textContent).not.toContain('private path');
    fixture.nativeElement.querySelector('button').click(); await fixture.whenStable();
    expect(loadItemImage).toHaveBeenCalledTimes(2);
    expect(fixture.nativeElement.querySelector('img')).not.toBeNull();
  });

  it('defers thumbnail reads until the row intersects the viewport and disconnects its observer', async () => {
    let notify!: (entries: { isIntersecting: boolean }[]) => void;
    const disconnect = vi.fn();
    vi.stubGlobal('IntersectionObserver', class {
      constructor(callback: typeof notify) { notify = callback; }
      observe = vi.fn();
      disconnect = disconnect;
    });
    const fixture = create(); await fixture.whenStable();
    expect(loadItemImage).not.toHaveBeenCalled();
    notify([{ isIntersecting: false }]); await fixture.whenStable();
    expect(loadItemImage).not.toHaveBeenCalled();
    notify([{ isIntersecting: true }]); await fixture.whenStable();
    expect(loadItemImage).toHaveBeenCalledOnce();
    expect(disconnect).toHaveBeenCalledOnce();
    fixture.destroy();
    expect(disconnect).toHaveBeenCalledTimes(2);
  });
});
