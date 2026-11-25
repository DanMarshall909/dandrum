import React,{useEffect,useRef,useState} from 'react';
import {Knob as SuppliedKnob} from '../vendor/design-system/primitives.mjs';
import {fractionDragSteps,stepRational} from '../shell/parameter-descriptor.mjs';

/** Keep the supplied drawing; adapt precision, typing, reset and pointer cleanup. */
export function ParameterKnob(props){
  const {value=.5,defaultValue=.5,onChange,disabled,popupOpen,descriptor}=props;
  const position=descriptor?descriptor.toPosition(value):value;
  const defaultPosition=descriptor?descriptor.toPosition(defaultValue):defaultValue;
  const descriptorKey=descriptor&&JSON.stringify([descriptor.kind,descriptor.min,descriptor.max,descriptor.defaultValue,descriptor.scale,descriptor.unit,descriptor.step,descriptor.fineStep,descriptor.coarseStep,descriptor.bipolar,descriptor.discrete,descriptor.marks]);
  const root=useRef(null),drag=useRef(null),lastValue=useRef(position),timers=useRef({}),rationalValue=useRef(null),lastDescriptor=useRef(descriptorKey);
  const [active,setActive]=useState(false),[hover,setHover]=useState(false),[focus,setFocus]=useState(false);
  const [rational,setRational]=useState(null);
  const clamp=v=>Math.max(0,Math.min(1,v));
  const native=v=>descriptor?descriptor.toNative(v):v;
  const change=v=>{if(!disabled){lastValue.current=clamp(v);onChange?.(native(lastValue.current));}};
  const gesture=(name)=>root.current?.dispatchEvent(new CustomEvent(`dandrum-control-${name}`,{bubbles:true,detail:{id:props.parameterId??props.label}}));
  const cancel=()=>{if(drag.current){props.onCancel?.(native(drag.current.value));setRational(drag.current.rational);rationalValue.current=native(drag.current.value);}drag.current=null;setActive(false);};
  const end=()=>{if(drag.current&&lastValue.current!==drag.current.value)props.onCommit?.(native(lastValue.current));drag.current=null;setActive(false);};
  useEffect(()=>{if(lastDescriptor.current!==descriptorKey||(rational&&Math.abs(value-rationalValue.current)>1e-9)){setRational(null);rationalValue.current=null;}lastDescriptor.current=descriptorKey;},[value,descriptorKey]);
  useEffect(()=>{const blur=()=>cancel();window.addEventListener('blur',blur);return()=>{Object.values(timers.current).forEach(clearTimeout);window.removeEventListener('blur',blur);};},[]);
  const nudge=v=>{gesture('begin');setRational(null);rationalValue.current=null;change(v);props.onCommit?.(native(lastValue.current));gesture('end');setActive(true);clearTimeout(timers.current.nudge);timers.current.nudge=setTimeout(()=>setActive(false),600);};
  const stepPosition=(amount)=>descriptor?descriptor.toPosition(descriptor.normalizeNative(value+amount)):position+amount;
  const pointerDown=e=>{
    if(e.target.closest('[role=dialog]')||e.button===2)return;
    if(disabled)return;
    e.stopPropagation();e.preventDefault();
    if(e.button===1||e.altKey){nudge(defaultPosition);return;}
    const slider=e.currentTarget.querySelector('[role=slider]');slider?.focus();
    e.currentTarget.setPointerCapture(e.pointerId);lastValue.current=position;drag.current={y:e.clientY,value:position,rational};setActive(true);
  };
  const pointerMove=e=>{
    if(!drag.current)return;e.stopPropagation();e.preventDefault();
    const distance=drag.current.y-e.clientY;
    if(drag.current.rational&&descriptor){const result=stepRational(drag.current.rational,fractionDragSteps(distance,e.shiftKey),descriptor);setRational(result.rational);rationalValue.current=result.nativeValue;change(result.position);return;}
    const factor=e.shiftKey ? .1 : 1;change(drag.current.value+distance/200*factor);
  };
  const keyDown=e=>{
    if(e.target.closest('input')||disabled)return;
    const steps={ArrowUp:1,ArrowRight:1,ArrowDown:-1,ArrowLeft:-1};
    let next;if(e.key in steps)next=stepPosition(steps[e.key]*(e.shiftKey?(descriptor?.fineStep??.005):(descriptor?.step??.05)));
    else if(e.key==='PageUp'||e.key==='PageDown')next=stepPosition((e.key==='PageUp'?1:-1)*(descriptor?.coarseStep??.5));
    else if(e.key==='Home')next=0;else if(e.key==='End')next=1;
    else if(e.key==='Delete'||e.key==='Backspace')next=defaultPosition;else return;
    e.preventDefault();e.stopPropagation();nudge(next);
  };
  return <div ref={root} data-knob={props.parameterId} data-reset={props.parameterId?'knob|'+props.parameterId:undefined} style={{display:'contents'}} onPointerDownCapture={pointerDown} onPointerMoveCapture={pointerMove}
    onPointerUpCapture={end} onPointerCancelCapture={cancel} onLostPointerCapture={cancel}
    onKeyDownCapture={keyDown} onDoubleClickCapture={e=>{if(disabled||e.target.closest('[role=dialog]'))return;e.preventDefault();e.stopPropagation();nudge(defaultPosition);}}
    onFocusCapture={()=>setFocus(true)} onBlurCapture={e=>{if(!e.currentTarget.contains(e.relatedTarget))setFocus(false);}}
    onMouseEnter={()=>{clearTimeout(timers.current.hover);timers.current.hover=setTimeout(()=>setHover(true),300);}}
    onMouseLeave={()=>{clearTimeout(timers.current.hover);setHover(false);}}
    onWheelCapture={e=>{if(disabled||e.target.closest('[role=dialog]'))return;e.stopPropagation();nudge(stepPosition((e.deltaY<0?1:-1)*(e.shiftKey?(descriptor?.fineStep??.002):(descriptor?.step??.02))));}}>
    <SuppliedKnob {...props} value={position} defaultValue={defaultPosition} bipolar={descriptor?.bipolar??props.bipolar}
      marks={descriptor?.marks}
      valueText={descriptor?.format(position)??props.valueText} accessibleValueText={descriptor?.accessibleText(position)??props.valueText}
      ariaValueMin={descriptor?.min} ariaValueMax={descriptor?.max} ariaValueNow={descriptor?.toNative(position)}
      editText={rational?`${rational.numerator}/${rational.denominator}`:descriptor?.format(position)}
      parseValue={descriptor?text=>descriptor.parseEdit(text):props.parseValue}
      onEditBegin={()=>gesture('begin')}
      onEditAccepted={result=>{setRational(result?.rational??null);rationalValue.current=result?.rational?result.nativeValue:null;props.onEditAccepted?.(result);props.onCommit?.(native(lastValue.current));gesture('end');}}
      onChange={change} active={active||props.active} popupOpen={!!popupOpen||active||hover||focus}/>
  </div>;
}
