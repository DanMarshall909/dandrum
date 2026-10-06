import {useModalFocus} from './useModalFocus.mjs';
import React,{useEffect,useRef} from 'react';
import * as DD from './design-system/index.jsx';
export function VoiceTemplateDialog({model,onClose}){
  const dialog=useModalFocus(onClose);
  useEffect(()=>{const previous=document.activeElement;dialog.current?.focus();return()=>previous?.focus();},[]);
  return <div className="dialog-overlay" onPointerDown={onClose}><section ref={dialog} role="dialog" aria-modal="true" aria-label="Voice template" tabIndex={-1} className="dialog-panel" onPointerDown={e=>e.stopPropagation()} onKeyDown={e=>{e.stopPropagation();if(e.key==='Escape')onClose();}}>
    <h2>Voice template</h2>{model.modules.map((m,i)=><div key={m.id} style={{display:'flex',gap:8,alignItems:'center',marginBottom:8}}><span style={{flex:1}}>{m.type}</span>
      <DD.IconButton label={'Move '+m.type+' up'} icon="chevron-up" disabled={!i} onClick={()=>model.move(m.id,-1)}/><DD.IconButton label={'Move '+m.type+' down'} icon="chevron-down" disabled={i===model.modules.length-1} onClick={()=>model.move(m.id,1)}/>
      <DD.IconButton label={'Remove '+m.type} icon="minus" disabled={model.modules.length===1} onClick={()=>model.remove(m.id)}/></div>)}
    <DD.MenuButton label="Add module" value="add" options={['Sample player','Lush','Filter','Amplifier'].map(type=>({id:type,label:type}))} onChange={model.add}/>
    <div className="dialog-actions"><DD.Button onClick={onClose}>Done</DD.Button></div>
  </section></div>;
}
