import {VoiceChain} from './VoiceChain.jsx';
import React from 'react';
import {PropertyEditor} from './PropertyEditor.jsx';
import {EnvelopeEditor} from './EnvelopeEditor.jsx';
import * as DD from './design-system/index.jsx';
export function VoiceSection({model,section:v}){return <><section style={{"background": "var(--dd-ink-2)","border": "1px solid " + (v.bd),"borderRadius": "6px","minWidth": "0"}}>
              <div style={{"height": "24px","display": "flex","alignItems": "center","gap": "6px","padding": "0 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
                <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>{v.title}</span>
                <span style={{"font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}>{v.sub}</span>
              </div>
              {model.ann && <><div style={{"padding": "6px 12px 0"}}><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px"}}>{v.ann}</span></div></>}
              <div style={{"padding": "4px 5px 5px","display": "flex","flexDirection": "column","gap": "4px"}}>
                {v.hasSeg && <>
                  <DD.SegmentedControl compact={true} fullWidth={true} value={v.segV} options={v.seg} onChange={v.setSeg}></DD.SegmentedControl>
                </>}
                <div style={{"display": "flex","flexWrap": "wrap","gap": "0"}}>
                  {(v.knobs || []).map((k,index) => <React.Fragment key={k.id ?? k.key ?? index}>
                    <div data-knob={k.key} data-reset={"knob|" + (k.key)} style={{"borderRadius": "4px","boxShadow": k.ring}}><DD.Knob {...model.kTight} size="sm" value={k.v} defaultValue={k.default} parseValue={k.parse} onCommit={k.commit} onChange={k.set} bipolar={k.bi} label={k.label} valueText={k.t} modulations={k.mods} popupOpen={k.pop} assigning={model.assigning}></DD.Knob></div>
                  </React.Fragment>)}
                </div>
              </div>
            </section></>;}
