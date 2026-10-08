import {HistoryStack} from './HistoryStack.jsx';
import {TypeTag} from './TypeTag.jsx';
import {Breadcrumb} from './Breadcrumb.jsx';
import React from 'react';
import {PropertyEditor} from './PropertyEditor.jsx';
import * as DD from './design-system/index.jsx';

export function Inspector({model}) {
  return (<aside style={{"minWidth": "0","minHeight": "0","display": "flex","flexDirection": "column","background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","overflow": "hidden"}}>
      <div style={{"flex": "none","padding": "8px 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","display": "flex","flexDirection": "column","gap": "4px"}}>
        <Breadcrumb model={model}/>
        <div style={{"display": "flex","alignItems": "center","gap": "8px"}}>
          <DD.Icon name={model.insp.icon} size={16} color="var(--dd-vermilion)"></DD.Icon>
          <span style={{"flex": "1","font": "600 15px var(--font-ui)","color": "var(--dd-paper-1)","whiteSpace": "nowrap","overflow": "hidden","textOverflow": "ellipsis"}}>{model.insp.title}</span>
          <TypeTag model={model}/>
        </div>
        {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","alignSelf": "flex-start"}}>{model.insp.ann}</span></>}
      </div>
      <HistoryStack model={model}/>
      <div style={{"flex": "1","minHeight": "0","overflow": "auto","padding": "8px 12px 12px","display": "flex","flexDirection": "column","gap": "2px"}}>
        {(model.insp.rows || []).map((r,index) => <React.Fragment key={r.id ?? r.key ?? index}>
          {r.h && <><div style={{"padding": "10px 0 4px","font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>{r.label}</div></>}
          {r.p && <>
            <PropertyEditor row={r}/>
          </>}
          {r.m && <>
            <DD.ModIndicator slot={r.slot} depth={r.depth} source={r.label} host={r.host}></DD.ModIndicator>
          </>}
          {r.t && <><div style={{"padding": "4px 0","font": "500 12px/1.4 var(--font-ui)","color": "var(--dd-paper-3)"}}>{r.label}</div></>}
          {r.b && <><div style={{"paddingTop": "6px"}}><DD.Button size="sm" variant={r.variant} icon={r.icon} disabled={r.disabled} onClick={r.action}>{r.label}</DD.Button></div></>}
          {r.knobs && <>
            <div style={{"display": "flex","flexWrap": "wrap","gap": "0","padding": "2px 0"}}>
              {(r.list || []).map((k,index) => <React.Fragment key={k.id ?? k.key ?? index}>
                <span data-knob={k.key} data-reset={"knob|"+k.key}><DD.Knob {...model.kTight} size="sm" value={k.v} defaultValue={k.default} parseValue={k.parse} onCommit={k.commit} onChange={k.set} bipolar={k.bi} label={k.label} valueText={k.t}></DD.Knob></span>
              </React.Fragment>)}
            </div>
          </>}
        </React.Fragment>)}
      </div>
    </aside>);
}
