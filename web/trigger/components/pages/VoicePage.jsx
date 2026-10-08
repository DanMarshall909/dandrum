import {VoicePolicyPanel} from '../VoicePolicyPanel.jsx';
import {VoiceSection} from '../VoiceSection.jsx';
import {VoiceChain} from '../VoiceChain.jsx';
import React from 'react';
import {PropertyEditor} from '../PropertyEditor.jsx';
import {EnvelopeEditor} from '../EnvelopeEditor.jsx';
import * as DD from '../design-system/index.jsx';
export function VoicePage({model}) { return <>{model.isVoice && <>
        <VoiceChain model={model}/>
        <div style={{"display": "grid","gridTemplateColumns": model.voiceCols,"gap": "4px"}}>
          {(model.voiceSecs || []).map((v,index) => <React.Fragment key={v.id ?? v.key ?? index}>
            <VoiceSection model={model} section={v}/>
          </React.Fragment>)}
        </div>
        <div style={{"display": "grid","gridTemplateColumns": model.envCols,"gap": "4px"}}>
          {(model.envs || []).map((e,index) => <React.Fragment key={e.id ?? e.key ?? index}>
            <section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","minWidth": "0"}}>
              <div style={{"minHeight": "30px","display": "flex","alignItems": "center","flexWrap": "wrap","gap": "4px 6px","padding": "4px 8px 4px 12px","boxSizing": "border-box","overflow": "hidden","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
                <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)","whiteSpace": "nowrap"}}>{e.title}</span>
                {e.slot && <><DD.ModGlyph slot="A" size={9}></DD.ModGlyph></>}
                <span style={{"font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}>{e.dest}</span>
                <div style={{"flex": "1"}}></div>
                <DD.SegmentedControl compact={true} value={e.shape} onChange={e.setShape} options={model.envShapes}></DD.SegmentedControl>
              </div>
              {model.ann && <><div style={{"padding": "6px 12px 0"}}><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px"}}>{e.ann}</span></div></>}
              <div style={{"padding": "10px 12px 12px","display": "flex","flexDirection": "column","gap": "8px"}}>
                <EnvelopeEditor envelope={e} height={model.envH} getTelemetry={model.getTelemetry} subscribeTelemetry={model.subscribeTelemetry}/>
                <div style={{"display": "grid","gridTemplateColumns": "repeat(4,minmax(0,1fr))","gap": "8px"}}>
                  {(e.vals || []).map((x,index) => <React.Fragment key={x.id ?? x.key ?? index}>
                    <div style={{"display": "flex","flexDirection": "column","gap": "2px"}}><span style={{"font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)"}}>{x.k}</span><DD.NumericField showLabel={false} label={e.title+" "+x.k} value={x.value} min={x.min} max={x.max} step={x.step} format={x.format} unit={x.unit} onChange={x.change} compact width={64}/></div>
                  </React.Fragment>)}
                </div>
              </div>
            </section>
          </React.Fragment>)}
          <VoicePolicyPanel model={model}/>
        </div>
      </>}</>; }
