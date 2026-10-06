import React,{useState} from 'react';
import {useModalFocus} from './useModalFocus.mjs';
import * as DD from './design-system/index.jsx';
export function PresetBrowser({presets,onLoad,onCancel}){
  const [query,setQuery]=useState(''),[selected,setSelected]=useState(presets[0]?.id),panel=useModalFocus(onCancel);
  return <div className="dialog-overlay" onPointerDown={onCancel}><section ref={panel} className="dialog-panel" role="dialog" aria-modal="true" aria-label="Preset browser" onPointerDown={e=>e.stopPropagation()}>
    <h2>Presets</h2><input aria-label="Search presets" value={query} onChange={e=>setQuery(e.target.value)} placeholder="Search presets"/>
    <div role="listbox" aria-label="Presets" style={{minHeight:100,margin:'10px 0'}}>{presets.filter(p=>p.name.toLowerCase().includes(query.toLowerCase())).map(p=><DD.ListRow key={p.id} label={p.name} detail={p.detail} selected={selected===p.id} onClick={()=>setSelected(p.id)}/>)}</div>
    <div className="dialog-actions"><DD.Button onClick={onCancel}>Cancel</DD.Button><DD.Button variant="primary" disabled={!selected} onClick={()=>onLoad(selected)}>Load preset</DD.Button></div>
  </section></div>;
}
