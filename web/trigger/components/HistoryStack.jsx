import {TypeTag} from './TypeTag.jsx';
import {Breadcrumb} from './Breadcrumb.jsx';
import React from 'react';
import {PropertyEditor} from './PropertyEditor.jsx';
import * as DD from './design-system/index.jsx';
export function HistoryStack({model}){return <>{model.hasHist && <>
        <div style={{"flex": "none","borderBottom": "1px solid var(--dd-line-1)","background": "var(--dd-ink-2)"}}>
          <div style={{"height": "28px","display": "flex","alignItems": "center","gap": "6px","padding": "0 6px 0 12px"}}>
            <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>History</span>
            <span style={{"font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}>{model.histCount}</span>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Non-destructive op stack</span></>}
            <div style={{"flex": "1"}}></div>
            <DD.IconButton icon="plus" label="Add operation…" size="sm" variant="ghost" onClick={model.addHistory}></DD.IconButton>
            <DD.IconButton icon="layers" label="Collapse stack to a new sample" size="sm" variant="ghost" disabled={model.noCollapseHistory} onClick={model.collapseHistory}></DD.IconButton>
          </div>
          <div style={{"padding": "0 4px 6px","display": "flex","flexDirection": "column","gap": "1px"}}>
            {(model.hist || []).map((o,index) => <React.Fragment key={o.id ?? o.key ?? index}>
              <div data-ctx={"op|" + (o.id)} role="button" tabIndex="0" draggable={!o.locked} onDragStart={o.drag} onDragOver={o.dragOver} onDrop={o.drop} onKeyDown={e=>{if(e.key==='Enter'){e.preventDefault();o.select();}}} onClick={o.select} style={{"display": "flex","alignItems": "center","gap": "6px","height": "24px","padding": "0 6px","borderRadius": "3px","cursor": "pointer","background": o.bg,"boxShadow": o.edge}} className="source-hover-5">
                <span style={{"width": "10px","font": "700 11px var(--font-value)","letterSpacing": "-1px","color": "var(--dd-paper-4)"}}>{o.grip}</span>
                <div role="checkbox" tabIndex={o.locked?-1:0} aria-label={o.name} aria-disabled={o.locked} onKeyDown={e=>{if(e.key===' '){e.preventDefault();o.toggle(e);}}} aria-checked={o.on} title={o.tTip} onClick={o.toggle} style={{"width": "14px","height": "14px","flex": "none","boxSizing": "border-box","border": "1px solid var(--dd-line-3)","borderRadius": "2px","display": "flex","alignItems": "center","justifyContent": "center","background": "var(--dd-ink-0)","opacity": o.tOp}}>
                  {o.on && <><DD.Icon name="ok" size={11} color="var(--dd-paper-1)"></DD.Icon></>}
                </div>
                <DD.Icon name={o.icon} size={14} color={o.ic}></DD.Icon>
                <span style={{"flex": "none","font": "600 13px var(--font-ui)","color": o.tc}}>{o.name}</span>
                <span style={{"flex": "1","minWidth": "0","overflow": "hidden","textOverflow": "ellipsis","whiteSpace": "nowrap","textAlign": "right","font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}>{o.detail}</span>
              </div>
            </React.Fragment>)}
            <div style={{"padding": "4px 6px 0","font": "500 11px/1.35 var(--font-ui)","color": "var(--dd-paper-3)"}}>{model.histNote}</div>
          </div>
        </div>
      </>}</>;}
