import React,{useRef,useState} from 'react';
import {WaveformPreview} from './WaveformPreview.jsx';
export function SliceWaveform({onMarkerChange,slices,...props}){
  const drag=useRef(null),[draft,setDraft]=useState(null);
  return <div style={{position:'relative'}}><WaveformPreview {...props} slices={slices.map(s=>draft?.id===s.id?{...s,pos:draft.pos}:s)}/>
    {slices.slice(1).map((slice,i)=><div key={slice.id} role="slider" tabIndex={0} aria-label={'Slice marker '+(i+2)} aria-valuenow={draft?.id===slice.id?draft.pos:slice.pos} aria-valuemin={0} aria-valuemax={1}
      style={{position:'absolute',left:`calc(${(draft?.id===slice.id?draft.pos:slice.pos)*100}% - 5px)`,top:0,width:9,height:13,border:'1px solid var(--dd-paper-1)',background:'var(--dd-vermilion)',borderRadius:2,cursor:'ew-resize',touchAction:'none'}}
      onClick={e=>e.stopPropagation()} onPointerDown={e=>{e.preventDefault();e.stopPropagation();e.currentTarget.focus();e.currentTarget.setPointerCapture(e.pointerId);drag.current={id:slice.id,x:e.clientX,pos:slice.pos,width:e.currentTarget.parentElement.clientWidth};}}
      onPointerMove={e=>{if(drag.current){e.stopPropagation();const d=drag.current;setDraft({id:d.id,pos:Math.max(0,Math.min(1,d.pos+(e.clientX-d.x)/d.width*(e.shiftKey?.1:1)))});}}}
      onPointerUp={()=>{if(draft)onMarkerChange(draft.id,draft.pos);drag.current=null;setDraft(null);}} onPointerCancel={()=>{drag.current=null;setDraft(null);}}
      onKeyDown={e=>{const steps={ArrowLeft:-1,ArrowRight:1,ArrowDown:-1,ArrowUp:1,PageDown:-10,PageUp:10};if(e.key in steps){e.preventDefault();e.stopPropagation();onMarkerChange(slice.id,slice.pos+steps[e.key]*.001*(e.shiftKey?.1:1));}}}/>)}
  </div>;
}
