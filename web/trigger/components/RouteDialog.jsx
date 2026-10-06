import {useModalFocus} from './useModalFocus.mjs';
import React,{useEffect,useRef,useState} from 'react';
import * as DD from './design-system/index.jsx';
export function RouteDialog({sources,destinations,onSubmit,onCancel}){
  const [source,setSource]=useState(sources[0]?.id),[destination,setDestination]=useState(destinations[0]?.id),panel=useModalFocus(onCancel);
  useEffect(()=>{const previous=document.activeElement;panel.current?.querySelector('button')?.focus();return()=>previous?.focus();},[]);
  return <div className="dialog-overlay" onPointerDown={onCancel}><section ref={panel} role="dialog" aria-modal="true" aria-label="Add modulation route" className="dialog-panel" onPointerDown={e=>e.stopPropagation()} onKeyDown={e=>{e.stopPropagation();if(e.key==='Escape')onCancel();}}>
    <h2>Add modulation route</h2><DD.MenuButton label="Source" value={source} options={sources} onChange={setSource}/><DD.MenuButton label="Destination" value={destination} options={destinations} onChange={setDestination}/>
    <div className="dialog-actions"><DD.Button onClick={onCancel}>Cancel</DD.Button><DD.Button variant="primary" disabled={!source||!destination} onClick={()=>onSubmit({source,destination,amount:.25,polarity:'Uni',curve:'Linear',on:true})}>Add route</DD.Button></div>
  </section></div>;
}
