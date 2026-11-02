async page => {
  await page.reload();
  const search=page.getByRole('searchbox',{name:'Search saved items'});
  await search.fill('متن');
  await page.getByRole('button',{name:'Settings',exact:true}).focus();
  await page.keyboard.press('Control+f');
  return await search.evaluate(input=>({focused:document.activeElement===input,value:input.value,selection:[input.selectionStart,input.selectionEnd]}));
}
