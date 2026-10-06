import {envelopeGeometry} from './envelope-geometry.mjs';
export function modulatorShape(mod,t){
  const c=mod.config,shape=mod.shape;
  if(mod.kind==='lfo'){
    const p=(t*(c.rate??2.4)+ (c.phase??0))%1;
    return shape==='tri'?1-4*Math.abs(p-.5):shape==='saw'?1-2*p:shape==='sq'?(p<.5?1:-1):shape==='sh'?[.6,-.3,.9,-.8,.2,-.5,.75,-.1][Math.floor(t*8)%8]:Math.sin(p*2*Math.PI);
  }
  if(mod.kind==='envelope'){
    const points=envelopeGeometry(mod).points,index=points.findIndex(p=>p[0]>=t),i=index<0?points.length-1:Math.max(0,index-1),a=points[i],b=points[i+1]??a;
    return a[1]+(b[1]-a[1])*Math.max(0,Math.min(1,(t-a[0])/Math.max(.0001,b[0]-a[0])));
  }
  if(mod.kind==='random'){
    const values=[.3,-.7,.55,.1,-.4,.85,-.2,.6,-.9,.25,.3],at=Math.min(9,Math.floor(t*10)),v=values[at];
    return shape==='smooth'?v+(values[at+1]-v)*(.5-.5*Math.cos((t*10%1)*Math.PI)):v;
  }
  const x=Math.max(0,Math.min(1,(t-(c.inLow??0))/Math.max(.0001,(c.inHigh??1)-(c.inLow??0))));
  const value=shape==='exp'?x*x:shape==='log'?Math.sqrt(x):shape==='s'?x*x*(3-2*x):x;return c.invert?1-value:value;
}
export function shapePoints(mod,route,amount=1){
  const bi=route?.polarity==='Bi';return Array.from({length:241},(_,i)=>{const t=i/240,raw=modulatorShape(mod,t),v=bi?raw:Math.max(0,['lfo','random'].includes(mod.kind)?(raw+1)/2:raw);
    return (t*300).toFixed(1)+','+(bi?50-v*50*amount:100-v*100*amount).toFixed(1);}).join(' ');
}
