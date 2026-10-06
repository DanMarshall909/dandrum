import {useModalFocus} from './useModalFocus.mjs';
import React,{useState} from 'react';
import * as DD from './design-system/index.jsx';
export function MidiLearnDialog({target,onReceive,onCancel}){
  const panel=useModalFocus(onCancel);
  const [cc,setCc]=useState(74),[channel,setChannel]=useState(1);
  return <div className="dialog-overlay" onPointerDown={onCancel}><section ref={panel} role="dialog" aria-modal="true" aria-label="MIDI learn" className="dialog-panel" onPointerDown={e=>e.stopPropagation()} onKeyDown={e=>{e.stopPropagation();if(e.key==='Escape')onCancel();}}>
    <h2>MIDI learn · {target}</h2><p>Waiting for a controller. Send a simulated MIDI CC in this standalone preview.</p>
    <DD.NumericField label="CC number" value={cc} min={0} max={127} onChange={setCc}/><DD.NumericField label="MIDI channel" value={channel} min={1} max={16} onChange={setChannel}/>
    <div className="dialog-actions"><DD.Button onClick={onCancel}>Cancel</DD.Button><DD.Button variant="primary" onClick={()=>onReceive(cc,channel)}>Send MIDI CC</DD.Button></div>
  </section></div>;
}
