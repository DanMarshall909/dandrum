import React,{useSyncExternalStore} from 'react';
import {AmountBar} from './AmountBar.jsx';
import * as DD from './design-system/index.jsx';
export function ModulationTable({model}){return <><section style={{minWidth:0,"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px"}}>
          <div style={{"minHeight": "34px","display": "flex","alignItems": "center","flexWrap": "wrap","gap": "8px","padding": "4px 8px 4px 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
            <span style={{"font": "700 13px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Modulation</span>
            <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>{model.routeCount} routes</span>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Modulation Route: Modulator → amount/transform → Parameter</span></>}
            <div style={{"flex": "1"}}></div>
            <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>Group by</span>
            <DD.SegmentedControl compact={true} value={model.groupBy} onChange={model.setGroupBy} options={model.groupByOpts}></DD.SegmentedControl>
            <DD.Button size="sm" icon="plus" onClick={model.addModulationRoute}>Add route</DD.Button>
          </div>
          <div style={{"display": "grid","gridTemplateColumns": model.modCols,"minHeight": "0"}}>
            <div style={{"minWidth": "0"}}>
              <div style={{"display": "grid","gridTemplateColumns": "minmax(72px,1.2fr) minmax(40px,80px) 40px 30px 42px minmax(64px,1fr) 20px 28px","alignItems": "center","columnGap": "6px","padding": "0 12px","height": "26px","font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)","borderBottom": "1px solid var(--dd-line-1)"}}>
                <span >Source</span><span >Amount</span><span ></span><span title="Polarity">Pol.</span><span >Curve</span><span >Destination</span><span >Live</span><span >On</span>
              </div>
              {(model.routeGroups || []).map((g,index) => <React.Fragment key={g.id ?? g.key ?? index}>
                {g.multi && <>
                  <div onClick={g.toggle} style={{"cursor": "pointer","display": "grid","gridTemplateColumns": "minmax(72px,1.2fr) minmax(40px,80px) 40px 30px 42px minmax(64px,1fr) 20px 28px","alignItems": "center","columnGap": "6px","padding": "0 12px","height": "28px","background": "var(--dd-ink-3)","borderTop": "1px solid var(--dd-line-1)"}} className="source-hover-4">
                    <span style={{"display": "flex","alignItems": "center","gap": "6px","font": "600 13px var(--font-ui)","color": "var(--dd-paper-1)","whiteSpace": "nowrap","overflow": "hidden"}}>
                      <DD.Icon name={g.chev} size={12} color="var(--dd-paper-3)"></DD.Icon>
                      {g.slot && <><DD.ModGlyph slot={g.slot} size={9}></DD.ModGlyph></>}
                      {g.noSlot && <><span style={{"width": "7px","height": "7px","border": "1px solid var(--dd-paper-3)","borderRadius": "1px","flex": "none"}}></span></>}
                      {g.name}
                    </span>
                    <span style={{"gridColumn": "2 / span 4","font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)","whiteSpace": "nowrap","overflow": "hidden","textOverflow": "ellipsis"}}>{g.summary}</span>
                    <span style={{"display": "flex","justifyContent": "flex-start"}}><DD.Button size="sm" variant="ghost" icon="plus" onClick={g.addDest}>Destination</DD.Button></span>
                    <span style={{"height": "3px","background": "var(--dd-ink-4)","borderRadius": "2px"}}><span style={{"display": "block","width": (g.live) + "%","height": "3px","background": "var(--dd-paper-2)","borderRadius": "2px"}}></span></span>
                    <span style={{"font": "500 11px var(--font-value)","color": "var(--dd-paper-3)","textAlign": "center"}}>{g.count}</span>
                  </div>
                </>}
                {g.show && <>
                {(g.items || []).map((r,index) => <React.Fragment key={r.id ?? r.key ?? index}>
                <div data-ctx={"route|" + (r.rk)} role="button" tabIndex={0} onKeyDown={e=>{if(e.key==='Enter')r.select();}} onClick={r.select} style={{"cursor": "pointer","display": "grid","gridTemplateColumns": "minmax(72px,1.2fr) minmax(40px,80px) 40px 30px 42px minmax(64px,1fr) 20px 28px","alignItems": "center","columnGap": "6px","padding": "0 12px","height": "28px","background": r.bg,"boxShadow": r.edge,"opacity": r.op}}>
                  <span style={{"display": "flex","alignItems": "center","gap": "6px","font": "500 13px var(--font-ui)","color": "var(--dd-paper-1)","whiteSpace": "nowrap","overflow": "hidden","paddingLeft": (r.indent) + "px"}}>
                    {r.child && <><span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-4)"}}>└</span></>}
                    {r.top && <><span style={{"display": "flex","alignItems": "center","gap": "6px"}}>
                    {r.slot && <><DD.ModGlyph slot={r.slot} size={9}></DD.ModGlyph></>}
                    {r.noSlot && <><span style={{"width": "7px","height": "7px","border": "1px solid var(--dd-paper-3)","borderRadius": "1px","flex": "none"}}></span></>}
                    {r.src}
                    {r.fixed && <><DD.Icon name="lock" size={12} color="var(--dd-paper-3)"></DD.Icon></>}
                    </span></>}
                  </span>
                  <AmountBar route={r}/>
                  <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-1)","textAlign": "right"}}>{r.amt}</span>
                  <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-2)"}}>{r.pol}</span>
                  <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-2)"}}>{r.curve}</span>
                  <span style={{"font": "500 13px var(--font-ui)","color": "var(--dd-paper-1)","whiteSpace": "nowrap","overflow": "hidden","textOverflow": "ellipsis"}}>→ {r.dest}</span>
                  <span style={{"height": "3px","background": "var(--dd-ink-4)","borderRadius": "2px"}}><span style={{"display": "block","width": (r.live) + "%","height": "3px","background": "var(--dd-paper-2)","borderRadius": "2px"}}></span></span>
                  <DD.Toggle compact={true} showLabel={false} label={"Enable "+r.src+" to "+r.dest} checked={r.on} disabled={r.fixed} onChange={r.toggle}></DD.Toggle>
                </div>
                </React.Fragment>)}
                </>}
              </React.Fragment>)}
            </div>
          </div>
        </section></>;}
