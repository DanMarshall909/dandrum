import React,{useEffect,useRef} from 'react';
import * as DD from './design-system/index.jsx';
export function ChoiceDialog({title,choices,onSelect,onCancel}){
  const panel=useRef(null);
  useEffect(()=>{const previous=document.activeElement;panel.current?.querySelector('button')?.focus();return()=>previous?.focus();},[]);
  return <div className="dialog-overlay" onPointerDown={onCancel}><section ref={panel} role="dialog" aria-modal="true" aria-label={title} className="dialog-panel" onPointerDown={e=>e.stopPropagation()} onKeyDown={e=>{e.stopPropagation();if(e.key==='Escape')onCancel();if(e.key==='Tab'){const items=[...panel.current.querySelectorAll('button')],first=items[0],last=items.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first.focus();}}}}>
    <h2>{title}</h2><div style={{display:'flex',flexWrap:'wrap',gap:8}}>{choices.map(choice=><DD.Button key={choice} onClick={()=>onSelect(choice)}>{choice}</DD.Button>)}</div>
    <div className="dialog-actions"><DD.Button onClick={onCancel}>Cancel</DD.Button></div>
  </section></div>;
}
