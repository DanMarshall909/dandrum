import {useModalFocus} from './useModalFocus.mjs';
import React,{useEffect,useRef,useState} from 'react';
import * as DD from './design-system/index.jsx';
export function RenameDialog({title,value,onSubmit,onCancel}){
  const panel=useModalFocus(onCancel);
  const [name,setName]=useState(value),input=useRef(null);
  useEffect(()=>{const previous=document.activeElement;input.current?.focus();input.current?.select();return()=>previous?.focus();},[]);
  return <div className="dialog-overlay" onPointerDown={onCancel}>
    <form ref={panel} role="dialog" aria-modal="true" aria-label={title} className="dialog-panel" onPointerDown={e=>e.stopPropagation()}
      onSubmit={e=>{e.preventDefault();if(name.trim())onSubmit(name);}} onKeyDown={e=>{e.stopPropagation();if(e.key==='Escape')onCancel();}}>
      <h2>{title}</h2><input ref={input} aria-label="Name" value={name} onChange={e=>setName(e.target.value)} maxLength={120}/>
      <div className="dialog-actions"><DD.Button onClick={onCancel}>Cancel</DD.Button><DD.Button variant="primary" disabled={!name.trim()} onClick={()=>onSubmit(name)}>Rename</DD.Button></div>
    </form>
  </div>;
}
