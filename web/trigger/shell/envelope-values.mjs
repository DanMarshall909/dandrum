/** Paired visible units and canonical envelope values. */
export function envelopeValue(mod,key,onChange){
  const value=mod.config[key]??0;
  const k=key[0].toUpperCase()+key.slice(1);
  if(key==='sustain')return mod.id==='Amp envelope'?
    {k,value:20*Math.log10(Math.max(.001,value)),min:-60,max:0,step:.1,unit:'dB',format:v=>v.toFixed(1),change:v=>onChange(Math.pow(10,v/20))}:
    {k,value:value*100,min:0,max:100,step:.1,unit:'%',change:v=>onChange(v/100)};
  const scale=value<1?1000:1;
  return {k,value:value*scale,min:0,max:10*scale,step:scale===1000?.1:.001,unit:scale===1000?'ms':'s',change:v=>onChange(v/scale)};
}
