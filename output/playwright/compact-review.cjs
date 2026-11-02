async page => {
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.reload();
  await page.getByText('Local storage ready').waitFor();
  await page.addScriptTag({path:'node_modules/axe-core/axe.min.js'});
  const results=[];
  for(const size of [{width:420,height:480},{width:360,height:420},{width:420,height:640}]) {
    await page.setViewportSize(size);
    for(const theme of ['dark','light']) {
      if(theme==='light') await page.getByRole('button',{name:'Use light theme'}).click();
      results.push({...size,theme,...await page.evaluate(async()=>({listHeight:Math.round(document.querySelector('.items').getBoundingClientRect().height),overflow:document.documentElement.scrollWidth>innerWidth,violations:(await axe.run(document)).violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)}))}))});
      await page.screenshot({path:`output/playwright/compact-${theme}-${size.width}-${size.height}.png`});
      if(theme==='light') await page.getByRole('button',{name:'Use dark theme'}).click();
    }
  }
  return results;
}
