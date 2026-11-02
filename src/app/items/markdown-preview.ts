import { Component, input, signal } from '@angular/core';
import { SavedText } from './saved-text/saved-text';

@Component({
  selector: 'app-markdown-preview',
  imports: [SavedText],
  template: `
    <button type="button" [disabled]="disabled()" [attr.aria-pressed]="preview()" (click)="preview.update(toggle)">Preview</button>
    @if (preview()) {
      <section aria-label="Markdown preview"><app-saved-text [content]="content()" [previewOnly]="true" /></section>
    }
  `,
  styles: `
    :host { display: block; min-width: 0; }
    button { min-height: 28px; padding: 4px 7px; border: 1px solid var(--control-border); border-radius: 4px; background: var(--surface); color: var(--fg); font: inherit; font-size: 12px; cursor: pointer; }
    button:hover, button[aria-pressed="true"] { background: var(--accent-soft); }
    button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
    button:disabled { opacity: .55; cursor: wait; }
  `,
})
export class MarkdownPreview {
  readonly content = input.required<string>();
  readonly disabled = input(false);
  protected readonly preview = signal(false);
  protected readonly toggle = (value: boolean) => !value;
}
