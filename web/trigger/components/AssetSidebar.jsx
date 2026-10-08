import React,{useSyncExternalStore} from 'react';
import {InstrumentTree} from './InstrumentTree.jsx';
import {SourceList} from './SourceList.jsx';
import {ModulatorList} from './ModulatorList.jsx';
import {AssetBrowser} from './AssetBrowser.jsx';
import * as DD from './design-system/index.jsx';

export function AssetSidebar({model:base}) {
  const telemetry=useSyncExternalStore(base.subscribeTelemetry,base.getTelemetry);
  const model={...base,srcGroups:base.srcGroups.map(g=>({...g,items:g.items.map(i=>{const playhead=telemetry.playheads.find(p=>p.asset===i.assetId);return {...i,playing:i.loadProgress!=null||!!playhead,pct:(i.loadProgress??playhead?.pos??0)*100};})})),
    modSrcs:base.modSrcs.map(g=>({...g,items:g.items.map(m=>({...m,live:(telemetry.mod[m.routeIds?.[0]]??0)*100}))}))};
  return (<aside style={{"minHeight": "0","display": "flex","flexDirection": "column","background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","overflow": "hidden"}}>
      {model.isMin && <>
        <div style={{"display": "flex","flexDirection": "column","alignItems": "center","gap": "4px","padding": "6px 0"}}>
          <DD.IconButton icon="layers" label="Instrument tree" size="sm" variant="ghost" selected={model.rail==='tree'} onClick={()=>model.openRail('tree')}></DD.IconButton>
          <DD.IconButton icon="folder" label="Sample browser" size="sm" variant="ghost" onClick={()=>model.openRail('browser')}></DD.IconButton>
          <DD.IconButton icon="keyboard" label="Key map" size="sm" variant="ghost" onClick={()=>model.goPage("mapping")}></DD.IconButton>
        </div>
      </>}
      {model.showTree && <>
        <InstrumentTree model={model}/><SourceList model={model}/><ModulatorList model={model}/>
        <div style={{"flex": "none","padding": "6px 12px","borderTop": "1px solid var(--dd-line-1)","font": "500 12px/1.35 var(--font-ui)","color": "var(--dd-paper-3)"}}>Click to preview a source. Drag sources onto pads or zones, modulators onto controls. Right-click a control for Modulation.</div>
      </>}
      {model.showBrowser && <>
        <AssetBrowser model={model}/>
      </>}
    </aside>);
}
