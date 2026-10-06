export function modulatorKnobs(presenter,mod,route){
  const isLfo=mod.kind==='lfo',isEnv=mod.kind==='envelope',keys=isLfo?['rate','phase','fadeIn','smooth']:isEnv?['attack','decay','sustain','release']:mod.kind==='random'?['smooth']:['inLow','inHigh','smooth'];
  const names={inLow:'In low',inHigh:'In high',fadeIn:'Fade in'},maxima={rate:20,attack:10,decay:10,release:10,fadeIn:10};
  const knobs=keys.map(key=>{
    const max=maxima[key]??1,value=mod.config[key]??(key==='inHigh'?1:0),label=names[key]??key[0].toUpperCase()+key.slice(1);
    return {key:mod.id+' '+key,label,v:value/max,default:key==='rate'?2.4/max:key==='sustain'?.5:0,t:key==='rate'?value.toFixed(2)+' Hz':max===10?value.toFixed(3)+' s':Math.round(value*100)+'%',
      parse:text=>{const n=Number.parseFloat(String(text).replace('−','-'));return Number.isFinite(n)?n/(max===1?100:max):null;},mods:[],set:v=>presenter.command('Change '+mod.name+' '+label,'setModulator',mod.id,{config:{...mod.config,[key]:v*max}})};
  });
  if(route)knobs.push({key:route.id+' amount',label:'Amount',v:(route.amount+1)/2,default:.625,bi:true,t:Math.round(route.amount*100)+'%',mods:[],parse:text=>(Number.parseFloat(text)/100+1)/2,
    set:value=>presenter.command('Change route amount','updateRoute',route.id,{amount:value*2-1})});
  return knobs;
}
