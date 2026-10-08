import React,{useEffect,useRef,useState} from 'react';
import {Knob as SuppliedKnob} from '../vendor/design-system/primitives.mjs';

/** Keep the supplied drawing; adapt precision, typing, reset and pointer cleanup. */
export function ParameterKnob(props){
  const {value=.5,defaultValue=.5,onChange,disabled,popupOpen}=props;
  const drag=useRef(null),lastValue=useRef(value),timers=useRef({});
  const [active,setActive]=useState(false),[hover,setHover]=useState(false),[focus,setFocus]=useState(false);
  const clamp=v=>Math.max(0,Math.min(1,v));
  const change=v=>{if(!disabled){lastValue.current=clamp(v);onChange?.(lastValue.current);}};
  const cancel=()=>{if(drag.current){props.onCancel?.(drag.current.value);}drag.current=null;setActive(false);};
  const end=()=>{if(drag.current&&lastValue.current!==drag.current.value)props.onCommit?.(lastValue.current);drag.current=null;setActive(false);};
  useEffect(()=>{const blur=()=>{drag.current=null;setActive(false);};window.addEventListener('blur',blur);return()=>{Object.values(timers.current).forEach(clearTimeout);window.removeEventListener('blur',blur);};},[]);
  const nudge=v=>{change(v);props.onCommit?.(lastValue.current);setActive(true);clearTimeout(timers.current.nudge);timers.current.nudge=setTimeout(()=>setActive(false),600);};
  const pointerDown=e=>{
    if(e.target.closest('input')||e.button===2)return;
    if(disabled)return;
    e.stopPropagation();e.preventDefault();
    if(e.button===1||e.altKey){nudge(defaultValue);return;}
    const slider=e.currentTarget.querySelector('[role=slider]');slider?.focus();
    e.currentTarget.setPointerCapture(e.pointerId);lastValue.current=value;drag.current={y:e.clientY,value};setActive(true);
  };
  const pointerMove=e=>{
    if(!drag.current)return;e.stopPropagation();e.preventDefault();
    const factor=e.shiftKey ? .1 : 1;
    change(drag.current.value+(drag.current.y-e.clientY)/200*factor);
  };
  const keyDown=e=>{
    if(e.target.closest('input')||disabled)return;
    const steps={ArrowUp:1,ArrowRight:1,ArrowDown:-1,ArrowLeft:-1,PageUp:10,PageDown:-10};
    let next;if(e.key in steps)next=value+steps[e.key]*.05*(e.shiftKey ? .1 : 1);
    else if(e.key==='Home')next=0;else if(e.key==='End')next=1;
    else if(e.key==='Delete'||e.key==='Backspace')next=defaultValue;else return;
    e.preventDefault();e.stopPropagation();nudge(next);
  };
  const typeValue=e=>{
    if(disabled||e.target.closest('input'))return;e.preventDefault();e.stopPropagation();
    const slider=e.currentTarget.querySelector('[role=slider]');slider?.focus();
    slider?.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));
  };
  return <div data-knob={props.parameterId} data-reset={props.parameterId?'knob|'+props.parameterId:undefined} style={{display:'contents'}} onPointerDownCapture={pointerDown} onPointerMoveCapture={pointerMove}
    onPointerUpCapture={end} onPointerCancelCapture={cancel} onLostPointerCapture={cancel}
    onKeyDownCapture={keyDown} onDoubleClickCapture={typeValue}
    onFocusCapture={()=>setFocus(true)} onBlurCapture={e=>{if(!e.currentTarget.contains(e.relatedTarget))setFocus(false);}}
    onMouseEnter={()=>{clearTimeout(timers.current.hover);timers.current.hover=setTimeout(()=>setHover(true),300);}}
    onMouseLeave={()=>{clearTimeout(timers.current.hover);setHover(false);}}
    onWheelCapture={e=>{if(disabled||e.target.closest('input'))return;e.stopPropagation();nudge(value+(e.deltaY<0?1:-1)*.02*(e.shiftKey ? .1 : 1));}}>
    <SuppliedKnob {...props} onChange={v=>{change(v);props.onCommit?.(lastValue.current);}} active={active||props.active} popupOpen={!!popupOpen||active||hover||focus}/>
  </div>;
}
