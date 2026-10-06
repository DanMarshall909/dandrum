import React,{useRef,useState,useEffect} from 'react';
import {WaveformPreview} from './WaveformPreview.jsx';

export function WaveformEditor({onRegionChange,...props}){
  const panel=useRef(null),viewport=props.viewport??{zoom:1,offset:0};
  const xpos=value=>(value-viewport.offset)*viewport.zoom;
  useEffect(()=>{const node=panel.current;if(!node)return;const wheel=e=>{if(!e.ctrlKey)return;e.preventDefault();e.stopPropagation();const zoom=Math.max(1,Math.min(16,viewport.zoom*(e.deltaY<0?1.25:.8))),box=node.getBoundingClientRect(),x=(e.clientX-box.left)/box.width,position=viewport.offset+x/viewport.zoom,offset=Math.max(0,Math.min(1-1/zoom,position-x/zoom));props.onViewport?.({zoom,offset});};node.addEventListener('wheel',wheel,{passive:false});return()=>node.removeEventListener('wheel',wheel);},[viewport.zoom,viewport.offset,props.onViewport]);
  const [draft,setDraft]=useState(null),drag=useRef(null),latest=useRef(null);
  const start=draft?.start??props.regionStart,end=draft?.end??props.regionEnd;
  const fadeIn=draft?.fades?.in??props.fadeIn??0,fadeOut=draft?.fades?.out??props.fadeOut??0;
  const handleChange=(field,value)=>{
    if(props.snapPosition&&['start','end','loopStart','loopEnd'].includes(field))value=props.snapPosition(value);
    const region={start,end,fades:{in:fadeIn,out:fadeOut},loop:props.loopStart==null?null:{start:props.loopStart,end:props.loopEnd,crossfade:props.crossfade}};
    if(field==='start')region.start=Math.max(0,Math.min(end-.0001,value));
    else if(field==='end')region.end=Math.min(1,Math.max(start+.0001,value));
    else if(field==='fadeIn')region.fades.in=Math.max(0,Math.min(end-start,value-start));
    else if(field==='fadeOut')region.fades.out=Math.max(0,Math.min(end-start,end-value));
    else if(field==='loopStart')region.loop.start=Math.max(start,Math.min(region.loop.end-.0001,value));
    else region.loop.end=Math.min(end,Math.max(region.loop.start+.0001,value));
    latest.current=region;setDraft(region);return region;
  };
  const handles=[['start','Region start',start,0],['end','Region end',end,0],['fadeIn','Fade in',start+fadeIn,1],['fadeOut','Fade out',end-fadeOut,1]];
  if(props.loopStart!=null)handles.push(['loopStart','Loop start',draft?.loop?.start??props.loopStart,2],['loopEnd','Loop end',draft?.loop?.end??props.loopEnd,2]);
  const finish=()=>{if(drag.current){drag.current=null;onRegionChange?.(latest.current);setDraft(null);}};
  return <div ref={panel} data-testid="waveform-editor" data-zoom={viewport.zoom} style={{position:'relative',overflow:'hidden'}}>
    <div style={{transform:`scaleX(${viewport.zoom}) translateX(${-viewport.offset*100}%)`,transformOrigin:'left center'}}>
    <WaveformPreview {...props} regionStart={start} regionEnd={end} fadeIn={fadeIn} fadeOut={fadeOut}
      loopStart={draft?.loop?.start??props.loopStart} loopEnd={draft?.loop?.end??props.loopEnd}/></div>
    {handles.map(([field,label,value,row])=><div key={field} role="slider" tabIndex={0} aria-label={label} aria-valuemin={0} aria-valuemax={1} aria-valuenow={value}
      title={label+' · drag or use arrow keys'}
      style={{position:'absolute',left:`calc(${xpos(value)*100}% - 5px)`,top:row===0?0:row===1?'calc(100% - 12px)':'50%',width:9,height:11,
        border:'1px solid var(--dd-paper-1)',background:row===2?'var(--dd-vermilion)':'var(--dd-ink-5)',borderRadius:2,cursor:'ew-resize',touchAction:'none'}}
      onClick={e=>e.stopPropagation()}
      onPointerDown={e=>{e.preventDefault();e.stopPropagation();e.currentTarget.focus();e.currentTarget.setPointerCapture(e.pointerId);
        drag.current={x:e.clientX,value,width:e.currentTarget.parentElement.clientWidth*viewport.zoom,field};latest.current=null;}}
    onPointerMove={e=>{if(!drag.current)return;e.stopPropagation();const d=drag.current;handleChange(d.field,d.value+(e.clientX-d.x)/d.width*(e.shiftKey ? .1 : 1));}}
      onPointerUp={()=>{if(latest.current)finish();else drag.current=null;}}
      onPointerCancel={()=>{drag.current=null;setDraft(null);}}
      onKeyDown={e=>{const steps={ArrowLeft:-1,ArrowRight:1,ArrowDown:-1,ArrowUp:1,PageDown:-10,PageUp:10};if(!(e.key in steps))return;
        e.preventDefault();e.stopPropagation();const delta=handleChange(field,value+steps[e.key]*.001*(e.shiftKey ? .1 : 1));onRegionChange?.(delta);setDraft(null);}}/>) }
  </div>;
}
