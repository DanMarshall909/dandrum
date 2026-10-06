import {bounded,entity} from './validation.mjs';
import {SelectorSim} from './selector-sim.mjs';
export class TelemetrySim {
  constructor(engine){this.engine=engine;this.active=[];this.playheads=[];this.selectorPos={};this.counter=0;this.timer=null;this.host=false;this.last=Date.now();this.selectors=new SelectorSim(this);}
  start(){if(this.timer)return;this.last=Date.now();this.timer=setInterval(()=>this.tick(),1000/30);this.timer.unref?.();}
  noteOn(note,velocity){
    const e=this.engine;e.call('noteOn',[note,velocity]);note=Math.round(bounded(note,0,127));velocity=Math.round(bounded(velocity,1,127));
    this.start();const rules=e.patch.rules.filter(r=>note>=r.noteLo&&note<=r.noteHi&&(r.target&&e.patch.selectors[r.target]||velocity>=r.velLo&&velocity<=r.velHi)),seen=new Set();
    for(const rule of rules){
      const source=e.patch.selectors[rule.target]?rule.target:rule.source;if(seen.has(source))continue;seen.add(source);
      let selected=this.selectors.resolve(source,note,velocity);
      const asset=e.patch.assets.find(a=>a.id===(e.patch.regions.find(r=>r.id===rule.source)?.assetId??rule.source));
      if(!selected.length&&asset?.state==='missing'){
        const fallback=e.patch.rules.find(r=>r.id!==rule.id&&r.group===rule.group&&r.target===rule.target&&r.root===rule.root);
        if(fallback)selected=this.selectors.resolve(fallback.source,note,velocity);
      }
      for(const layer of selected){const region=e.patch.regions.find(r=>r.id===layer),asset=e.patch.assets.find(a=>a.id===(region?.assetId??layer));if(asset)this.playheads.push({asset:asset.id,pos:region?.start??0,start:region?.start??0,end:region?.end??1,duration:asset.duration,note});this.active.push({id:++this.counter,note,vel:velocity,layer,remaining:1.5,released:false});}
    }
    const limit=e.patch.voicePolicy.mode==='poly'?e.patch.voicePolicy.limit:1;
    if(this.active.length>limit)this.active.splice(0,this.active.length-limit);
    this.tick(0);
  }
  noteOff(note){this.engine.call('noteOff',[note]);for(const voice of this.active)if(voice.note===note){voice.released=true;voice.remaining=Math.min(voice.remaining,.35);}}
  audition(id,region){
    const e=this.engine,asset=entity(e.patch.assets,id);if(asset.state!=='loaded')throw new Error('Cannot audition an unavailable asset');e.call('audition',[id,region]);this.start();
    this.playheads=this.playheads.filter(p=>p.asset!==id);this.playheads.push({asset:id,pos:region?.start??0,start:region?.start??0,end:region?.end??1,duration:asset.duration});
    this.active.push({id:++this.counter,note:region?.root??60,vel:96,layer:id,remaining:Math.max(.2,asset.duration*((region?.end??1)-(region?.start??0))),released:true});this.tick(0);
  }
  auditionSource(id,note=60,velocity=100){
    const e=this.engine;e.call('auditionSource',[id,note,velocity]);
    const layers=this.selectors.resolve(id,note,velocity);if(!layers.length)throw new Error('Source is unavailable');
    for(const layer of layers){const region=e.patch.regions.find(r=>r.id===layer),asset=e.patch.assets.find(a=>a.id===(region?.assetId??layer));
      if(asset)this.audition(asset.id,region);
      else{this.start();this.active.push({id:++this.counter,note,vel:velocity,layer,remaining:1.5,released:true});}
    }
    this.tick(0);
  }
  tick(elapsed){
    const now=Date.now(),dt=elapsed??Math.max(0,(now-this.last)/1000);this.last=now;
    this.active.forEach(v=>v.remaining-=dt);this.active=this.active.filter(v=>v.remaining>0);
    this.playheads.forEach(p=>p.pos+=dt/p.duration);this.playheads=this.playheads.filter(p=>p.pos<p.end);
    const e=this.engine,phase=now/1000;
    if(this.host){const macro=e.patch.macros[2];if(macro){macro.value=.5+.3*Math.sin(phase);macro.host=true;e.emit('host',{id:macro.id,value:macro.value,host:true});}}
    e.emit('telemetry',{voices:{active:this.active.length,max:e.patch.voicePolicy.limit},notes:this.active.map(({note,vel,layer})=>({note,vel,layer})),playheads:this.playheads.map(({asset,pos})=>({asset,pos})),selectorPos:this.selectorPos,
      mod:Object.fromEntries(e.patch.routes.map((r,i)=>[r.id,r.on?(Math.sin(phase*2+i)+1)/2:0])),meters:Object.fromEntries(e.patch.buses.map(b=>[b.id,this.active.length?[ -18+Math.min(12,this.active.length),-19+Math.min(12,this.active.length)]:[-90,-90]])),host:Object.fromEntries(e.patch.macros.map(m=>[m.id,!!m.host])),hostValues:Object.fromEntries(e.patch.macros.filter(m=>m.host).map(m=>[m.id,m.value]))});
  }
  reset(){this.stop();this.selectorPos={};this.selectors.reset();this.tick(0);this.start();}
  stop(){clearInterval(this.timer);this.timer=null;this.active=[];this.playheads=[];}
}
