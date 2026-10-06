import React from 'react';
import * as DD from './design-system/index.jsx';
export function InstrumentTree({model}){return <>
        <div style={{"height": "30px","flex": "none","display": "flex","alignItems": "center","gap": "6px","padding": "0 8px 0 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)"}}>
          <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Instrument</span>
          {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Group tree</span></>}
          <div style={{"flex": "1"}}></div>
          <DD.IconButton icon="plus" label="Add group" size="sm" variant="ghost" onClick={model.addGroup}></DD.IconButton>
        </div>
        <div onDragOver={model.onNodeOver} onDrop={model.onNodeDrop} style={{"flex": "1 1 0","minHeight": "0","overflow": "auto","padding": "4px 0"}}>
          {(model.tree || []).map((r,index) => <React.Fragment key={r.id ?? r.key ?? index}>
            <div data-node={r.id} data-ctx={"node|" + (r.id)} style={{"paddingLeft": (r.pad) + "px","paddingRight": "4px","borderRadius": "3px","background": r.dbg,"boxShadow": r.drop}}>
              <DD.ListRow icon={r.icon} label={r.label} detail={r.detail} selected={r.sel} active={r.active} disabled={r.dim} onClick={r.go}></DD.ListRow>
            </div>
          </React.Fragment>)}
        </div>
</>;}
