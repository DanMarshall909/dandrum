import React,{useRef} from 'react';
export function AmountBar({route:r}){
  const drag=useRef(null);
  return <span role="slider" tabIndex={0} aria-label={r.src+' to '+r.dest+' amount'} aria-valuenow={r.a} aria-valuemin={-1} aria-valuemax={1}
    title="Drag to change amount · Alt starts with the inverted amount" style={{position:'relative',height:6,background:'var(--dd-ink-0)',border:'1px solid var(--dd-line-2)',borderRadius:2,cursor:'ew-resize',touchAction:'none'}}
    onClick={e=>e.stopPropagation()} onPointerDown={e=>{e.preventDefault();e.stopPropagation();e.currentTarget.focus();e.currentTarget.setPointerCapture(e.pointerId);drag.current={x:e.clientX,width:e.currentTarget.clientWidth,amount:e.altKey?-r.a:r.a};if(e.altKey)r.setAmount(-r.a);}}
    onPointerMove={e=>{if(drag.current){e.stopPropagation();const d=drag.current;r.setAmount(Math.max(-1,Math.min(1,d.amount+(e.clientX-d.x)/d.width*2*(e.shiftKey?.1:1))));}}}
    onPointerUp={()=>drag.current=null} onPointerCancel={()=>drag.current=null} onKeyDown={e=>{const delta={ArrowLeft:-1,ArrowDown:-1,ArrowRight:1,ArrowUp:1,PageDown:-10,PageUp:10}[e.key];if(delta){e.preventDefault();e.stopPropagation();r.setAmount(Math.max(-1,Math.min(1,r.a+delta*.01*(e.shiftKey?.1:1))));}}}>
    <span style={{position:'absolute',left:'50%',top:-3,bottom:-3,width:1,background:'var(--dd-line-3)'}}/><span style={{position:'absolute',left:r.l+'%',width:r.w+'%',top:0,bottom:0,background:r.c,borderRadius:1}}/>
  </span>;
}
