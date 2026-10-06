import React,{useRef,useSyncExternalStore} from 'react';
export function EnvelopeEditor({envelope:e,height,getTelemetry,subscribeTelemetry}){
  const telemetry=useSyncExternalStore(subscribeTelemetry,getTelemetry),drag=useRef(null),live=telemetry.mod?.[e.liveRoute]??0;
  const change=(key,x,y)=>e.onPoint?.(key,x,y);
  return <div style={{position:'relative',height,background:'var(--dd-ink-0)',borderRadius:2,border:'1px solid var(--dd-line-1)'}}>
    <svg viewBox="0 0 300 100" preserveAspectRatio="none" style={{position:'absolute',inset:'6px 8px',width:'calc(100% - 16px)',height:'calc(100% - 12px)',overflow:'visible'}}>
      <polyline points={e.pts} fill="none" stroke="var(--dd-paper-1)" style={{strokeWidth:1.5,vectorEffect:'non-scaling-stroke'}}/>
      <line x1={e.sx} y1="0" x2={e.sx} y2="100" stroke="var(--dd-line-3)" style={{strokeDasharray:'3 3',vectorEffect:'non-scaling-stroke'}}/>
    </svg>
    {e.handles.map(h=><div key={h.key} data-knob={e.id+' '+h.key} role="slider" tabIndex={0} aria-label={e.title+' '+h.key} aria-valuenow={h.value??e.config[h.key]} aria-valuemin={0} aria-valuemax={h.key==='sustain'||h.key.startsWith('point-')?1:10}
      style={{position:'absolute',left:`calc(8px + (100% - 16px) * ${h.x})`,top:`calc(6px + (100% - 12px) * ${h.y})`,width:9,height:9,margin:'-5px 0 0 -5px',borderRadius:9,background:'var(--dd-ink-5)',border:'1px solid var(--dd-paper-1)',cursor:'move',touchAction:'none'}}
      onPointerDown={event=>{event.preventDefault();event.currentTarget.focus();event.currentTarget.setPointerCapture(event.pointerId);drag.current={x:event.clientX,y:event.clientY,startX:h.x,startY:h.y,width:event.currentTarget.parentElement.clientWidth-16,height:height-12};}}
      onPointerMove={event=>{if(!drag.current)return;const d=drag.current;change(h.key,d.startX+(event.clientX-d.x)/d.width*(event.shiftKey?.1:1),d.startY+(event.clientY-d.y)/d.height*(event.shiftKey?.1:1));}}
      onPointerUp={()=>drag.current=null} onPointerCancel={()=>drag.current=null}
      onKeyDown={event=>{const delta={ArrowLeft:-1,ArrowDown:-1,ArrowRight:1,ArrowUp:1,PageUp:10,PageDown:-10}[event.key];if(delta){event.preventDefault();event.stopPropagation();if(h.key.startsWith('point-'))change(h.key,h.x,h.y-delta*.01*(event.shiftKey?.1:1));else e.onChange(h.key,Math.max(0,Math.min(h.key==='sustain'?1:10,(e.config[h.key]??0)+delta*.001*(event.shiftKey?.1:1))));}}}/>)}
    <div style={{position:'absolute',left:`calc(8px + (100% - 16px) * ${live})`,top:6,bottom:6,width:1,background:'var(--dd-mod-a)',pointerEvents:'none'}}/>
  </div>;
}
