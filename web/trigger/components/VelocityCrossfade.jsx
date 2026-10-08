import {SelectorModeBar} from './SelectorModeBar.jsx';
import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';
export function VelocityCrossfade({model}){return <>{model.isVel && <>
              <div style={{"display": "flex","flexDirection": "column","gap": "6px"}}>
                <div onPointerDown={model.onVelDown} style={{"position": "relative","display": "grid","gridTemplateRows": "28px 28px","gap": "2px","padding": "3px 0","background": "var(--dd-ink-0)","borderRadius": "2px","border": "1px solid var(--dd-line-1)","touchAction": "none","userSelect": "none"}}>
                  <div style={{"position": "relative"}}>
                    <svg viewBox="0 0 126 100" preserveAspectRatio="none" style={{"position": "absolute","inset": "0","width": "100%","height": "100%","overflow": "visible"}}>
                      <defs ><linearGradient id="vgHard" gradientUnits="userSpaceOnUse" x1={model.vXa} y1="0" x2={model.vXb} y2="0">{(model.vHardStops || []).map((s,index) => <React.Fragment key={s.id ?? s.key ?? index}><stop offset={s.o} stopColor="var(--dd-vermilion)" stopOpacity={s.a}/></React.Fragment>)}</linearGradient></defs>
                      <polygon points={model.vHardPts} fill="url(#vgHard)" style={{"stroke": "var(--dd-vermilion)","strokeWidth": "1px","vectorEffect": "non-scaling-stroke"}}/>
                    </svg>
                    <span style={{"position": "absolute","right": "8px","top": "7px","font": "600 12px var(--font-ui)","color": model.vHardTc,"whiteSpace": "nowrap"}}>{model.vHardLabel}</span>
                  </div>
                  <div style={{"position": "relative"}}>
                    <svg viewBox="0 0 126 100" preserveAspectRatio="none" style={{"position": "absolute","inset": "0","width": "100%","height": "100%","overflow": "visible"}}>
                      <defs ><linearGradient id="vgSoft" gradientUnits="userSpaceOnUse" x1={model.vXa} y1="0" x2={model.vXb} y2="0">{(model.vSoftStops || []).map((s,index) => <React.Fragment key={s.id ?? s.key ?? index}><stop offset={s.o} stopColor="var(--dd-velocity-soft-fill)" stopOpacity={s.a}/></React.Fragment>)}</linearGradient></defs>
                      <polygon points={model.vSoftPts} fill="url(#vgSoft)" style={{"stroke": "var(--dd-velocity-soft-line)","strokeWidth": "1px","vectorEffect": "non-scaling-stroke"}}/>
                    </svg>
                    <span style={{"position": "absolute","left": "8px","top": "7px","font": "600 12px var(--font-ui)","color": "var(--dd-paper-1)","whiteSpace": "nowrap"}}>{model.vSoftLabel}</span>
                  </div>
                  <div style={{"position": "absolute","top": "0","bottom": "0","left": (model.vBandL) + "%","width": (model.vBandW) + "%","boxSizing": "border-box","borderLeft": "1px dashed var(--dd-vermilion)","borderRight": "1px dashed var(--dd-vermilion)","background": model.vBandBg,"pointerEvents": "none"}}></div>
                  <div data-vh="l" title="Drag to set where Hard starts fading in" style={{"position": "absolute","top": "-3px","bottom": "-3px","left": "calc(" + (model.vBandL) + "% - 7px)","width": "8px","cursor": "ew-resize","display": "flex","alignItems": "center","justifyContent": "flex-end"}}><span style={{"width": "4px","height": "18px","borderRadius": "1px","background": "var(--dd-vermilion)"}}></span></div>
                  <div data-vh="r" title="Drag to set where Soft finishes fading out" style={{"position": "absolute","top": "-3px","bottom": "-3px","left": "calc(" + (model.vBandR) + "% - 1px)","width": "8px","cursor": "ew-resize","display": "flex","alignItems": "center","justifyContent": "flex-start"}}><span style={{"width": "4px","height": "18px","borderRadius": "1px","background": "var(--dd-vermilion)"}}></span></div>
                  <div data-vh="c" title="Drag to move the split" style={{"position": "absolute","top": "-3px","bottom": "-3px","left": "calc(" + (model.vMid) + "% - 5px)","width": "10px","cursor": "col-resize","display": "flex","justifyContent": "center"}}><span style={{"width": "2px","height": "100%","background": "var(--dd-paper-1)"}}></span></div>
                  <div style={{"position": "absolute","left": (model.vLast) + "%","top": "0","bottom": "0","width": "1px","background": "var(--dd-paper-3)","pointerEvents": "none"}}></div>
                </div>
                <div style={{"display": "flex","justifyContent": "space-between","font": "500 11px var(--font-value)","color": "var(--dd-paper-3)"}}><span >1</span><span >{model.vAxis}</span><span >127</span></div>
                <div style={{"display": "flex","alignItems": "center","gap": "12px","flexWrap": "wrap"}}>
                  <span style={{"font": "600 12px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Crossfade</span>
                  <span style={{"font": "500 13px var(--font-value)","color": "var(--dd-paper-1)"}}>{model.vXfText}</span>
                  <DD.NumericField label="Width" min={0} max={60} value={model.vXfW} unit="vel" width={72} compact={true} onChange={model.setVelW}></DD.NumericField>
                  <DD.SegmentedControl compact={true} value={model.vCurve} onChange={model.setVelCurve} options={model.vCurveOpts}></DD.SegmentedControl>
                  <DD.Button size="sm" variant="ghost" onClick={model.velNoXf} disabled={model.vNoXf}>No crossfade</DD.Button>
                  <span style={{"flex": "1","minWidth": "160px","font": "500 12px/1.35 var(--font-ui)","color": "var(--dd-paper-3)"}}>Drag the ember edges to widen the crossfade, the white line to move the split.</span>
                </div>
              </div>
            </>}</>;}
