import React from 'react';
import * as DD from './design-system/index.jsx';
export function ModulatorList({model}){return <>
        <div style={{"height": "30px","flex": "none","display": "flex","alignItems": "center","gap": "6px","padding": "0 8px 0 12px","background": "var(--dd-ink-3)","borderTop": "1px solid var(--dd-line-1)","borderBottom": "1px solid var(--dd-line-1)"}}>
          <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Modulators</span>
          {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Modulators</span></>}
          <div style={{"flex": "1"}}></div>
          <DD.IconButton icon="plus" label="Add modulator" size="sm" variant="ghost" onClick={model.addModulator}></DD.IconButton>
        </div>
        <div style={{"flex": "0 1 auto","maxHeight": "24%","minHeight": "0","overflow": "auto","padding": "4px 0"}}>
                {(model.modSrcs || []).map((s,index) => <React.Fragment key={s.id ?? s.key ?? index}>
                  <div role="button" tabIndex="0" aria-expanded={s.open} onKeyDown={e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();s.toggle();}}} onClick={s.toggle} style={{"display": "flex","alignItems": "center","gap": "6px","height": "26px","padding": "0 12px 0 6px","cursor": "pointer","background": s.hbg}} className="source-hover-2">
                    <DD.Icon name={s.chev} size={12} color="var(--dd-paper-3)"></DD.Icon>
                    <span style={{"flex": "1","minWidth": "0","overflow": "hidden","textOverflow": "ellipsis","font": "600 12px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-2)","whiteSpace": "nowrap"}}>{s.cat}</span>
                    <span style={{"display": "flex","gap": "3px","alignItems": "center"}}>
                      {(s.slots || []).map((g,index) => <React.Fragment key={g.id ?? g.key ?? index}><DD.ModGlyph slot={g} size={8}></DD.ModGlyph></React.Fragment>)}
                    </span>
                    <span title={s.title} style={{"flex": "none","whiteSpace": "nowrap","font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}>{s.summary}</span>
                  </div>
                  {s.open && <>
                  {(s.items || []).map((s,index) => <React.Fragment key={s.id ?? s.key ?? index}>
                  <div data-ctx={"mod|" + (s.id??s.name)} role="button" tabIndex={0} onKeyDown={e=>{if(e.key==='Enter')s.pick();}} draggable={true} onDragStart={s.drag} onDragEnd={model.endModDrag} onClick={s.pick} title={s.tip} style={{"display": "flex","alignItems": "center","gap": "8px","height": "26px","padding": "0 12px 0 24px","cursor": "grab","background": s.bg,"boxShadow": s.edge}} className="source-hover-3">
                    <span style={{"width": "10px","display": "flex","justifyContent": "center"}}>
                      {s.slot && <><DD.ModGlyph slot={s.slot} size={9}></DD.ModGlyph></>}
                      {s.noSlot && <><span style={{"width": "7px","height": "7px","border": "1px solid var(--dd-paper-3)","borderRadius": "1px"}}></span></>}
                    </span>
                    <span style={{"flex": "1","font": "500 13px var(--font-ui)","color": "var(--dd-paper-1)","whiteSpace": "nowrap"}}>{s.name}</span>
                    <span style={{"font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}>{s.n}</span>
                    <span style={{"width": "24px","height": "3px","background": "var(--dd-ink-4)","borderRadius": "2px"}}><span style={{"display": "block","width": (s.live) + "%","height": "3px","background": "var(--dd-paper-2)","borderRadius": "2px"}}></span></span>
                  </div>
                  </React.Fragment>)}
                  </>}
                </React.Fragment>)}
        </div>
</>;}
