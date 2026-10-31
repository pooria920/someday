import { Component, computed, input, signal, ViewEncapsulation } from '@angular/core';
import DOMPurify from 'dompurify';
import { Marked } from 'marked';

const tags = ['p', 'br', 'div', 'span', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'strong', 'b', 'em', 'i', 'u', 's', 'del', 'ul', 'ol', 'li', 'blockquote', 'pre', 'code', 'table', 'thead', 'tbody', 'tfoot', 'tr', 'th', 'td', 'hr'];
const markdown = new Marked({
  gfm: true,
  renderer: { html: ({ text }) => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;') },
});

export function renderText(content: string, formattedHtml: string | null): string {
  const source = formattedHtml ?? markdown.parse(content, { async: false });
  const fragment = DOMPurify.sanitize(source, { ALLOWED_TAGS: tags, ALLOWED_ATTR: ['style'], ALLOW_DATA_ATTR: false, RETURN_DOM_FRAGMENT: true });
  for (const element of fragment.querySelectorAll<HTMLElement>('[style]')) {
    const style = element.style;
    const semantic = [
      ['strong', ['bold', '600', '700', '800', '900'].includes(style.fontWeight)],
      ['em', style.fontStyle === 'italic'],
      ['u', (style.textDecorationLine || style.textDecoration) === 'underline'],
      ['s', (style.textDecorationLine || style.textDecoration) === 'line-through'],
    ] as const;
    element.removeAttribute('style');
    for (const [tag, enabled] of semantic) {
      if (enabled) {
        const wrapper = document.createElement(tag);
        wrapper.append(...element.childNodes);
        element.append(wrapper);
      }
    }
  }
  for (const block of fragment.querySelectorAll('p, li, blockquote, td, th, h1, h2, h3, h4, h5, h6')) {
    block.setAttribute('dir', 'auto');
  }
  const container = document.createElement('div');
  container.append(fragment);
  return container.innerHTML;
}

@Component({
  selector: 'app-saved-text',
  encapsulation: ViewEncapsulation.None,
  template: `
    <div class="saved-text">
      <button type="button" class="text-view-toggle" [attr.aria-pressed]="original()" (click)="original.update(invert)">{{ original() ? 'Show formatted text' : 'Show original text' }}</button>
      @if (original() || !expanded()) {
        <pre class="item-content text-original" dir="auto">{{ content() }}</pre>
      } @else {
        <div class="item-content text-formatted" dir="auto" [innerHTML]="rendered()"></div>
      }
    </div>
  `,
  styles: `
    app-saved-text { display: block; min-width: 0; }
    .saved-text .text-view-toggle { margin-top: 10px; padding: 4px 6px; background: transparent; border: 1px solid var(--control-border); border-radius: 5px; color: var(--fg); font: inherit; font-size: 12px; cursor: pointer; }
    .saved-text .text-view-toggle:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
    .saved-text .item-content { text-align: start; line-height: 1.8; padding: 12px; margin: 8px 0 0; border: 1px solid var(--border); border-radius: 8px; background: var(--bg); overflow-wrap: anywhere; }
    .saved-text .text-original { white-space: pre-wrap; unicode-bidi: plaintext; font: inherit; }
    .saved-text .text-formatted { overflow-x: auto; white-space: normal; }
    .saved-text .text-formatted :is(p, ul, ol, blockquote, pre, table) { margin-block: 0.7em; }
    .saved-text .text-formatted > :first-child { margin-block-start: 0; }
    .saved-text .text-formatted > :last-child { margin-block-end: 0; }
    .saved-text .text-formatted :is(h1,h2,h3,h4,h5,h6) { font-weight: 700; line-height: 1.5; margin-block: 0.8em 0.4em; }
    .saved-text .text-formatted h1 { font-size: 1.5em; }
    .saved-text .text-formatted h2 { font-size: 1.3em; }
    .saved-text .text-formatted :is(h3,h4,h5,h6) { font-size: 1.1em; }
    .saved-text .text-formatted :is(ul,ol) { padding-inline-start: 1.6em; list-style-position: outside; }
    .saved-text .text-formatted ul { list-style-type: disc; }
    .saved-text .text-formatted ol { list-style-type: decimal; }
    .saved-text .text-formatted blockquote { border-inline-start: 3px solid var(--control-border); padding-inline-start: 10px; }
    .saved-text .text-formatted :is(pre,code) { font-family: Consolas, monospace; }
    .saved-text .text-formatted pre { direction: ltr; unicode-bidi: isolate; text-align: left; white-space: pre; overflow-x: auto; padding: 8px; background: var(--raised); border-radius: 5px; }
    .saved-text .text-formatted code { unicode-bidi: isolate; }
    .saved-text .text-formatted table { display: block; max-width: 100%; overflow-x: auto; border-collapse: collapse; }
    .saved-text .text-formatted :is(td,th) { border: 1px solid var(--control-border); padding: 4px 8px; }
  `,
})
export class SavedText {
  readonly content = input.required<string>();
  readonly expanded = input(true);
  readonly formattedHtml = input<string | null>(null);
  protected readonly original = signal(false);
  protected readonly invert = (value: boolean) => !value;
  protected readonly rendered = computed(() => renderText(this.content(), this.formattedHtml()));
}
