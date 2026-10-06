import React from 'react';
import {OutputBusPanel} from './OutputBusPanel.jsx';
import {RoutingGraph} from './RoutingGraph.jsx';
import * as DD from './design-system/index.jsx';
export function RoutingTable({model}){return <><section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","minWidth": "0"}}>
            <div style={{"height": "34px","display": "flex","alignItems": "center","gap": "8px","padding": "0 8px 0 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
              <span style={{"font": "700 13px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Routing</span>
              {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Each menu = Connection → Bus; each knob = send Connection gain</span></>}
              <div style={{"flex": "1"}}></div>
              <DD.Button size="sm" variant="ghost" onClick={model.toggleRoutingGraph}>{model.showRoutingGraph?"Hide graph":"Show graph"}</DD.Button>
            </div>
            {model.showRoutingGraph&&<RoutingGraph rows={model.routeRows} busses={model.busses}/>}
            <div style={{"display": "grid","gridTemplateColumns": "minmax(110px,1fr) 130px 40px 40px","alignItems": "center","columnGap": "8px","padding": "0 12px","height": "26px","font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)","borderBottom": "1px solid var(--dd-line-1)"}}>
              <span >Source</span><span >Output bus</span><span style={{"textAlign": "center"}}>Reverb</span><span style={{"textAlign": "center"}}>Delay</span>
            </div>
            {(model.routeRows || []).map((r,index) => <React.Fragment key={r.id ?? r.key ?? index}>
              <div onClick={r.select} style={{"display": "grid","gridTemplateColumns": "minmax(110px,1fr) 130px 40px 40px","alignItems": "center","columnGap": "8px","padding": "3px 12px","background": r.bg,"boxShadow": r.edge}}>
                <span style={{"display": "flex","flexDirection": "column","minWidth": "0","paddingLeft": (r.pad) + "px"}}><span style={{"font": "500 13px var(--font-ui)","color": "var(--dd-paper-1)","whiteSpace": "nowrap","overflow": "hidden","textOverflow": "ellipsis"}}>{r.name}</span><span style={{"font": "500 11px var(--font-ui)","color": "var(--dd-paper-3)"}}>{r.kind}</span></span>
                <DD.MenuButton label={"Output for "+r.name} showLabel={false} value={r.bus} options={r.options} onChange={r.setBus} compact={true} width={130}></DD.MenuButton>
                <span style={{"display": "flex","justifyContent": "center"}}><DD.Knob {...model.kTight} size="xs" value={r.sa} defaultValue={0} onChange={v=>r.setSend("rev",v)} label={r.name+" Reverb send"} showLabel={false}></DD.Knob></span>
                <span style={{"display": "flex","justifyContent": "center"}}><DD.Knob {...model.kTight} size="xs" value={r.sb} defaultValue={0} onChange={v=>r.setSend("dly",v)} label={r.name+" Delay send"} showLabel={false}></DD.Knob></span>
              </div>
            </React.Fragment>)}
            <div style={{"padding": "8px 12px","borderTop": "1px solid var(--dd-line-1)","font": "500 12px/1.35 var(--font-ui)","color": "var(--dd-paper-3)"}}>Inherit means the row follows its parent group. Setting a bus on a child overrides it; the row then shows its own bus in full text.</div>
          </section></>;}
