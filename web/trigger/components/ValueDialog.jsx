import React,{useEffect,useRef,useState} from 'react';
import * as DD from './design-system/index.jsx';
export function ValueDialog({title,label,value,min=0,max=127,onSubmit,onCancel}){
  const [current,setCurrent]=useState(value),form=useRef(null);
  useEffect(()=>{const previous=document.activeElement;form.current?.querySelector('input')?.focus();return()=>previous?.focus();},[]);
  const submit=()=>{if(Number.isFinite(current)&&current>=min&&current<=max)onSubmit(current);};
  return <div className="dialog-overlay" onPointerDown={onCancel}><form ref={form} role="dialog" aria-modal="true" aria-label={title} className="dialog-panel" onPointerDown={e=>e.stopPropagation()}
    onSubmit={e=>{e.preventDefault();submit();}} onKeyDown={e=>{e.stopPropagation();if(e.key==='Escape')onCancel();if(e.key==='Tab'){const items=[...form.current.querySelectorAll('input,button')].filter(el=>!el.disabled),first=items[0],last=items.at(-1);
      if(e.shiftKey&&document.activeElement===first){e.preventDefault();last.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first.focus();}}}}>
    <h2>{title}</h2><DD.NumericField label={label} value={current} min={min} max={max} onChange={setCurrent}/>
    <div className="dialog-actions"><DD.Button onClick={onCancel}>Cancel</DD.Button><DD.Button variant="primary" onClick={submit}>Apply</DD.Button></div>
  </form></div>;
}
