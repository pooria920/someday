async page => {
  await page.reload();
  await page.getByText('Local storage ready').waitFor();
  await page.setViewportSize({ width: 360, height: 420 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.addScriptTag({ path: 'node_modules/axe-core/axe.min.js' });
  const results = [];
  const toggle = page.getByRole('button', { name: 'Always on top', exact: true });
  for (const mode of ['dark', 'light']) {
    if (mode === 'light') await page.getByRole('button', { name: 'Use light theme', exact: true }).click();
    for (const enabled of [true, false]) {
      await toggle.click();
      await page.waitForFunction(value => document.querySelector('[aria-label="Always on top"]')?.getAttribute('aria-pressed') === String(value), enabled);
      const pressed = await toggle.getAttribute('aria-pressed');
      if (pressed !== String(enabled)) throw new Error('Toggle state mismatch');
      const result = await page.evaluate(async () => {
        const audit = await axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'best-practice'] } });
        return { violations: audit.violations.map(v => ({ id: v.id, nodes: v.nodes.map(n => ({ target: n.target, summary: n.failureSummary })) })), overflow: document.documentElement.scrollWidth > innerWidth };
      });
      results.push({ mode, enabled, ...result });
      if (enabled) await page.screenshot({ path: `output/playwright/on-top-${mode}.png` });
    }
  }
  return results;
}
