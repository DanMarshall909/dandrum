import React from 'react';
import * as DD from './design-system/index.jsx';
export function SourceList({model}){return <>
        <div style={{"height": "30px","flex": "none","display": "flex","alignItems": "center","gap": "6px","padding": "0 8px 0 12px","background": "var(--dd-ink-3)","borderTop": "1px solid var(--dd-line-1)","borderBottom": "1px solid var(--dd-line-1)"}}>
          <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Sources</span>
          {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Assets + Asset Regions</span></>}
          <div style={{"flex": "1"}}></div>
          <DD.IconButton icon="plus" label="Add source…" size="sm" variant="ghost" onClick={model.browseSamples}></DD.IconButton>
        </div>
        <div style={{"flex": "0 1 auto","maxHeight": "26%","minHeight": "0","overflow": "auto","padding": "4px 0"}}>
          {(model.srcGroups || []).map((g,index) => <React.Fragment key={g.id ?? g.key ?? index}>
            <div role="button" tabIndex="0" aria-expanded={g.open} onKeyDown={e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();g.toggle();}}} onClick={g.toggle} style={{"display": "flex","alignItems": "center","gap": "6px","height": "26px","padding": "0 12px 0 6px","cursor": "pointer","background": g.hbg}} className="source-hover-0">
              <DD.Icon name={g.chev} size={12} color="var(--dd-paper-3)"></DD.Icon>
              <span style={{"flex": "1","minWidth": "0","overflow": "hidden","textOverflow": "ellipsis","whiteSpace": "nowrap","font": "600 12px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>{g.name}</span>
              <span style={{"flex": "none","whiteSpace": "nowrap","font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}>{g.summary}</span>
            </div>
            {g.open && <>
              {(g.items || []).map((i,index) => <React.Fragment key={i.id ?? i.key ?? index}>
                <div data-ctx={"src|" + (i.id)} role="button" tabIndex={0} onKeyDown={e=>{if(e.key==='Enter')i.play();}} draggable={true} onDragStart={i.drag} onDragEnd={model.endSrcDrag} onClick={i.play} title="Click to preview · drag onto a pad, layer or zone" style={{"position": "relative","display": "flex","alignItems": "center","gap": "8px","height": "26px","padding": "0 10px 0 24px","cursor": "grab","background": i.bg,"boxShadow": i.edge}} className="source-hover-1">
                  <DD.Icon name={i.icon} size={14} color={i.ic}></DD.Icon>
                  <span style={{"flex": "1","minWidth": "0","overflow": "hidden","textOverflow": "ellipsis","whiteSpace": "nowrap","font": "500 13px var(--font-ui)","color": "var(--dd-paper-1)"}}>{i.name}</span>
                  <span style={{"flex": "none","font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}>{i.detail}</span>
                  {i.playing && <><div style={{"position": "absolute","left": "24px","right": "10px","bottom": "2px","height": "2px","background": "var(--dd-ink-4)","borderRadius": "1px"}}><div style={{"width": (i.pct) + "%","height": "2px","background": "var(--dd-paper-1)","borderRadius": "1px"}}></div></div></>}
                </div>
              </React.Fragment>)}
            </>}
          </React.Fragment>)}
        </div>
</>;}
