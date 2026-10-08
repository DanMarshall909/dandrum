import React from 'react';
import * as DD from './design-system/index.jsx';

export function WorkspaceTabs({model}) {
  return (<nav style={{"height": "34px","flex": "none","display": "flex","alignItems": "center","gap": "8px","padding": "0 4px 0 0","background": "var(--dd-ink-1)","borderBottom": "1px solid var(--dd-line-1)"}}>
    <DD.Tabs items={model.tabs} value={model.page} onChange={model.goPage} compact={model.isMin}></DD.Tabs>
    <div style={{"flex": "1"}}></div>
    <DD.Button size="sm" variant="ghost" icon="folder" selected={model.browser} onClick={model.toggleBrowser}>{model.browserLabel}</DD.Button>
  </nav>);
}
