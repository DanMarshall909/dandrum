const clamp=(value,min,max)=>Math.max(min,Math.min(max,value));
const finite=value=>Number.isFinite(value)?value:null;
const decimals=step=>Math.max(0,Math.ceil(-Math.log10(Math.abs(step||1))));
const normalizedText=text=>String(text).trim().replaceAll('−','-');
const numberPattern='[+-]?(?:\\d+(?:\\.\\d*)?|\\.\\d+)';

const defaultParser=unit=>text=>{
  const escaped=String(unit??'').replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
  const match=normalizedText(text).match(new RegExp(`^(${numberPattern})${escaped?`\\s*${escaped}`:''}$`,'i'));
  return match?Number(match[1]):null;
};

const parseFraction=text=>{
  const match=normalizedText(text).match(/^([+-]?\d+)\s*\/\s*(\d+)$/);
  if(!match)return null;
  const numerator=Number(match[1]);const denominator=Number(match[2]);
  return Number.isSafeInteger(numerator)&&Number.isSafeInteger(denominator)&&denominator>0?{numerator,denominator}:null;
};

const compactDecimal=(value,precision=2)=>{
  const absolute=Math.abs(value).toFixed(precision).replace(/0+$/,'').replace(/\.$/,'');
  const compact=absolute.startsWith('0.')?absolute.slice(1):absolute;
  if(value<0)return '−'+compact;
  if(value>0)return '+'+compact;
  return '0';
};
const percentAmount=value=>Math.round(Math.abs(value)*100);

export function createParameterDescriptor(options={}){
  const enumValues=options.values?.map(Number);
  const inferredMin=enumValues?.[0]??0;const inferredMax=enumValues?.at(-1)??1;
  const {
    kind='number',min=inferredMin,max=inferredMax,defaultValue=min,scale='linear',unit='',step=1,
    fineStep=step/10,coarseStep=step*10,precision=decimals(step),marks=[],bipolar=false,
    discrete=['integer','enum'].includes(kind),formatNative,parseNative=defaultParser(unit),accessibleNative,
  }=options;
  if(!Number.isFinite(min)||!Number.isFinite(max)||max<=min)throw new TypeError('Parameter range must be finite and increasing');
  if(scale==='log'&&(min<=0||max<=0))throw new TypeError('Logarithmic parameter ranges must be positive');
  if(enumValues&&(!enumValues.length||enumValues.some((value,index)=>!Number.isFinite(value)||(index&&value<=enumValues[index-1]))))throw new TypeError('Enumerated values must be finite and increasing');
  const nearestEnum=value=>enumValues.reduce((nearest,candidate)=>Math.abs(candidate-value)<Math.abs(nearest-value)?candidate:nearest);
  const rawNative=enumValues?
    position=>enumValues[Math.round(clamp(position,0,1)*(enumValues.length-1))]:
    scale==='log'?position=>min*Math.pow(max/min,clamp(position,0,1)):
    position=>min+(max-min)*clamp(position,0,1);
  const rawPosition=enumValues?
    value=>enumValues.indexOf(nearestEnum(clamp(value,min,max)))/(enumValues.length-1):
    scale==='log'?value=>Math.log(clamp(value,min,max)/min)/Math.log(max/min):
    value=>(clamp(value,min,max)-min)/(max-min);
  const normalizeNative=value=>{
    const bounded=clamp(Number(value),min,max);
    if(enumValues)return nearestEnum(bounded);
    if(!discrete||!Number.isFinite(step)||step<=0)return bounded;
    const snapped=Math.round(bounded/step)*step;
    return clamp(Number(snapped.toFixed(Math.max(precision,decimals(step)))),min,max);
  };
  const toNative=position=>normalizeNative(rawNative(position));
  const toPosition=value=>rawPosition(normalizeNative(value));
  const formatValue=value=>{
    if(formatNative)return formatNative(value);
    const text=precision?Number(value.toFixed(precision)).toFixed(precision):String(Math.round(value));
    return unit?text+' '+unit:text;
  };
  const descriptor={
    kind,min,max,defaultValue:normalizeNative(defaultValue),scale,unit,step,fineStep,coarseStep,precision,marks:[...marks],bipolar,discrete,values:enumValues,
    normalizeNative,toNative,toPosition,
    format:position=>formatValue(toNative(position)),
    accessibleText:position=>accessibleNative?accessibleNative(toNative(position)):formatValue(toNative(position)),
    parseEdit:text=>{
      const rational=parseFraction(text);
      if(rational){
        const proportion=rational.numerator/rational.denominator;
        const position=bipolar?clamp(.5+proportion/2,0,1):clamp(proportion,0,1);
        return {valid:true,nativeValue:toNative(position),position,rational};
      }
      const parsed=finite(parseNative(text));
      if(parsed==null)return {valid:false,error:`Enter a valid${unit?' '+unit:''} value`};
      const nativeValue=normalizeNative(parsed);
      return {valid:true,nativeValue,position:toPosition(nativeValue),rational:null};
    },
  };
  descriptor.defaultPosition=descriptor.toPosition(descriptor.defaultValue);
  return Object.freeze(descriptor);
}

const parsePercent=text=>{const match=normalizedText(text).match(new RegExp(`^(${numberPattern})\\s*%?$`));return match?Number(match[1])/100:null;};
const parseUnit=text=>{const match=normalizedText(text).match(new RegExp(`^(${numberPattern})$`));return match?Number(match[1]):null;};
const parsePan=text=>{
  const value=normalizedText(text).toUpperCase();if(value==='C'||value==='CENTER'||value==='CENTRE')return 0;
  const leading=value.match(new RegExp(`^([LR])\\s*(${numberPattern})\\s*%?$`));
  if(leading)return (leading[1]==='L'?-1:1)*Math.abs(Number(leading[2]))/100;
  const trailing=value.match(new RegExp(`^(${numberPattern})\\s*%?\\s*([LR])$`));
  return trailing?(trailing[2]==='L'?-1:1)*Math.abs(Number(trailing[1]))/100:null;
};
export const parsePhysical=(units,scale=1)=>text=>{
  const match=normalizedText(text).match(new RegExp(`^(${numberPattern})\\s*(${units})?$`,'i'));
  if(!match)return null;return Number(match[1])*(typeof scale==='function'?scale(match[2]?.toLowerCase()):scale);
};

export function createParameterVariant(kind,overrides={}){
  switch(kind){
    case 'percent':return createParameterDescriptor({kind,min:0,max:1,defaultValue:0,step:.01,precision:2,formatNative:value=>percentAmount(value)+'%',parseNative:parsePercent,accessibleNative:value=>percentAmount(value)+' percent',...overrides});
    case 'unit-interval':return createParameterDescriptor({kind,min:0,max:1,defaultValue:0,step:.01,precision:2,formatNative:value=>compactDecimal(value).replace(/^\+/,''),parseNative:parseUnit,accessibleNative:value=>String(Number(value.toFixed(2))),...overrides});
    case 'bipolar-percent':return createParameterDescriptor({kind,min:-1,max:1,defaultValue:0,bipolar:true,step:.01,precision:2,formatNative:value=>value===0?'0':(value<0?'−':'+')+percentAmount(value)+'%',parseNative:parsePercent,accessibleNative:value=>value===0?'zero':`${Math.abs(Math.round(value*100))} percent ${value<0?'negative':'positive'}`,...overrides});
    case 'bipolar-unit':return createParameterDescriptor({kind,min:-1,max:1,defaultValue:0,bipolar:true,step:.01,precision:2,formatNative:value=>compactDecimal(value),parseNative:parseUnit,accessibleNative:value=>value===0?'zero':`${Math.abs(Number(value.toFixed(2)))} ${value<0?'negative':'positive'}`,...overrides});
    case 'pan':return createParameterDescriptor({kind,min:-1,max:1,defaultValue:0,bipolar:true,step:.01,precision:2,formatNative:value=>value===0?'C':`${value<0?'L':'R'} ${percentAmount(value)}%`,parseNative:parsePan,accessibleNative:value=>value===0?'centre':`${percentAmount(value)} percent ${value<0?'left':'right'}`,...overrides});
    case 'frequency':return createParameterDescriptor({kind,min:20,max:20000,defaultValue:1000,scale:'log',unit:'Hz',step:1,precision:0,formatNative:value=>Math.round(value)+' Hz',parseNative:parsePhysical('k?hz',unit=>unit==='khz'?1000:1),accessibleNative:value=>Math.round(value)+' hertz',...overrides});
    case 'time':return createParameterDescriptor({kind,min:.001,max:10,defaultValue:.1,scale:'log',unit:'s',step:.001,precision:3,formatNative:value=>value<1?Math.round(value*1000)+' ms':Number(value.toFixed(3))+' s',parseNative:parsePhysical('ms|s',unit=>unit==='ms'?.001:1),accessibleNative:value=>value<1?Math.round(value*1000)+' milliseconds':Number(value.toFixed(3))+' seconds',...overrides});
    case 'pitch':return createParameterDescriptor({kind,min:-24,max:24,defaultValue:0,unit:'st',step:1,precision:0,discrete:true,parseNative:parsePhysical('st|semitones?'),accessibleNative:value=>`${value} semitones`,...overrides});
    case 'cents':return createParameterDescriptor({kind,min:-100,max:100,defaultValue:0,unit:'ct',step:1,precision:0,discrete:true,parseNative:parsePhysical('ct|cents?'),accessibleNative:value=>`${value} cents`,...overrides});
    case 'decibel':return createParameterDescriptor({kind,min:-60,max:12,defaultValue:0,unit:'dB',step:.1,precision:1,parseNative:parsePhysical('db'),accessibleNative:value=>`${Number(value.toFixed(1))} decibels`,...overrides});
    case 'integer':return createParameterDescriptor({kind,min:0,max:127,defaultValue:0,step:1,precision:0,discrete:true,parseNative:parseUnit,...overrides});
    case 'enum':{
      const values=overrides.values??[0,1];const labels=overrides.labels??values.map(String);
      const parser=text=>{const normalized=normalizedText(text).toLowerCase();const index=labels.findIndex(label=>label.toLowerCase()===normalized);return index>=0?values[index]:parseUnit(text);};
      return createParameterDescriptor({kind,values,defaultValue:values[0],discrete:true,formatNative:value=>labels[values.indexOf(value)],parseNative:parser,accessibleNative:value=>labels[values.indexOf(value)],...overrides});
    }
    case 'tempo-note':{
      const parser=text=>{const match=normalizedText(text).match(/^(\d+)\s*\/\s*(\d+)\s*(?:note|([dt]))$/i);if(!match||Number(match[2])===0)return null;const beats=4*Number(match[1])/Number(match[2]);return beats*(match[3]?.toLowerCase()==='d'?1.5:match[3]?.toLowerCase()==='t'?2/3:1);};
      return createParameterDescriptor({kind,min:.125,max:4,defaultValue:1,unit:'note',step:.125,precision:3,parseNative:parser,...overrides});
    }
    default:return createParameterDescriptor({kind,...overrides});
  }
}

export function stepRational(rational,direction,descriptor){
  const limit=rational.denominator;const minimum=descriptor.bipolar?-limit:0;
  const numerator=clamp(rational.numerator+Math.trunc(direction),minimum,limit);
  const next={numerator,denominator:rational.denominator};
  const position=descriptor.bipolar?.5+numerator/limit/2:numerator/limit;
  return {valid:true,nativeValue:descriptor.toNative(position),position,rational:next};
}

export const fractionDragSteps=(verticalPixels,shiftKey=false)=>Math.trunc(verticalPixels/(shiftKey?24:6));
