import {entityDelta} from './entity-delta.mjs';
const scopes={
  addNode:['nodes'],removeNode:['nodes','rules','selectors'],renameNode:['nodes','name'],duplicateNode:['nodes','rules','selectors'],moveNode:['nodes'],
  addRule:['rules'],updateRule:['rules'],removeRule:['rules'],setRules:['rules'],
  setRegion:['regions','history'],splitSlice:['regions','rules','history'],mergeSlices:['regions','rules','history'],removeSlice:['regions','rules','history'],
  setSlices:['regions','rules','history'],moveSliceBoundary:['regions','history'],mapSlices:['regions','rules','history'],editSlices:['regions','rules','history'],removeSlices:['regions','rules','history'],
  setHistory:['history','regions','rules'],reorderHistory:['history','regions','rules'],addHistory:['history','regions','rules'],removeHistory:['history','regions','rules'],collapseHistory:['history','regions','assets'],
  setSelector:['selectors'],reorderCandidates:['selectors'],setCandidate:['selectors'],
  addCandidate:['selectors'],removeCandidate:['selectors'],assignSource:['nodes','selectors','rules','regions'],
  setModuleParam:['modules','params'],setTemplate:['modules','params'],setVoicePolicy:['voicePolicy'],
  addRoute:['routes'],updateRoute:['routes'],removeRoute:['routes'],setModulator:['modulators'],addModulator:['modulators'],removeModulator:['modulators','routes'],
  setMacro:['macros'],renameMacro:['macros'],bindMacro:['macros'],learnMidi:['midiLearn'],receiveMidi:['macros','params','midiLearn'],
  setOutput:['nodes'],setSend:['nodes'],setBus:['buses'],setChain:['chains'],addProcessor:['chains'],moveProcessor:['chains'],bypass:['modules','chains'],setProcessorParam:['chains'],removeProcessor:['chains'],
  importAssets:['assets'],relink:['assets'],removeAsset:['assets','regions','rules','selectors','history'],replaceRegionAsset:['regions','history'],
  importSamples:['assets','nodes','regions','rules','selectors','history','params','modules','modulators','routes','chains'],mapAssets:['assets','nodes','regions','rules','selectors','params','modules','modulators','routes','chains'],
};
export function operationCommand(label,method,args=[]){
  let forward,inverse;
  const key=change=>`${change.kind}|${change.collection??change.key}|${change.id??''}|${JSON.stringify(change.path??[])}`;
  const combine=(a,b)=>Array.from(new Map([...a,...b].map(change=>[key(change),change])).values());
  const allowed=scopes[method];if(!allowed)throw new Error(`Unspecified command scope: ${method}`);
  const keep=change=>allowed.includes(change.collection??change.key)&&(!['setMacro','renameMacro','bindMacro'].includes(method)||change.id===args[0]);
  const command={id:method,label,
    mergeKey:['setRegion','setHistory','moveSliceBoundary','editSlices','updateRule','updateRoute','setModulator','setMacro','setCandidate','setVoicePolicy','setSend'].includes(method)?`${method}:${args[0]??'policy'}:${label}`:undefined,
    operands:()=>({forward,inverse}),
    merge(next){const operands=next.operands();forward=combine(forward,operands.forward);inverse=combine(operands.inverse,inverse);return command;},
    async do(engine){
      if(forward)return engine.applyDelta(forward);
      const before=engine.getPatch(),result=await engine[method](...args),after=engine.getPatch();
      forward=entityDelta(before,after).filter(keep);inverse=entityDelta(after,before).filter(keep);return result;
    },
    undo:engine=>engine.applyDelta(inverse),
  };
  return command;
}
