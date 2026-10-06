import React from 'react';
import * as DD from './design-system/index.jsx';
export function ZoneLane({model}){return <><div onDrop={model.onMapDrop} onDragOver={model.onFileDrag} ref={model.mapRef} onContextMenu={model.onMapCtx} onPointerMove={model.onMapMove} style={{"padding": "8px","display": "flex","flexDirection": "column","gap": "4px"}}>
            <DD.KeyMap zones={model.zones} selectedId={model.zoneSel} onSelect={model.selZone} onChange={model.chZones} lowNote={24} highNote={96} gridHeight={model.keymapH} title={model.mappingTitle} onNoteOn={model.onKeyOn} onNoteOff={model.onKeyOff}></DD.KeyMap>
            <div style={{"position": "relative","height": "14px","marginLeft": "30px","background": "var(--dd-ink-0)","borderRadius": "2px"}}>
              <span style={{"position": "absolute","left": "-30px","top": "1px","width": "26px","textAlign": "right","font": "600 11px/12px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-4)"}}>XF</span>
              {(model.relSegs || []).map((r,index) => <React.Fragment key={r.id ?? r.key ?? index}>
                <div title={r.title} style={{"position": "absolute","top": "2px","bottom": "2px","left": (r.l) + "%","width": (r.w) + "%","boxSizing": "border-box","borderRadius": "1px","background": r.bg,"border": r.bd}}></div>
              </React.Fragment>)}
            </div>
            <div style={{"display": "flex","alignItems": "center","gap": "12px","flexWrap": "wrap","minHeight": "20px","font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>
              {(model.relList || []).map((r,index) => <React.Fragment key={r.id ?? r.key ?? index}>
                <span style={{"display": "flex","alignItems": "center","gap": "6px","whiteSpace": "nowrap"}}><span style={{"width": "14px","height": "8px","boxSizing": "border-box","borderRadius": "1px","background": r.bg,"border": r.bd}}></span><span style={{"color": "var(--dd-paper-2)"}}>{r.kind}</span>{r.text}</span>
              </React.Fragment>)}
              {model.noRel && <><span >No key crossfades or gaps. Shift-drag a zone edge to overlap or separate.</span></>}
              <div style={{"flex": "1"}}></div>
              {model.dragShift && <><span style={{"font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-vermilion)"}}>Shift · free edge</span></>}
              {model.mapNote && <><span style={{"color": "var(--dd-paper-1)"}}>{model.mapNote}</span></>}
              {model.clipLabel && <><span style={{"font": "500 11px var(--font-value)","color": "var(--dd-paper-3)","border": "1px solid var(--dd-line-2)","borderRadius": "2px","padding": "0 5px"}}>Clipboard · {model.clipLabel}</span></>}
            </div>
            {model.menuOpen && <>
              <div onPointerDown={model.closeMenu} onContextMenu={model.closeMenuCtx} style={{"position": "fixed","inset": "0","zIndex": "20"}}></div>
              <div style={{"position": "absolute","left": (model.menuX) + "px","top": (model.menuY) + "px","zIndex": "21"}}>
                <DD.ContextMenu items={model.menuItems} title={model.menuTitle} width={260} onClose={model.closeMenu}></DD.ContextMenu>
              </div>
            </>}
          </div></>;}
