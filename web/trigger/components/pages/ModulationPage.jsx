import {ModulatorEditor} from '../ModulatorEditor.jsx';
import {ModulationTable} from '../ModulationTable.jsx';
import React,{useSyncExternalStore} from 'react';
import {AmountBar} from '../AmountBar.jsx';
import * as DD from '../design-system/index.jsx';
export function ModulationPage({model}) {return model.isMod?<ModulationContent model={model}/>:null;}
function ModulationContent({model:base}){
  const telemetry=useSyncExternalStore(base.subscribeTelemetry,base.getTelemetry);
  const model={...base,routeGroups:base.routeGroups.map(g=>({...g,live:(telemetry.mod[g.items[0]?.id]??0)*100,items:g.items.map(r=>({...r,live:(telemetry.mod[r.id]??0)*100}))})),me:{...base.me,live:telemetry.mod[base.me.routeId]??0}};
  return <>{model.isMod && <>
        <ModulationTable model={model}/>
        <ModulatorEditor model={model}/>
        <section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","padding": "10px 12px","display": "flex","gap": "16px","alignItems": "center","flexWrap": "wrap"}}>
          <div data-knob="Cutoff" data-reset="knob|Cutoff" style={{"borderRadius": "4px","boxShadow": model.cutoffRing}}><DD.Knob {...model.kTight} size="md" value={model.cutoffValue} defaultValue={.42} parseValue={model.cutoffUnits.parse} onChange={model.setCutoff} label="Cutoff" valueText={model.cutoffUnits.format(model.cutoffValue)} modulations={model.cutoffMods} popupOpen={model.cutoffPop} assigning={model.assigning}></DD.Knob></div>
          <div style={{"display": "flex","flexDirection": "column","gap": "4px","maxWidth": "420px"}}>
            <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Contextual view</span>
            <span style={{"font": "500 13px/1.35 var(--font-ui)","color": "var(--dd-paper-2)"}}>Every modulated control shows up to two rings and a +N chip. Hover or focus opens the value popup listing its sources. Drag a modulator from the left panel onto any control, or right-click / M for Assign modulation…</span>
          </div>
        </section>
      </>}</>; }
