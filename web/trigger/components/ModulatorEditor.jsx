import {ShapePreview} from './ShapePreview.jsx';
import {ModulationTable} from './ModulationTable.jsx';
import React,{useSyncExternalStore} from 'react';
import {AmountBar} from './AmountBar.jsx';
import * as DD from './design-system/index.jsx';
export function ModulatorEditor({model}){return <><section style={{minWidth:0,"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px"}}>
          <div style={{"minHeight": "30px","display": "flex","alignItems": "center","flexWrap": "wrap","gap": "4px 8px","padding": "4px 8px 4px 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
            <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Modulator</span>
            {model.me.slot && <><DD.ModGlyph slot={model.me.slot} size={9}></DD.ModGlyph></>}
            <span style={{"font": "600 13px var(--font-ui)","color": "var(--dd-paper-1)","whiteSpace": "nowrap"}}>{model.me.name}</span>
            <span style={{"font": "500 11px var(--font-value)","color": "var(--dd-paper-3)","whiteSpace": "nowrap"}}>→ {model.me.dest} · {model.me.amt} · {model.me.pol}</span>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Modulator editor (shape) + Route transform (amount, polarity)</span></>}
            <DD.Button size="sm" variant="ghost" icon="plus" onClick={model.me.addDest}>Add destination</DD.Button>
            <div style={{"flex": "1"}}></div>
            <DD.SegmentedControl compact={true} value={model.me.shape} onChange={model.me.setShape} options={model.me.shapes}></DD.SegmentedControl>
          </div>
          <div style={{"display": "flex","flexWrap": "wrap","gap": "8px 12px","padding": "8px 12px 8px","alignItems": "center"}}>
            <ShapePreview model={model}/>
            <div style={{"display": "flex","flexWrap": "wrap","gap": "0"}}>
              {(model.me.knobs || []).map((k,index) => <React.Fragment key={k.id ?? k.key ?? index}>
                <div data-knob={k.key} data-reset={"knob|" + (k.key)} style={{"borderRadius": "4px","boxShadow": k.ring}}><DD.Knob {...model.kTight} size="sm" value={k.v} defaultValue={k.default} parseValue={k.parse} onCommit={k.commit} onChange={k.set} bipolar={k.bi} label={k.label} valueText={k.t} modulations={k.mods} assigning={model.assigning}></DD.Knob></div>
              </React.Fragment>)}
            </div>
          </div>
          <div style={{"display": "flex","flexWrap": "wrap","alignItems": "center","gap": "6px 16px","padding": "0 12px 8px"}}>
            {(model.me.toggles || []).map((t,index) => <React.Fragment key={t.id ?? t.key ?? index}>
              <DD.Toggle compact={true} label={t.label} checked={t.on} onChange={t.change}></DD.Toggle>
            </React.Fragment>)}
            <div style={{"flex": "1"}}></div>
            <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>Dashed: full-range shape · solid: what reaches {model.me.dest} after amount and polarity</span>
          </div>
        </section></>;}
