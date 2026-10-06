import {CycleSequence} from './CycleSequence.jsx';
import {VelocityCrossfade} from './VelocityCrossfade.jsx';
import {SelectorModeBar} from './SelectorModeBar.jsx';
import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';
export function CandidateTable({model}){return <><section onDrop={model.onLayerDrop} onDragOver={model.onFileDrag} style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px"}}>
          <div style={{"minHeight": "34px","display": "flex","alignItems": "center","flexWrap": "wrap","gap": "8px","padding": "4px 8px 4px 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
            <span style={{"font": "700 13px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Layers</span>
            <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-3)"}}>{model.layerCtx}</span>
          </div>
          <div style={{"padding": "12px","display": "flex","flexDirection": "column","gap": "12px"}}>
            <div style={{"display": "flex","alignItems": "center","gap": "8px","flexWrap": "wrap"}}>
              <span style={{"font": "600 12px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Play</span>
              <SelectorModeBar model={model}/>
              {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Selector policy</span></>}
            </div>
            <span style={{"font": "500 13px/1.35 var(--font-ui)","color": "var(--dd-paper-2)"}}>{model.modeDesc}</span>

            <VelocityCrossfade model={model}/>
            <CycleSequence model={model}/>
          </div>
          <div style={{"display": "grid","gridTemplateColumns": "14px 20px minmax(60px,1.6fr) minmax(45px,.8fr) minmax(40px,.7fr) 20px 20px minmax(35px,.6fr) minmax(25px,.5fr) minmax(30px,.5fr) minmax(50px,1fr)","alignItems": "center","columnGap": "4px","padding": "0 12px","height": "26px","font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)","borderTop": "1px solid var(--dd-line-1)","borderBottom": "1px solid var(--dd-line-1)"}}>
            <span ></span><span >#</span><span >Sample</span><span >Velocity</span><span >{model.weightHead}</span><span >M</span><span >S</span><span >Gain</span><span >Pan</span><span >Tune</span><span >Output</span>
          </div>
          {(model.cands || []).map((c,index) => <React.Fragment key={c.id ?? c.key ?? index}>
            <div data-ctx={"candidate|"+c.id} role="button" tabIndex={0} onClick={c.select} onKeyDown={e=>{if(e.key==='Enter')c.select();}} draggable onDragStart={c.drag} onDragOver={c.over} onDrop={c.drop} style={{"display": "grid","gridTemplateColumns": "14px 20px minmax(60px,1.6fr) minmax(45px,.8fr) minmax(40px,.7fr) 20px 20px minmax(35px,.6fr) minmax(25px,.5fr) minmax(30px,.5fr) minmax(50px,1fr)","alignItems": "center","columnGap": "4px","padding": "0 12px","height": "30px","background": c.bg,"boxShadow": c.edge,"font": "500 13px var(--font-ui)","color": "var(--dd-paper-1)"}}>
              <span style={{"color": "var(--dd-paper-4)","font": "700 12px var(--font-value)","letterSpacing": "-1px"}}>⋮⋮</span>
              <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-3)"}}>{c.ord}</span>
              <span style={{"display": "flex","alignItems": "center","gap": "6px","minWidth": "0","overflow": "hidden"}}><span style={{"width": "6px","height": "6px","borderRadius": "6px","background": c.dot,"flex": "none"}}></span><span style={{"flex": "none","whiteSpace": "nowrap"}}>{c.name}</span><span title={c.file} style={{"flex": "0 1 auto","minWidth": "0","overflow": "hidden","textOverflow": "ellipsis","font": "500 11px var(--font-value)","color": "var(--dd-paper-3)","whiteSpace": "nowrap"}}>{c.file}</span></span>
              <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-2)"}}>{c.vel}</span>
              <span style={{"display": "flex","alignItems": "center","gap": "4px"}}><span style={{"flex": "1","height": "4px","background": "var(--dd-ink-4)","borderRadius": "2px"}}><span style={{"display": "block","width": (c.wpct) + "%","height": "4px","background": "var(--dd-paper-2)","borderRadius": "2px"}}></span></span><span style={{"font": "500 11px var(--font-value)","color": "var(--dd-paper-2)"}}>{c.w}</span></span>
              <button aria-label={"Mute "+c.name} aria-pressed={c.muted} onClick={c.toggleMute} style={{border:0,padding:0,background:"transparent",font:"700 11px var(--font-ui)",color:c.mc}}>M</button>
              <button aria-label={"Solo "+c.name} aria-pressed={c.solo} onClick={c.toggleSolo} style={{border:0,padding:0,background:"transparent",font:"700 11px var(--font-ui)",color:c.solo?"var(--dd-vermilion)":"var(--dd-paper-4)"}}>S</button>
              <span style={{"font": "500 12px var(--font-value)"}}>{c.gain}</span>
              <span style={{"font": "500 12px var(--font-value)"}}>{c.pan}</span>
              <span style={{"font": "500 12px var(--font-value)"}}>{c.tune}</span>
              <span style={{"color": "var(--dd-paper-2)"}}>{c.out}</span>
            </div>
          </React.Fragment>)}
          <div style={{"padding": "8px 12px","display": "flex","alignItems": "center","gap": "8px","borderTop": "1px solid var(--dd-line-1)"}}>
            <DD.Button size="sm" variant="ghost" icon="plus" onClick={model.addCandidate}>Add sample…</DD.Button>
            <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>Drag rows to reorder. Drop files here to add candidates.</span>
          </div>
        </section></>;}
