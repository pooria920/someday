async page => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 420, height: 640 });
  await page.addScriptTag({ path: 'node_modules/axe-core/axe.min.js' });
  const results = [];
  const audit = async (surface, palette, mode) => {
    const violations = await page.evaluate(async () => (await axe.run(document, {
      runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'best-practice'] }
    })).violations.map(v => ({ id: v.id, nodes: v.nodes.map(n => ({ target: n.target, summary: n.failureSummary })) })));
    results.push({ surface, palette, mode, violations });
  };
  for (const mode of ['Dark', 'Light']) {
    for (const palette of ['Lime', 'Violet', 'Ocean', 'Rose', 'Amber']) {
      await page.getByRole('button', { name: mode, exact: true }).click();
      await page.getByRole('button', { name: palette + ' palette', exact: true }).click();
      await audit('settings', palette, mode);
      if (palette === 'Violet') await page.screenshot({ path: `output/playwright/palette-settings-${mode.toLowerCase()}.png` });
      await page.getByRole('button', { name: 'Close settings', exact: true }).click();
      await audit('inbox', palette, mode);
      await page.screenshot({ path: `output/playwright/palette-${palette.toLowerCase()}-${mode.toLowerCase()}.png` });
      await page.getByRole('button', { name: 'Settings', exact: true }).click();
    }
  }
  await page.setViewportSize({ width: 360, height: 420 });
  const overflow = await page.evaluate(() => ({
    page: document.documentElement.scrollWidth > innerWidth,
    dialog: document.querySelector('dialog').scrollWidth > document.querySelector('dialog').clientWidth
  }));
  return { results, overflow };
}
