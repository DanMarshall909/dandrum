import React from 'react';
import {PropertyEditor} from './PropertyEditor.jsx';
import {EnvelopeEditor} from './EnvelopeEditor.jsx';
import * as DD from './design-system/index.jsx';
export function VoiceChain({model}){return <><section style={{"display": "flex","alignItems": "center","gap": "6px","flexWrap": "wrap","padding": "6px 8px 6px 12px","background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px"}}>
          <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Voice · Keys</span>
          {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Voice Template (curated view)</span></>}
          <div style={{"flex": "1"}}></div>
          {(model.chain || []).map((c,index) => <React.Fragment key={c.id ?? c.key ?? index}>
            <div style={{"display": "flex","alignItems": "center","gap": "6px"}}>
              <button onClick={c.select} style={{"height": "24px","display": "flex","alignItems": "center","padding": "0 8px","borderRadius": "4px","background": c.bg,"border": "1px solid " + (c.bd),"font": "600 12px var(--font-ui)","color": "var(--dd-paper-1)"}}>{c.label}</button>
              {c.arrow && <><span style={{"color": "var(--dd-paper-4)","font": "500 12px var(--font-value)"}}>{c.sep}</span></>}
            </div>
          </React.Fragment>)}
          <DD.Button size="sm" variant="ghost" onClick={model.editTemplate}>Edit template…</DD.Button>
        </section></>;}
