import {SliceTable} from './SliceTable.jsx';
import {SliceWaveform} from './SliceWaveform.jsx';
import React from 'react';
import * as DD from './design-system/index.jsx';
export function AnalysisLane({model}){return <><div style={{"position": "relative","height": "28px","background": "var(--dd-ink-0)","borderRadius": "2px","border": "1px solid var(--dd-line-1)"}}>
              <span style={{"position": "absolute","left": "8px","top": "6px","font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-4)"}}>Transients</span>
              {model.trDone && <>
                {(model.transients || []).map((t,index) => <React.Fragment key={t.id ?? t.key ?? index}>
                  <div style={{"position": "absolute","left": (t.x) + "%","bottom": "0","width": "2px","height": (t.h) + "px","background": t.c,"borderRadius": "1px 1px 0 0"}}></div>
                </React.Fragment>)}
              </>}
              {model.trRun && <>
                <div style={{"position": "absolute","left": "0","top": "0","bottom": "0","width": model.analysisProgress*100+"%","background": "var(--dd-ink-3)"}}></div>
                <div style={{"position": "absolute","left": model.analysisProgress*100+"%","top": "0","bottom": "0","width": "1px","background": "var(--dd-paper-2)"}}></div>
              </>}
            </div></>;}
