import {VoiceSection} from './VoiceSection.jsx';
import {VoiceChain} from './VoiceChain.jsx';
import React from 'react';
import {PropertyEditor} from './PropertyEditor.jsx';
import {EnvelopeEditor} from './EnvelopeEditor.jsx';
import * as DD from './design-system/index.jsx';
export function VoicePolicyPanel({model}){return <><section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","minWidth": "0"}}>
            <div style={{"height": "30px","display": "flex","alignItems": "center","gap": "6px","padding": "0 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
              <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Voices</span>
              {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px"}}>Voice Policy</span></>}
            </div>
            <div style={{"padding": "10px 12px 12px","display": "flex","flexDirection": "column","gap": "8px"}}>
              <DD.SegmentedControl compact={true} fullWidth={true} value={model.voiceMode} onChange={model.setVoiceMode} options={model.polyOpts}></DD.SegmentedControl>
              {model.voicePolicyRows.map(row=><PropertyEditor key={row.label} row={row}/>)}
            </div>
          </section></>;}
