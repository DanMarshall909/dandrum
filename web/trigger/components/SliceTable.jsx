import {SliceWaveform} from './SliceWaveform.jsx';
import React from 'react';
import * as DD from './design-system/index.jsx';
export function SliceTable({model}){return <><section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px"}}>
          <div style={{"minHeight": "30px","display": "flex","alignItems": "center","flexWrap": "wrap","gap": "8px","padding": "2px 8px 2px 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
            <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Slice list</span>
            <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>{model.sliceSelectionCount} selected</span>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Asset Region + Trigger Rule (note) per row</span></>}
            <div style={{"flex": "1"}}></div>
            <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>Map from</span>
            <DD.NumericField showLabel={false} label="Map from note" value={model.sliceMapFrom} min={0} max={128-model.sliceTotal} onChange={model.setSliceMapFrom} format={model.fmtNote} width={72} compact={true}></DD.NumericField>
            <DD.Button size="sm" icon="keyboard" onClick={model.mapSlices}>Map sequentially</DD.Button>
          </div>
          <div style={{"display": "grid","gridTemplateColumns": "24px minmax(60px,1.4fr) minmax(45px,.8fr) minmax(60px,1fr) minmax(36px,.6fr) minmax(36px,.6fr) minmax(30px,.5fr) minmax(40px,.6fr) minmax(34px,.5fr) minmax(50px,1fr)","alignItems": "center","columnGap": "4px","padding": "0 12px","height": "26px","font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)","borderBottom": "1px solid var(--dd-line-1)"}}>
            <span >#</span><span >Name</span><span >Note</span><span >Range</span><span >Tune</span><span >Gain</span><span >Pan</span><span >Play</span><span >Choke</span><span >Output</span>
          </div>
          {(model.sliceRows || []).map((s,index) => <React.Fragment key={s.id ?? s.key ?? index}>
            <div data-ctx={"slice|" + s.id} role="button" tabIndex={0} onKeyDown={e=>{if(e.key==='Enter')s.play(e);}} draggable={true} onDragStart={s.drag} onDragEnd={model.endSrcDrag} onClick={s.play} title="Click to preview · drag onto a pad, layer or zone" style={{"cursor": "grab","display": "grid","gridTemplateColumns": "24px minmax(60px,1.4fr) minmax(45px,.8fr) minmax(60px,1fr) minmax(36px,.6fr) minmax(36px,.6fr) minmax(30px,.5fr) minmax(40px,.6fr) minmax(34px,.5fr) minmax(50px,1fr)","alignItems": "center","columnGap": "4px","padding": "0 12px","height": "26px","background": s.bg,"boxShadow": s.edge,"font": "500 13px var(--font-ui)","color": "var(--dd-paper-1)"}}>
              <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-3)"}}>{s.n}</span>
              <span >{s.name}</span>
              <span style={{"font": "500 12px var(--font-value)"}}>{s.note}</span>
              <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-2)"}}>{s.range}</span>
              <span style={{"font": "500 12px var(--font-value)"}}>{s.tune}</span>
              <span style={{"font": "500 12px var(--font-value)"}}>{s.gain}</span>
              <span style={{"font": "500 12px var(--font-value)"}}>{s.pan}</span>
              <span style={{"color": "var(--dd-paper-2)"}}>{s.mode}</span>
              <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-2)"}}>{s.choke}</span>
              <span style={{"color": "var(--dd-paper-2)"}}>{s.out}</span>
            </div>
          </React.Fragment>)}
        </section></>;}
