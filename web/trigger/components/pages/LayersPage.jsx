import {CandidateTable} from '../CandidateTable.jsx';
import {CycleSequence} from '../CycleSequence.jsx';
import {VelocityCrossfade} from '../VelocityCrossfade.jsx';
import {SelectorModeBar} from '../SelectorModeBar.jsx';
import React,{useSyncExternalStore} from 'react';
import * as DD from '../design-system/index.jsx';
export function LayersPage({model}) { return model.isLayers?<LayersContent model={model}/>:null; }
function LayersContent({model:base}) {
  const telemetry=useSyncExternalStore(base.subscribeTelemetry,base.getTelemetry),position=telemetry.selectorPos?.[base.selectorId];
  const lastVelocity=telemetry.notes?.at(-1)?.vel;
  const model={...base,cands:base.cands.map(c=>({...c,dot:position?.last===c.id?'var(--dd-paper-1)':'var(--dd-paper-4)'})),
    rrSeq:base.rrSeq.map(q=>({...q,tag:position?.last===q.id?'Last':position?.next===q.id?'Next':'',bd:position?.last===q.id?'var(--dd-paper-2)':position?.next===q.id?'var(--dd-vermilion)':'var(--dd-line-2)'})),
    vLast:lastVelocity==null?0:(lastVelocity-1)/126*100,vAxis:lastVelocity==null?'Play a note to see its velocity':'last hit vel '+lastVelocity};
  return <>{model.isLayers && <>
        <CandidateTable model={model}/>
      </>}</>; }
