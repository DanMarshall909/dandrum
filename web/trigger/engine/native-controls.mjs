/** Bind only controls actually published by the native instrument. */
export function nativeControls(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,rule=patch.rules.find(rule=>rule.id===presenter.state.zoneSel);
  const groups=new Map();
  const knob=parameter=>{
    const min=parameter.physicalMin??0,max=parameter.physicalMax??1,physical=min+parameter.value*(max-min);
    return {label:parameter.id.split('.').at(-1).replace(/_/g,' '),key:parameter.id,v:parameter.value,default:parameter.default,t:Number(physical.toFixed(4)).toString(),mods:[],bi:min<0,
      parse:text=>{const value=Number.parseFloat(text.replace('−','-'));return Number.isFinite(value)&&max>min?Math.max(0,Math.min(1,(value-min)/(max-min))):null;},
      set:value=>presenter.changeParameter(parameter.id,value)};
  };
  for(const parameter of Object.values(patch.params)){
    const label=parameter.id.split('.').slice(0,-1).join(' · ');
    if(!groups.has(label))groups.set(label,[]);groups.get(label).push(knob(parameter));
  }
  model.voiceSecs=Array.from(groups,([title,knobs])=>({id:title,title,sub:'Native sampler controls',ann:'Exposed native parameters',hasSeg:false,knobs}));
  // Region controls prepare real asset geometry; remaining controls use native host IDs.
  const nativeRuleGroup=rule?.nativeControlGroup;
  const selected=Object.values(patch.params).filter(parameter=>parameter.nativeScope!=='sampleGroup'||parameter.nativeControlGroup===nativeRuleGroup);
  model.sampleKnobGroups=[model.sampleKnobGroups[0],{title:'Native controls',ann:'Exposed native parameters',knobs:selected.map(knob)}];
}
