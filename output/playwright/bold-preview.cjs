async page => {
  await page.addInitScript(() => {
    const items = [
      {id:'preview-1', pinned:true, itemType:'text', content:'Make room for the ideas\nthat arrive unannounced.', formattedHtml:null, image:null, createdAt:'2026-11-02T05:00:00Z', categoryId:'ideas'},
      {id:'preview-2', pinned:false, itemType:'link', content:'https://www.are.na/explore', formattedHtml:null, image:null, createdAt:'2026-11-01T05:00:00Z', categoryId:'ideas'},
      {id:'preview-3', pinned:false, itemType:'text', content:'بعضی ایده‌ها فقط کمی زمان می‌خواهند.\nبرای روزی که فرصت باشد.', formattedHtml:null, image:null, createdAt:'2026-11-01T04:00:00Z', categoryId:null}
    ];
    window.__TAURI_INTERNALS__ = {
      metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},
      transformCallback: () => 1, unregisterCallback: () => {},
      invoke: async (cmd, args) => {
        if(cmd === 'get_foundation_status') return {theme:'dark',pendingCount:3,schemaVersion:8,launchAtLogin:false,autoSaveImages:false,autoSaveImagesAvailable:true,shortcutAvailable:true,toggleShortcutAvailable:true,shortcuts:{capture:'Ctrl+Shift+S',toggle:'Ctrl+Shift+Space'},captureResult:null};
        if(cmd === 'list_items') return {items:args.view === 'archive' ? [] : items,categories:[{id:'ideas',name:'Ideas'}],pendingCount:3};
        if(cmd === 'load_note_draft') return {content:'',revision:0};
        if(cmd === 'plugin:event|listen') return 1;
        return null;
      }
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener: () => {}};
  });
  await page.reload();
  await page.setViewportSize({width:420,height:760});
  await page.getByText('Local storage ready').waitFor();
  await page.screenshot({path:'output/playwright/bold-inbox-dark.png'});
}
