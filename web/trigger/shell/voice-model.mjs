import {shapePoints} from './modulator-shape.mjs';
import {moduleControls,moduleParameterId} from '../engine/voice-defaults.mjs';
import {parameterUnits} from './parameter-units.mjs';
import {envelopeGeometry} from './envelope-geometry.mjs';
import {envelopeValue} from './envelope-values.mjs';
const p=(label,value,field)=>({p:true,label,value,field});
export function voiceModel(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,command=(label,method,...args)=>presenter.command(label,method,...args);
  const modules=patch.modules,selected=modules.find(m=>m.id===presenter.state.selectedModule)??modules.find(m=>m.id==='filter')??modules[0];
  const select=id=>presenter.setState({selectedModule:id});
  model.chain=modules.map((module,i)=>({id:module.id,label:module.type,select:()=>select(module.id),arrow:true,sep:i===0&&modules[1]?.type==='Lush'?'+':'→',
    bg:module.id===selected?.id?'var(--dd-vermilion-wash)':'var(--dd-ink-4)',bd:module.id===selected?.id?'var(--dd-vermilion)':'var(--dd-line-2)'})).concat([{id:'out',label:'Group out',bg:'transparent',bd:'var(--dd-line-2)',arrow:false,select:()=>presenter.go('routing')}]);
  const sourceSections=model.voiceSecs;
  const templates={'Sample player':[sourceSections[0],sourceSections[2]],Lush:[sourceSections[1]],Filter:[sourceSections[3]],Amplifier:[sourceSections[4]]};
  model.voiceSecs=modules.flatMap(module=>(templates[module.type]??[]).map(section=>{
    const key=module.type==='Filter'?'type':'mode';return {...section,id:module.id+'-'+section.title,sub:module.type==='Lush'?'Lush · Dandrum synth':module.type==='Filter'?'Ladder '+(module.params.slope??24)+' dB':module.type,segV:module.params[key]??section.segV,
      setSeg:value=>command('Change '+module.type+' '+key,'setModuleParam',module.id,key,value),select:()=>select(module.id),
      knobs:section.knobs.map(knob=>{const id=moduleParameterId(module,knob.key),value=patch.params[id]?.value??module.params[moduleControls[module.type]?.[knob.key]]??knob.v;
        const units=parameterUnits(knob.key);return {...knob,key:id,v:value,t:units.format(value),parse:units.parse,set:value=>presenter.changeParameter(id,value)};})};
  }));
  if(modules[0]?.type==='Sample player'&&modules[1]?.type==='Lush'){const pitch=model.voiceSecs.splice(1,1)[0];model.voiceSecs.splice(2,0,pitch);}
  model.editTemplate=()=>presenter.setState({dialog:{kind:'voice-template'}});
  model.template={modules,add:type=>command('Add voice module','setTemplate',[...modules,{id:'voice-'+Date.now(),type,params:{}}]),
    remove:id=>command('Remove voice module','setTemplate',modules.filter(m=>m.id!==id)),move:(id,direction)=>{const next=[...modules],index=next.findIndex(m=>m.id===id),to=Math.max(0,Math.min(next.length-1,index+direction));[next[index],next[to]]=[next[to],next[index]];return command('Move voice module','setTemplate',next);}};
  const config=(mod,key,value)=>command('Change '+mod.name+' '+key,'setModulator',mod.id,{config:{...mod.config,[key]:value}});
  model.envs=patch.modulators.filter(m=>m.kind==='envelope').slice(0,2).map(mod=>{
    const geometry=envelopeGeometry(mod);
    return {id:mod.id,title:mod.name,slot:mod.slot,dest:'→ '+patch.routes.filter(route=>route.source===mod.id).map(route=>route.destination).join(', '),ann:'Envelope modulator → Route',
      shape:mod.shape,setShape:value=>command('Change envelope shape','setModulator',mod.id,{shape:value}),pts:geometry.pointsText,sx:geometry.sustainX,
      liveRoute:patch.routes.find(route=>route.source===mod.id)?.id,config:mod.config,onChange:(key,value)=>config(mod,key,value),handles:geometry.handles,
      onPoint:(key,x,y)=>{if(key.startsWith('point-')){const points=geometry.points.map(p=>[...p]),i=Number(key.slice(6));points[i]=[Math.max(points[i-1]?.[0]??0,Math.min(points[i+1]?.[0]??1,x)),Math.max(0,Math.min(1,1-y))];return config(mod,'points',points);}return config(mod,key,geometry.valueFor(key,x,y));},
      vals:['attack',...(mod.shape==='ahdsr'?['hold']:[]),'decay','sustain','release'].map(key=>envelopeValue(mod,key,value=>config(mod,key,value)))};
  });
  const policy=patch.voicePolicy;model.voiceMode=policy.mode;model.setVoiceMode=mode=>command('Change voice mode','setVoicePolicy',{mode});
  const numeric=(label,key,min,max,step=1)=>p(label,policy[key]??0,{value:policy[key]??0,min,max,step,onChange:value=>command('Change '+label,'setVoicePolicy',{[key]:value})});
  const choice=(label,key,values)=>p(label,policy[key],{value:policy[key],options:values.map(id=>({id,label:id})),onChange:value=>command('Change '+label,'setVoicePolicy',{[key]:value})});
  model.voicePolicyRows=[numeric('Voice limit','limit',1,128),choice('Steal','steal',['oldest','oldest-released','quietest']),choice('Same note','sameNote',['retrigger','reuse','stack']),numeric('Exclusive group','exclusiveGroup',0,127),{...numeric('Glide','glide',0,10,.001),unit:'s'}];
  if(model.page!=='voice'||presenter.state.selectedMacro)return;
  const controls=Object.keys(moduleControls[selected?.type]??{}),first=controls[0],firstId=selected?moduleParameterId(selected,first):null;
  const destination=selected?.type==='Filter'?'Filter cutoff':selected?.type==='Amplifier'?'Amp gain':selected?.type==='Sample player'?'Sample start':'Lush Level';
  const incoming=patch.routes.filter(route=>route.destination===destination),modulator=id=>patch.modulators.find(m=>m.id===id);
  const typeField=selected?.type==='Filter'?p('Type',selected.params.type,{value:selected.params.type,options:[['lp','Ladder · LP'],['bp','Band pass'],['hp','High pass'],['nt','Notch']].map(([id,label])=>({id,label})),onChange:value=>command('Change filter type','setModuleParam',selected.id,'type',value)}):p('Type',selected?.type);
  model.insp={icon:'settings',title:selected?.type??'Voice',type:'Module',ann:'Module params + incoming Modulation Routes',rows:selected?[
    {h:true,label:first+' · '+parameterUnits(first).format(patch.params[firstId]?.value??.5)},
    ...incoming.map(route=>({m:true,label:modulator(route.source)?.name??route.source,slot:modulator(route.source)?.slot,depth:route.amount})),
    {b:true,label:'Assign modulation…',icon:'modulate',action:e=>{const pt=presenter.winPt(e,264,240);presenter.setState({kmenu:{...pt,label:firstId,path:['mod','add']}});}},
    {h:true,label:'Module'},typeField,
    ...(selected.type==='Filter'?[{...p('Slope',selected.params.slope,{value:selected.params.slope,min:12,max:48,step:12,onChange:value=>command('Change filter slope','setModuleParam',selected.id,'slope',value)}),unit:'dB/oct'}]:[]),
    p('Position',(modules.indexOf(selected)+1)+' of '+modules.length+' in voice'),p('Bypass',selected.bypassed?'On':'Off'),
    {b:true,label:selected.bypassed?'Enable module':'Bypass module',action:()=>command('Toggle module bypass','bypass',selected.id,!selected.bypassed)}]:[{t:true,label:'Choose a voice template.'}]};
  model.crumbs=[patch.name,'Voice',selected?.type??'Template'].map((label,i)=>({label,sep:i<2,c:'var(--dd-paper-2)'}));
}
