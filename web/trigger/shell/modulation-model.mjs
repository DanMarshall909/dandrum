import {modulatorKnobs} from './modulator-knobs.mjs';
import {shapePoints} from './modulator-shape.mjs';
import {parameterUnits} from './parameter-units.mjs';
export const destinationFor=label=>({Cutoff:'Filter cutoff',Reso:'Filter resonance',Drive:'Filter drive',Tune:'Pitch',Start:'Sample start',Level:'Amp gain',Gain:'Sample gain'}[label]??label);
export function modulationModel(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,command=(label,method,...args)=>presenter.command(label,method,...args),selected=patch.routes.find(r=>r.id===presenter.state.selectedRoute)??patch.routes[1]??patch.routes[0];
  const mod=patch.modulators.find(m=>m.id===(presenter.state.modInsp?presenter.state.pickedMod:selected?.source))??patch.modulators.find(m=>m.id===selected?.source)??patch.modulators[0];
  const amount=value=>(value>=0?'+':'')+Math.round(value*100)+'%',color=slot=>slot?'var(--dd-mod-'+slot.toLowerCase()+')':'var(--dd-paper-3)';
  const openDestination=(e,source)=>{const pt=presenter.winPt(e,264,300);presenter.setState({dmenu:{...pt,src:source},cmenu:null,kmenu:null});};
  model.addModulationRoute=()=>presenter.setState({dialog:{kind:'add-route'}});model.addModulator=()=>presenter.setState({dialog:{kind:'add-modulator'}});model.routeCount=patch.routes.length;model.groupBy=presenter.state.groupBy??'src';model.setGroupBy=value=>presenter.setState({groupBy:value});
  model.routes=patch.routes.map(r=>{const source=patch.modulators.find(m=>m.id===r.source),slot=source?.slot,s=r.id===selected?.id;return {...r,rk:r.id,src:source?.name??r.source,dest:r.destination,a:r.amount,amt:amount(r.amount),pol:r.polarity,slot,noSlot:!slot,
    fixed:!!r.locked,l:r.amount>=0?50:50+r.amount*50,w:Math.abs(r.amount)*50,c:color(slot),live:0,op:r.on?1:.5,indent:0,top:true,bg:s?'var(--dd-vermilion-wash)':'transparent',edge:s?'inset 2px 0 0 var(--dd-vermilion)':'none',
    select:()=>presenter.setState({selectedRoute:r.id,modInsp:false,editRoute:null}),setAmount:value=>command('Change route amount','updateRoute',r.id,{amount:value}),
    toggle:()=>command('Toggle modulation route','updateRoute',r.id,{on:!r.on})};});
  const groups=new Map();for(const route of model.routes){const key=model.groupBy==='dst'?route.dest:route.source;if(!groups.has(key))groups.set(key,[]);groups.get(key).push(route);}
  model.routeGroups=[...groups].map(([key,items])=>{const multi=items.length>1,open=presenter.state.routeOpen?.[key]!==false,byDest=model.groupBy==='dst';return {id:key,name:byDest?key:items[0].src,multi,show:!multi||open,slot:byDest?null:items[0].slot,noSlot:byDest||items[0].noSlot,count:items.length,
    chev:open?'chevron-down':'chevron-right',summary:items.map(r=>(byDest?r.src:r.dest)+' '+r.amt).join(' · '),live:0,
    items:items.map(r=>({...r,child:multi&&!byDest,top:!multi||byDest,indent:multi?22:0})),toggle:()=>presenter.setState({routeOpen:{...presenter.state.routeOpen,[key]:!open}}),
    addDest:e=>{e.stopPropagation();openDestination(e,items[0].source);}};});
  const categories=[['env','Envelopes','envelope'],['lfo','LFOs','lfo'],['perf','Performance','performance'],['macro','Macros','macro'],['rand','Random','random']];
  model.modSrcs=categories.map(([id,cat,kind])=>{const list=patch.modulators.filter(m=>m.kind===kind),open=!!presenter.state.openCats?.[id];return {id,cat,open,chev:open?'chevron-down':'chevron-right',slots:list.map(m=>m.slot).filter(Boolean),
    summary:list.length+' · '+patch.routes.filter(r=>list.some(m=>m.id===r.source)).length,title:list.length+' sources',hbg:list.some(m=>m.id===mod?.id)?'var(--dd-vermilion-wash)':'transparent',
    toggle:()=>presenter.setState({openCats:{...presenter.state.openCats,[id]:!open}}),items:list.map(m=>({id:m.id,routeIds:patch.routes.filter(r=>r.source===m.id).map(r=>r.id),name:m.name,slot:m.slot,noSlot:!m.slot,n:patch.routes.filter(r=>r.source===m.id).length,live:0,
      bg:m.id===mod?.id?'var(--dd-vermilion-wash)':'transparent',edge:m.id===mod?.id?'inset 2px 0 0 var(--dd-vermilion)':'none',tip:'Click for details · drag onto a control to add a route',
      pick:()=>{presenter.go('mod');presenter.setState({pickedMod:m.id,modInsp:true});},drag:e=>{e.dataTransfer.setData('text/plain',m.id);presenter.setState({dragMod:{id:m.id,name:m.name,slot:m.slot}});}}))};});
  model.onKnobDrop=e=>{const drag=presenter.state.dragMod,target=e.target.closest('[data-knob]');if(!drag||!target)return;e.preventDefault();e.stopPropagation();
    presenter.command('Add modulation route','addRoute',{source:drag.id,destination:destinationFor(target.dataset.knob),amount:e.altKey?-.25:.25,polarity:'Uni',curve:'Linear',on:true});presenter.setState({dragMod:null,dropOver:null});};
  model.tabs=model.tabs.map(tab=>tab.id==='mod'?{...tab,badge:String(patch.routes.length)}:tab);
  const knobMods=key=>patch.routes.filter(r=>r.destination===destinationFor(key)&&r.on).map(r=>({slot:patch.modulators.find(m=>m.id===r.source)?.slot,source:patch.modulators.find(m=>m.id===r.source)?.name??r.source,depth:r.amount}));
  for(const section of [...model.voiceSecs,...model.sampleKnobGroups])for(const knob of section.knobs)knob.mods=knobMods(knob.key);
  model.cutoffMods=knobMods('Cutoff');model.cutoffValue=patch.params.Cutoff?.value??0;model.cutoffUnits=parameterUnits('Cutoff');model.setCutoff=value=>presenter.changeParameter('Cutoff',value);
  if(mod){
    const lfo=mod.kind==='lfo',env=mod.kind==='envelope',rand=mod.kind==='random',current=patch.routes.find(r=>r.source===mod.id&&r.id===selected?.id)??patch.routes.find(r=>r.source===mod.id),a=current?.amount??.25;
    const shapeList=lfo?[['sine','Sine'],['tri','Tri'],['saw','Saw'],['sq','Square'],['sh','S&H']]:env?[['adsr','ADSR'],['ahdsr','AHDSR'],['multi','Multi']]:rand?[['step','Steps'],['smooth','Smooth']]:[['linear','Linear'],['exp','Exp'],['log','Log'],['s','S-curve']];
    const toggles=lfo?[['tempoSync','Tempo sync'],['retrigger','Retrigger on note'],['perVoice','Per voice']]:env?[['retrigger','Retrigger'],['velocityScale','Velocity scales level'],['loop','Loop']]:rand?[['newPerNote','New value per note'],['perVoice','Per voice']]:[['invert','Invert'],['perVoice','Per voice']];
    model.me={id:mod.id,routeId:current?.id,name:mod.name,slot:mod.slot,dest:patch.routes.filter(r=>r.source===mod.id).map(r=>r.destination).join(', ')||'Unassigned',amt:amount(a),pol:current?.polarity==='Bi'?'bipolar':'unipolar',bi:current?.polarity==='Bi',col:color(mod.slot),
      shapes:shapeList.map(([id,label])=>({id,label})),shape:mod.shape,setShape:shape=>command('Change '+mod.name+' shape','setModulator',mod.id,{shape}),ghost:shapePoints(mod,current),pts:shapePoints(mod,current,a),live:0,yTop:current?.polarity==='Bi'?'+100%':'100%',yBot:current?.polarity==='Bi'?'−100%':'0',xLabel:lfo?(mod.config.rate??2.4).toFixed(2)+' Hz':env?'time · note on → release':rand?'per note':'input 0–100%',
      knobs:modulatorKnobs(presenter,mod,current),toggles:toggles.map(([key,label])=>({label,on:!!mod.config[key],change:value=>command('Change '+mod.name+' '+label,'setModulator',mod.id,{config:{...mod.config,[key]:value}})})),addDest:e=>openDestination(e,mod.id)};
  }else model.isMod=false;
  presenter.controlKnobs=[...model.sampleKnobGroups.flatMap(g=>g.knobs),...model.voiceSecs.flatMap(s=>s.knobs),...(model.me?.knobs??[]),...(model.processorControls??[]),...model.insp.rows.filter(row=>row.knobs).flatMap(row=>row.list)];
  if(model.page==='mod'&&mod){
    const currentRoute=presenter.state.modInsp?patch.routes.find(r=>r.source===mod.id):selected;
    const property=(label,value,field)=>({p:true,label,value,field});
    model.insp={icon:'modulate',title:currentRoute?model.routes.find(r=>r.id===currentRoute.id)?.src+' → '+currentRoute.destination:mod.name,type:currentRoute?'Route':'Modulator',ann:'Modulation Route',rows:[{h:true,label:'Route'},property('Source',mod.name),
      ...(currentRoute?[property('Destination',currentRoute.destination),property('Amount',currentRoute.amount*100,{value:currentRoute.amount*100,min:-100,max:100,step:1,onChange:v=>command('Change route amount','updateRoute',currentRoute.id,{amount:v/100})}),
        property('Polarity',currentRoute.polarity,{value:currentRoute.polarity,options:['Uni','Bi'].map(id=>({id,label:id})),onChange:value=>command('Change route polarity','updateRoute',currentRoute.id,{polarity:value})}),property('Curve',currentRoute.curve,{value:currentRoute.curve,options:['Linear','Exp','Log','S-curve'].map(id=>({id,label:id})),onChange:value=>command('Change route curve','updateRoute',currentRoute.id,{curve:value})})]:[]),
      {h:true,label:'Source'},property('Shape',mod.shape),property('Used by',patch.routes.filter(r=>r.source===mod.id).length+' routes'),{b:true,label:'Add destination…',icon:'plus',action:e=>openDestination(e,mod.id)}]};
    model.crumbs=[patch.name,'Modulation',mod.name].map((label,i)=>({label,sep:i<2,c:'var(--dd-paper-2)'}));
    presenter.routeKeyboard={remove:()=>selected&&!selected.locked&&command('Remove route','removeRoute',selected.id)};
  }
}
