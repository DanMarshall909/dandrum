export const parameterDefaults={Start:.012,End:.94,'Fade in':.04,'Fade out':.3,Tune:.5,Fine:.47,Gain:.72,Pan:.5,Cutoff:.42,Reso:.22,Drive:.1,'Key trk':.5,'Env amt':.71,Level:.62,'Vel sens':.3,
  Attack:.15,Decay:.4,Sustain:.5,Release:.5,Rate:.45,Phase:0,Smooth:0,Amount:.42,'Key track':1,Bend:.17,
  'Lush Level':.55,'Lush Pan':.5,'Lush Tune':.5,'Lush Detune':.34,'Lush Brightness':.6,'Lush Width':.7,'In low':0,'In high':1};
export const defaultParameters=()=>Object.fromEntries(Object.entries(parameterDefaults).map(([id,value])=>[id,{id,name:id,value,default:value,min:0,max:1,unit:'normalised'}]));
export function ensureVoice(p){
  if(p.modules.length)return;
  p.params=defaultParameters();
  p.modules=[{id:'sample',type:'Sample player',params:{mode:'once'}},{id:'lush',type:'Lush',params:{mode:'note'}},{id:'filter',type:'Filter',params:{type:'lp',slope:24}},{id:'amp',type:'Amplifier',params:{}}];
  p.modulators=[['Amp envelope','envelope',null],['Filter envelope','envelope','A'],['LFO 1','lfo','B'],['Velocity','performance','C']].map(([id,kind,slot])=>({id,name:id,kind,slot,shape:kind==='lfo'?'sine':kind==='envelope'?'adsr':'linear',config:{attack:.012,decay:.38,sustain:.5,release:1.2,rate:2.4,phase:0,smooth:0},locked:id==='Amp envelope'}));
  p.routes=[{id:'amp-route',source:'Amp envelope',destination:'Amp gain',amount:1,polarity:'Uni',curve:'Exp',on:true,locked:true}];
}

export const moduleControls={
  'Sample player':{Start:'start',Gain:'gain',Tune:'tune',Fine:'fine','Key track':'keyTrack',Bend:'bend'},
  Lush:{'Lush Level':'level','Lush Pan':'pan','Lush Tune':'tune','Lush Detune':'detune','Lush Brightness':'brightness','Lush Width':'width'},
  Filter:{Cutoff:'cutoff',Reso:'resonance',Drive:'drive','Key trk':'keyTrack','Env amt':'envAmount'},
  Amplifier:{Level:'level',Pan:'pan','Vel sens':'velocitySensitivity'}
};
export function moduleParameterId(module,label){return module.paramIds?.[label]??label;}
export function initialiseModule(p,module){
  const base=['sample','lush','filter','amp'].includes(module.id);
  module.paramIds??={};module.params??={};
  for(const [label,key] of Object.entries(moduleControls[module.type]??{})){
    const id=module.paramIds[label]??(base?label:module.id+' '+label);module.paramIds[label]=id;
    const value=module.params[key]??p.params[id]?.value??parameterDefaults[label]??.5;
    p.params[id]??={id,name:label,value,default:parameterDefaults[label]??.5,min:0,max:1,unit:'normalised'};
    module.params[key]=p.params[id].value;
  }
}
export function setModuleValue(p,module,key,value){
  module.params[key]=structuredClone(value);
  const label=Object.entries(moduleControls[module.type]??{}).find(([label,k])=>k===key||label===key)?.[0];
  if(label){const id=moduleParameterId(module,label);if(p.params[id])p.params[id].value=value;}
}
export function synchroniseModuleParameters(p,id,value){for(const module of p.modules){const label=Object.keys(moduleControls[module.type]??{}).find(label=>moduleParameterId(module,label)===id);if(label)module.params[moduleControls[module.type][label]]=value;}}
