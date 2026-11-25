import React,{useEffect,useMemo,useRef,useState} from 'react';
import {NumericField as SuppliedField} from '../vendor/design-system/primitives.mjs';
import {createParameterDescriptor,stepRational} from '../shell/parameter-descriptor.mjs';

export function NumericField({showLabel=true,descriptor:provided,...props}){
  const decimals=Math.max(0,Math.ceil(-Math.log10(props.step??1)));
  const format=props.format??(value=>String(Number(value.toFixed(decimals))));
  const descriptor=useMemo(()=>provided??createParameterDescriptor({kind:'numeric-field',min:props.min??0,max:props.max??100,defaultValue:props.defaultValue??props.min??0,step:props.step??1,precision:decimals,discrete:true,formatNative:format}),[provided,props.min,props.max,props.defaultValue,props.step,format,decimals]);
  const descriptorKey=JSON.stringify([descriptor.kind,descriptor.min,descriptor.max,descriptor.defaultValue,descriptor.scale,descriptor.unit,descriptor.step,descriptor.fineStep,descriptor.coarseStep,descriptor.bipolar,descriptor.discrete,descriptor.marks]);
  const root=useRef(null),rationalValue=useRef(null),lastDescriptor=useRef(descriptorKey);
  const [rational,setRational]=useState(null),position=descriptor.toPosition(props.value);
  useEffect(()=>{if(lastDescriptor.current!==descriptorKey||(rational&&Math.abs(props.value-rationalValue.current)>1e-9)){setRational(null);rationalValue.current=null;}lastDescriptor.current=descriptorKey;},[props.value,descriptorKey]);
  const display=value=>descriptor.format(descriptor.toPosition(value));
  const change=value=>{if(display(value)!==display(props.value))props.onChange?.(value);};
  const gesture=name=>root.current?.dispatchEvent(new CustomEvent(`dandrum-control-${name}`,{bubbles:true,detail:{id:props.parameterId??props.label}}));
  return <span ref={root} data-hide-field-label={!showLabel||undefined}><SuppliedField {...props} unit={provided?undefined:props.unit} min={descriptor.min} max={descriptor.max} step={descriptor.step} fineStep={descriptor.fineStep} coarseStep={descriptor.coarseStep} defaultValue={descriptor.defaultValue}
    format={display} parseValue={text=>descriptor.parseEdit(text)} accessibleValueText={descriptor.accessibleText(position)} snapValue={descriptor.discrete}
    editText={rational?`${rational.numerator}/${rational.denominator}`:display(props.value)}
    scrubValue={rational?({verticalPixels,shiftKey})=>{const divisor=shiftKey?12:4,result=stepRational(rational,Math.round(verticalPixels/divisor),descriptor);setRational(result.rational);return result.nativeValue;}:undefined}
    onEditBegin={()=>gesture('begin')} onEditEnd={()=>gesture('end')}
    onEditAccepted={result=>{setRational(result?.rational??null);rationalValue.current=result?.rational?result.nativeValue:null;}} onChange={change}/></span>;
}
