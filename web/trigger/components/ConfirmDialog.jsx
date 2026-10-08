import React,{useEffect,useRef} from 'react';
import * as DD from './design-system/index.jsx';
export function ConfirmDialog({onConfirm,onCancel}){
  const panel=useRef(null);
  useEffect(()=>{const previous=document.activeElement;panel.current?.querySelector('button')?.focus();return()=>previous?.focus();},[]);
  const keyboard=e=>{
    if(e.key==='Escape'){e.stopPropagation();onCancel();}
    if(e.key==='Tab'){const buttons=[...panel.current.querySelectorAll('button')],index=buttons.indexOf(document.activeElement);
      e.preventDefault();buttons[(index+(e.shiftKey?-1:1)+buttons.length)%buttons.length]?.focus();}
  };
  return <div className="dialog-overlay" onPointerDown={onCancel}>
    <div ref={panel} role="dialog" aria-modal="true" aria-labelledby="discard-title" className="dialog-panel" onKeyDown={keyboard} onPointerDown={e=>e.stopPropagation()}>
      <h2 id="discard-title">Discard unsaved edits?</h2>
      <p>Replacing the patch clears its edit history after the new patch loads.</p>
      <div className="dialog-actions">
        <DD.Button onClick={onCancel}>Keep editing</DD.Button>
        <DD.Button variant="primary" onClick={onConfirm}>Discard and continue</DD.Button>
      </div>
    </div>
  </div>;
}
