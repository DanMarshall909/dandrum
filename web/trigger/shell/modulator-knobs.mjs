import {createParameterDescriptor,createParameterVariant,parsePhysical} from './parameter-descriptor.mjs';
export function modulatorKnobs(presenter,mod,route){
  const isLfo=mod.kind==='lfo',isEnv=mod.kind==='envelope',keys=isLfo?['rate','phase','fadeIn','smooth']:isEnv?['attack','decay','sustain','release']:mod.kind==='random'?['smooth']:['inLow','inHigh','smooth'];
  const names={inLow:'In low',inHigh:'In high',fadeIn:'Fade in'},maxima={rate:20,attack:10,decay:10,release:10,fadeIn:10};
  const knobs=keys.map(key=>{
    const max=maxima[key]??1,value=mod.config[key]??(key==='inHigh'?1:0),label=names[key]??key[0].toUpperCase()+key.slice(1);
    const descriptor=max===1?createParameterVariant('percent'):createParameterDescriptor({kind:key==='rate'?'frequency':'time',min:0,max:1,step:.001,formatNative:position=>key==='rate'?(position*max).toFixed(2)+' Hz':(position*max).toFixed(3)+' s',parseNative:text=>{const n=parsePhysical(key==='rate'?'hz':'s')(text);return Number.isFinite(n)?n/max:null;}});
    return {key:mod.id+' '+key,label,v:value/max,default:key==='rate'?2.4/max:key==='sustain'?.5:0,t:key==='rate'?value.toFixed(2)+' Hz':max===10?value.toFixed(3)+' s':Math.round(value*100)+'%',descriptor,
      parse:text=>{const result=descriptor.parseEdit(text);return result.valid?result.nativeValue:null;},mods:[],set:v=>presenter.command('Change '+mod.name+' '+label,'setModulator',mod.id,{config:{...mod.config,[key]:v*max}})};
  });
  if(route){const descriptor=createParameterVariant('bipolar-percent');knobs.push({key:route.id+' amount',label:'Amount',v:route.amount,default:.25,bi:true,t:Math.round(route.amount*100)+'%',descriptor,mods:[],parse:text=>{const result=descriptor.parseEdit(text);return result.valid?result.nativeValue:null;},
    set:value=>presenter.command('Change route amount','updateRoute',route.id,{amount:value})});
  }
  return knobs;
}
