async page => {
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.reload();
  await page.getByText('Local storage ready').waitFor();
  await page.addScriptTag({path:'node_modules/axe-core/axe.min.js'});
  const results=[];
  for (const width of [360,420]) {
    await page.setViewportSize({width,height:760});
    for (const theme of ['dark','light']) {
      if(theme==='light') await page.getByRole('button',{name:'Use light theme'}).click();
      const axe=await page.evaluate(async()=>{const result=await axe.run(document,{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21aa','best-practice']}});return result.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}));});
      const overflow=await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth);
      results.push({width,theme,overflow,violations:axe});
      await page.screenshot({path:`output/playwright/bold-inbox-${theme}-${width}.png`});
      if(theme==='light') await page.getByRole('button',{name:'Use dark theme'}).click();
    }
  }
  await page.getByRole('button',{name:'New note',exact:true}).click();
  await page.getByRole('textbox',{name:'Your note'}).fill('A small thought, saved for later.\nیک ایده برای بعد.');
  await page.screenshot({path:'output/playwright/bold-note-dark.png'});
  results.push({dialog:'note',violations:await page.evaluate(async()=>(await axe.run(document)).violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)})))});
  await page.getByRole('button',{name:'Cancel',exact:true}).click();
  await page.getByRole('button',{name:'Settings',exact:true}).click();
  results.push({dialog:'settings',violations:await page.evaluate(async()=>(await axe.run(document)).violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)})))});
  await page.screenshot({path:'output/playwright/bold-settings-dark.png'});
  return results;
}

