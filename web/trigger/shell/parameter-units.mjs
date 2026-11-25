// Normalised mock controls use paired display/entry conversions. Anchor values
// preserve the received fixture; the real adapter owns its DSP transfer functions.
import {createParameterDescriptor} from './parameter-descriptor.mjs';
const clamp=v=>Math.max(0,Math.min(1,v)),signed=n=>n<0?'−'+Math.abs(n):n>0?'+'+n:String(n);
const anchor=(v,middle,shown,min,max)=>v<=middle?min+(shown-min)*v/middle:shown+(max-shown)*(v-middle)/(1-middle);
const unanchor=(n,middle,shown,min,max)=>n<=shown?middle*(n-min)/(shown-min):middle+(1-middle)*(n-shown)/(max-shown);
export function parameterUnits(label,duration=4.82){
  let display=v=>v*100,normal=n=>n/100,format=n=>Math.round(n)+'%';
  if(label==='Start'||label==='End'){display=v=>v*duration;normal=n=>n/duration;format=n=>n.toFixed(3)+' s';}
  else if(['Tune','Lush Tune'].includes(label)){display=v=>(v-.5)*48;normal=n=>n/48+.5;format=n=>signed(Math.round(n*10)/10)+' st';}
  else if(label==='Fine'){display=v=>(v-.5)*100;normal=n=>n/100+.5;format=n=>signed(Math.round(n))+' ct';}
  else if(['Pan','Lush Pan'].includes(label)){display=v=>(v-.5)*200;normal=n=>n/200+.5;format=n=>Math.abs(n)<.5?'C':(n<0?'L':'R')+Math.round(Math.abs(n));}
  else if(label==='Bend'){display=v=>v/.17*2;normal=n=>n/2*.17;format=n=>'±'+Math.round(n*10)/10+' st';}
  else if(label==='Env amt'){display=v=>(v-.5)*200;normal=n=>n/200+.5;format=n=>signed(Math.round(n))+'%';}
  else if(label==='Cutoff'){display=v=>anchor(v,.42,2400,20,20000);normal=n=>unanchor(n,.42,2400,20,20000);format=n=>n>=1000?(n/1000).toFixed(2)+' kHz':Math.round(n)+' Hz';}
  else if(['Gain','Level','Lush Level'].includes(label)){
    const [middle,shown]=label==='Gain'?[.72,-1.5]:label==='Level'?[.62,-.8]:[.55,-6];
    display=v=>anchor(v,middle,shown,-60,6);normal=n=>unanchor(n,middle,shown,-60,6);format=n=>signed(Number(n.toFixed(1)))+' dB';
  }else if(label==='Drive'){display=v=>v*15;normal=n=>n/15;format=n=>n.toFixed(1)+' dB';}
  const parseNative=text=>{
    const raw=String(text).trim().replace('−','-').replace('±','');
    if(['Pan','Lush Pan'].includes(label)&&raw.toUpperCase()==='C')return .5;
    const unit=label==='Cutoff'?'(?:k?Hz)':label==='Start'||label==='End'?'s':label==='Fine'?'(?:ct|cents?)':label==='Tune'||label==='Lush Tune'||label==='Bend'?'(?:st|semitones?)':['Gain','Level','Lush Level','Drive'].includes(label)?'dB':'%';
    const match=raw.match(new RegExp(`^([LR])?\\s*([+-]?(?:\\d+(?:\\.\\d*)?|\\.\\d+))\\s*${unit}?$`,'i'));
    if(!match)return null;
    let n=Number(match[2]);
    if(label==='Cutoff'&&/khz/i.test(raw))n*=1000;
    if(['Pan','Lush Pan'].includes(label)&&match[1]?.toUpperCase()==='L')n=-Math.abs(n);
    if(['Pan','Lush Pan'].includes(label)&&match[1]?.toUpperCase()==='R')n=Math.abs(n);
    return clamp(normal(n));
  };
  const accessible=value=>{
    const shown=display(value);
    if(['Pan','Lush Pan'].includes(label))return Math.abs(shown)<.5?'centre':`${Math.round(Math.abs(shown))} percent ${shown<0?'left':'right'}`;
    return format(shown).replace('−','minus ').replace('±','plus or minus ').replace(/\bst\b/,'semitones').replace(/\bct\b/,'cents').replace(/\bdB\b/,'decibels').replace(/\bkHz\b/,'kilohertz').replace(/\bHz\b/,'hertz');
  };
  const descriptor=createParameterDescriptor({
    kind:['Pan','Lush Pan'].includes(label)?'pan':['Tune','Lush Tune','Fine','Env amt'].includes(label)?'bipolar':'physical',
    min:0,max:1,defaultValue:.5,step:.01,fineStep:.001,coarseStep:.1,bipolar:['Pan','Lush Pan','Tune','Lush Tune','Fine','Env amt'].includes(label),
    formatNative:value=>format(display(value)),parseNative,
    accessibleNative:accessible,
  });
  return {descriptor,format:value=>descriptor.format(descriptor.toPosition(value)),parse:text=>{const result=descriptor.parseEdit(text);return result.valid?result.nativeValue:null;}};
}
