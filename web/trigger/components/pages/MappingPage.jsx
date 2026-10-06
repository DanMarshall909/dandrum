import {ZoneLane} from '../ZoneLane.jsx';
import React from 'react';
import * as DD from '../design-system/index.jsx';
export function MappingPage({model}) { return <>{model.isMapping && <>
        <section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px"}}>
          <div style={{"minHeight": "34px","display": "flex","alignItems": "center","flexWrap": "wrap","gap": "6px","padding": "4px 8px 4px 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
            <span style={{"font": "700 13px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Key map</span>
            <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>{model.mappingSummary}</span>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>KeyMap = editor for note × velocity Trigger Rules</span></>}
            <div style={{"flex": "1"}}></div>
            <DD.Button size="sm" variant="ghost" disabled={model.noZone} onClick={model.mapByRoot}>Map by root</DD.Button>
            <DD.Button size="sm" variant="ghost" disabled={model.noZone} onClick={model.mapVelocity}>Velocity layers</DD.Button>
            <DD.Button size="sm" variant="ghost" disabled={model.noZone} onClick={model.mapSequential}>Sequential</DD.Button>
            <div style={{"width": "1px","height": "18px","background": "var(--dd-line-2)"}}></div>
            <DD.IconButton icon="layers" onClick={model.duplicateZone} disabled={model.noZone} label="Duplicate zone (Ctrl+D)" size="sm" variant="ghost"></DD.IconButton>
            <DD.IconButton icon="close" onClick={model.deleteZone} disabled={model.noZone} label="Delete zone (Delete)" size="sm" variant="ghost"></DD.IconButton>
          </div>
          <ZoneLane model={model}/>
          <div style={{"display": "grid","gridTemplateColumns": "repeat(auto-fit,minmax(180px,1fr))","gap": "1px","background": "var(--dd-line-1)","borderTop": "1px solid var(--dd-line-1)","borderRadius": "0 0 6px 6px","overflow": "hidden"}}>
            {(model.dropRules || []).map((d,index) => <React.Fragment key={d.id ?? d.key ?? index}>
              <div style={{"background": "var(--dd-ink-2)","padding": "8px 12px","display": "flex","flexDirection": "column","gap": "2px"}}>
                <span style={{"font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)"}}>{d.k}</span>
                <span style={{"font": "500 12px/1.35 var(--font-ui)","color": "var(--dd-paper-2)"}}>{d.v}</span>
              </div>
            </React.Fragment>)}
          </div>
        </section>
      </>}</>; }
