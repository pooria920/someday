import { afterNextRender, Component, computed, DestroyRef, effect, ElementRef, inject, input, signal } from '@angular/core';
import { ImageMetadata, Native } from '../../core/platform/tauri/native';

@Component({
  selector: 'app-saved-image',
  host: { '[class.thumbnail]': "variant() === 'thumbnail'" },
  styles: `
    :host { display: block; min-width: 0; }
    :host(:not(.thumbnail)) { margin-top: 12px; }
    :host.thumbnail { display: grid; place-items: center; flex-shrink: 0; width: 48px; height: 48px; overflow: hidden; border-radius: 8px; background: var(--raised); }
    img { display: block; width: 100%; height: auto; object-fit: contain; border-radius: 8px; }
    :host.thumbnail img { height: 100%; object-fit: cover; }
    .placeholder { display: block; padding: 12px; color: var(--muted); font-size: 12px; text-align: center; }
    :host.thumbnail .placeholder { padding: 2px; font-size: 10px; }
    .error { color: var(--danger); }
    button { cursor: pointer; color: var(--fg); border: 1px solid var(--control-border); border-radius: 8px; padding: 6px 10px; margin-top: 8px; font: inherit; background: var(--surface); }
    button:hover { background: var(--raised); }
    button:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  `,
  template: `
    @if (failed()) {
      @if (variant() === 'original') {
        <div class="placeholder error" role="alert">Could not load this image.<br /><button type="button" (click)="retry()">Retry image</button></div>
      } @else { <span class="placeholder" title="Image preview unavailable" aria-hidden="true">Image</span> }
    } @else if (url(); as source) {
      <img [src]="source" [alt]="variant() === 'thumbnail' ? '' : image().name" decoding="async" (error)="failed.set(true)" />
    } @else {
      <span class="placeholder" [attr.role]="variant() === 'original' ? 'status' : null" [attr.aria-hidden]="variant() === 'thumbnail' ? 'true' : null">{{ variant() === 'thumbnail' ? 'Image' : 'Loading image…' }}</span>
    }
  `,
})
export class SavedImage {
  readonly itemId = input.required<string>();
  readonly image = input.required<ImageMetadata>();
  readonly variant = input.required<'thumbnail' | 'original'>();
  private readonly native = inject(Native);
  private readonly element = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly destroyRef = inject(DestroyRef);
  private readonly visible = signal(false);
  private readonly attempt = signal(0);
  private readonly mimeType = computed(() => this.image().mimeType);
  protected readonly url = signal('');
  protected readonly failed = signal(false);

  constructor() {
    afterNextRender(() => {
      if (this.variant() === 'original' || typeof IntersectionObserver === 'undefined') {
        this.visible.set(true);
        return;
      }
      const observer = new IntersectionObserver(entries => {
        if (entries.some(entry => entry.isIntersecting)) {
          this.visible.set(true);
          observer.disconnect();
        }
      });
      observer.observe(this.element.nativeElement);
      this.destroyRef.onDestroy(() => observer.disconnect());
    });
    effect(onCleanup => {
      const id = this.itemId();
      const variant = this.variant();
      const mime = variant === 'thumbnail' ? 'image/png' : this.mimeType();
      this.attempt();
      if (!this.visible()) { return; }
      this.url.set('');
      this.failed.set(false);
      let cancelled = false;
      let objectUrl: string | undefined;
      void this.native.loadItemImage(id, variant).then(bytes => {
        if (cancelled) { return; }
        objectUrl = URL.createObjectURL(new Blob([bytes], { type: mime }));
        this.url.set(objectUrl);
      }).catch(() => { if (!cancelled) { this.failed.set(true); } });
      onCleanup(() => {
        cancelled = true;
        if (objectUrl) { URL.revokeObjectURL(objectUrl); }
      });
    });
  }

  protected retry() { this.attempt.update(value => value + 1); }
}
