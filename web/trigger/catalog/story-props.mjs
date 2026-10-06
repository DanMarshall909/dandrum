import {shapePoints} from '../shell/modulator-shape.mjs';
import {envelopeGeometry} from '../shell/envelope-geometry.mjs';
const valueText=v=>Math.round(v*100)+'%';
const callbacks=(value,emit,path='model')=>{
  if(typeof value==='function')return (...args)=>{emit(path,args);return value(...args);};
  if(Array.isArray(value))return value.map((v,i)=>callbacks(v,emit,path+'['+i+']'));
  if(value&&Object.getPrototypeOf(value)===Object.prototype)return Object.fromEntries(Object.entries(value).map(([key,v])=>[key,key.endsWith('Ref')||/^(get|subscribe|fmt|format|parse)/.test(key)?v:callbacks(v,emit,path+'.'+key)]));
  return value;
};
export function storyProps(name,base,controls,emit){
  base=callbacks(base,emit);
  const {state,label,value,checked,shape}=controls,change=(...args)=>emit('onChange',args),noop=(...args)=>emit('callback',args);
  const model={...base,ann:false,isSample:true,isSlices:true,isMapping:true,isLayers:true,isVoice:true,isMod:true,isRouting:true,isFx:true,notPerf:true,
    kTight:base.kTight,viewport:{zoom:1,offset:0},setViewport:noop,patchName:label,assetName:label,assetChoices:[{id:base.assetId,label}],
    showTree:true,showBrowser:true,showRoutingGraph:true,isEmpty:false,isDrop:false,hasHist:true,
    insp:{...base.insp,title:label},me:{...base.me,name:label,shape},macros:base.macros.map((m,i)=>i?m:{...m,name:label,v:value}),
    openRail:noop,browseSamples:noop,replaceSample:noop,replaceRegion:noop};
  if(state==='empty'){for(const key of ['tree','assets','srcGroups','modSrcs','hist','zones','sliceRows','slices','cands','rrSeq','chain','voiceSecs','envs','routeGroups','routes','routeRows','insertLayers','fxLayers','macros','busses'])model[key]=[];model.insp={...model.insp,rows:[]};model.isEmpty=true;}
  if(state==='loading'){model.isLoading=true;model.waveReady=false;model.loadingProgress=value;model.trRun=true;model.trDone=false;model.analysisProgress=value;model.status={kind:'busy',title:'Loading',text:label};}
  if(state==='error'){model.missing=true;model.banner=true;model.trFail=true;model.trRun=false;model.analysisError='Simulated decoder failure';model.status={kind:'error',title:'Unavailable',text:label};}
  if(state==='selected'){model.insp.type='Selected';model.tree=model.tree.map((r,i)=>({...r,sel:i===0}));}
  const disabled=state==='disabled',long=state==='long text',text=long?label+' · a long source or destination name that must remain readable without losing controls':label;
  const options=[{id:'one',label:'One'},{id:'two',label:'Two'},{id:'three',label:'Three'}],items=[{label:'Preview',onSelect:()=>emit('audition',[])},{separator:true},{label:'Rename…',onSelect:()=>emit('renameNode',[])},{label:'Delete',danger:true,disabled,onSelect:()=>emit('removeNode',[])}];
  const envelope={...base.envs[0],title:text,shape,config:{...base.envs[0]?.config,sustain:value},onChange:change};
  const geometry=envelopeGeometry({kind:'envelope',shape,config:envelope.config});envelope.pts=geometry.pointsText;envelope.handles=geometry.handles;envelope.onPoint=noop;envelope.vals=envelope.vals?.map(v=>({...v,change, value:v.k==='Sustain'?value:v.value}));
  model.me.ghost=shapePoints({kind:'envelope',shape,config:envelope.config},null);model.me.pts=shapePoints({kind:'envelope',shape,config:envelope.config},null,value);
  const section={...base.voiceSecs[3],title:text,segV:shape,knobs:base.voiceSecs[3]?.knobs.map((k,i)=>({...k,disabled,v:i? k.v:value,t:i?k.t:valueText(value),set:change}))};
  const macro={...base.macros[0],name:text,v:value,host:state==='hover',off:disabled,set:change,bg:state==='selected'?'var(--dd-vermilion-wash)':'transparent'};
  const route={...base.routes[1],src:text,a:value*2-1,amt:valueText(value),l:value<.5?value*100:50,w:Math.abs(value-.5)*100,on:checked,setAmount:change};
  model.envs=model.envs.map((e,i)=>i?e:envelope);model.voiceSecs=model.voiceSecs.map((s,i)=>i===3?section:s);model.macros=model.macros.map((m,i)=>i?m:macro);
  const props={model,label:text,title:text,value,defaultValue:.5,valueText:valueText(value),checked,selected:state==='selected',active:state==='hover',disabled,onChange:change,onClick:noop,onClose:noop,onCancel:noop,onConfirm:noop,onSubmit:noop,onSelect:noop,onReceive:noop,
    children:text,icon:'level',name:'level',size:'md',options,items,width:260,height:100,slot:'A',source:text,depth:value,
    getTelemetry:model.getTelemetry,subscribeTelemetry:model.subscribeTelemetry,assetId:model.assetId,peaks:model.pianoPeaks,regionStart:.012,regionEnd:.94,fadeIn:.004,fadeOut:.08,
    slices:model.slices,onMarkerChange:noop,onRegionChange:noop,onViewport:noop,viewport:model.viewport,rows:model.routeRows,busses:model.busses,layers:model.insertLayers,outputs:model.outputs,
    zones:model.zones,selectedId:model.zoneSel,lowNote:24,highNote:96,gridHeight:160,onNoteOn:note=>emit('noteOn',[note,100]),onNoteOff:note=>emit('noteOff',[note]),
    row:{label:text,value:valueText(value),field:{value,min:0,max:1,step:.01,onChange:change}},route,envelope,section,macro,telemetry:model.getTelemetry(),
    getEvents:()=>[{time:Date.now(),kind:'call',name:'setParam',args:['Cutoff',value]}],onClear:noop,switches:{},onSwitch:noop,choices:['Filter','Amplifier'],sources:model.routeSources,destinations:model.routeDestinations,
    presets:[{id:'felt-kit',name:'Felt Kit',detail:'Keys · Drums · Break'}],onLoad:noop,target:text,dragging:state!=='empty',hint:text,visible:true,progress:value,
    job:{state:state==='error'?'failed':state==='loading'?'running':'done',kind:'pitch',progress:value,result:{note:60,cents:3},message:text,apply:noop,discard:noop,cancel:noop}};
  if(name==='RenameDialog')props.value=text;
  if(name==='VoiceTemplateDialog')props.model=base.template;
  if(name==='DD.Tabs'||name==='DD.SegmentedControl'){props.value='one';props.items=options;}
  if(name==='DD.PropertyRow')props.value=valueText(value);
  if(name==='DD.MenuButton'||name==='ChoiceMenu'){props.value='one';props.size='sm';}
  if(name==='DD.Icon'||name==='DD.ModGlyph')props.size=16;
  if(name==='DD.PadCell'){props.note=36;props.name=text;props.velocity=checked?100:0;}
  if(name==='DD.Meter')props.levels=[-14,-15];
  if(name==='DD.StatusMessage')props.kind=state==='error'?'error':state==='loading'?'busy':'ok';
  return props;
}
