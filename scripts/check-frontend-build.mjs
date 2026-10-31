import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { JSDOM } from 'jsdom';

const root = resolve(import.meta.dirname, '..');
const config = JSON.parse(readFileSync(resolve(root, 'src-tauri/tauri.conf.json'), 'utf8'));
const dist = resolve(root, 'src-tauri', config.build.frontendDist);
const document = new JSDOM(readFileSync(resolve(dist, 'index.html'), 'utf8')).window.document;

// Tauri adds a nonce to critical CSS, making CSP ignore unsafe-inline and block
// Angular's runtime component styles. Keep a normal external stylesheet instead.
assert.equal(document.querySelector('style, script:not([src]), [onload]'), null,
  'Packaged HTML must not require inline styles/scripts or CSS load handlers. Keep inlineCritical disabled.');
const stylesheets = [...document.querySelectorAll('link[rel="stylesheet"]')];
assert.ok(stylesheets.length, 'The packaged frontend must include a global stylesheet.');
for (const link of stylesheets) {
  assert.equal(link.media, '', 'Global CSS must load without a script changing its media attribute.');
  assert.match(link.getAttribute('href'), /^styles-[\w-]+\.css$/);
  assert.ok(readFileSync(resolve(dist, link.getAttribute('href')), 'utf8').length,
    'The referenced global stylesheet must be bundled and nonempty.');
}
console.log('Packaged frontend CSS/CSP check passed.');
