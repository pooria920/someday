import { TestBed } from '@angular/core/testing';
import axe from 'axe-core';
import { SavedText, renderText } from './saved-text';

describe('Saved text', () => {
  it('renders Markdown structure without fetching images or activating links', () => {
    const html = renderText('# سلام\n\n**Bold** and *italic*\n\n- One\n- Two\n\n> Quote\n\n```ts\nconst a = 1;\n```\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n[Link](https://example.com) ![Image](https://example.com/a.png)', null);
    const container = document.createElement('div'); container.innerHTML = html;
    for (const tag of ['h1', 'strong', 'em', 'ul', 'blockquote', 'pre', 'code', 'table']) {
      expect(container.querySelector(tag)).not.toBeNull();
    }
    expect(container.querySelector('a, img, [href], [src]')).toBeNull();
    expect(container.querySelector('h1')?.getAttribute('dir')).toBe('auto');
  });

  it('prefers saved HTML, translates approved styles and strips active content', () => {
    const html = renderText('# ignored', '<p style="font-weight:700;font-style:italic;color:red;position:fixed" onclick="bad()">سلام<img src="https://bad"><iframe src="https://bad"></iframe></p><script>bad()</script><svg onload="bad()"></svg>');
    const container = document.createElement('div'); container.innerHTML = html;
    expect(container.querySelector('em strong')?.textContent).toBe('سلام');
    for (const forbidden of ['ignored', 'onclick', 'style=', '<script', '<svg', '<iframe', '<img', 'https://']) {
      expect(html).not.toContain(forbidden);
    }
  });

  it('preserves original text when toggled and passes DOM accessibility checks', async () => {
    const fixture = TestBed.createComponent(SavedText);
    const content = '  # سلام\r\n**متن** 📝  ';
    fixture.componentRef.setInput('content', content);
    await fixture.whenStable();
    const element: HTMLElement = fixture.nativeElement;
    expect(element.querySelector('strong')).not.toBeNull();
    const button = element.querySelector<HTMLButtonElement>('button')!;
    button.click(); await fixture.whenStable();
    expect(element.querySelector('pre')?.textContent).toBe(content);
    expect(button.getAttribute('aria-pressed')).toBe('true');
    // jsdom has no layout; contrast is checked in the real browser separately.
    const results = await axe.run(element, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations.map(violation => violation.id)).toEqual([]);
    button.click(); await fixture.whenStable();
    expect(element.querySelector('strong')).not.toBeNull();
  });
});
