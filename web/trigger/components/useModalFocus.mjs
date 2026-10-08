import {useEffect,useRef} from 'react';
/** Shared dialog focus, containment, Escape and restoration. */
export function useModalFocus(onCancel){
  const panel=useRef(null),cancel=useRef(onCancel);cancel.current=onCancel;
  useEffect(()=>{const previous=document.activeElement,node=panel.current;
    const items=()=>[...node.querySelectorAll('button:not([disabled]),input:not([disabled]),[tabindex="0"]')].filter(e=>e.getClientRects().length);
    (items()[0]??node)?.focus();
    const key=e=>{if(!node?.contains(e.target))return;
      if(e.key==='Escape'){e.preventDefault();e.stopImmediatePropagation();cancel.current?.();}
      if(e.key==='Tab'){const list=items(),at=list.indexOf(document.activeElement);if(!list.length){e.preventDefault();node.focus();return;}
        if(e.shiftKey&&at<=0||!e.shiftKey&&at===list.length-1){e.preventDefault();list[e.shiftKey?list.length-1:0].focus();}}
    };
    document.addEventListener('keydown',key,true);return()=>{document.removeEventListener('keydown',key,true);if(previous?.isConnected)previous.focus();};
  },[]);return panel;
}
