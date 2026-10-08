// Normalised mock controls use paired display/entry conversions. Anchor values
// preserve the received fixture; the real adapter owns its DSP transfer functions.
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
  return {format:v=>format(display(v)),parse:text=>{
    const raw=String(text).trim().replace('−','-').replace('±','');
    if(['Pan','Lush Pan'].includes(label)&&raw.toUpperCase()==='C')return .5;
    let n=Number.parseFloat(raw.replace(/^[LR]/i,''));if(!Number.isFinite(n))return null;
    if(label==='Cutoff'&&/khz/i.test(raw))n*=1000;
    if(['Pan','Lush Pan'].includes(label)&&/^L/i.test(raw))n=-n;
    return clamp(normal(n));
  }};
}
