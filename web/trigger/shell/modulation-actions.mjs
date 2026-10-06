export const modulationActions = {
resetTarget(kind, key) {
    const control=kind==='knob'&&this.controlKnobs?.find(knob=>knob.key===key);
    if(control){control.set(control.default??0);control.commit?.(control.default??0);this.setState({cmenu:null,kmenu:null});return;}
    if(kind==='knob'&&key.startsWith('Region ')){const knob=this.buildModel().sampleKnobGroups.flatMap(g=>g.knobs).find(k=>k.key===key);knob?.set(knob.default);knob?.commit?.(knob.default);this.setState({cmenu:null,kmenu:null});}
    else if (kind === 'knob' && this.props.store.getSnapshot().patch.params[key]) { this.props.store.changeParam(key,this.props.store.getSnapshot().patch.params[key].default);this.setState({cmenu:null,kmenu:null}); }
    else if(kind==='macro'){const macro=this.props.store.getSnapshot().patch.macros.find(m=>m.id===key||m.name===key);if(macro)this.props.store.operation('Reset '+macro.name,'setMacro',macro.id,macro.default).catch(error=>this.setState({modNote:error.message}));this.setState({cmenu:null,kmenu:null});}
    else if(kind==='knob')this.setState({cmenu:null,kmenu:null,modNote:'This control has no editable default'});
  },
modCats() { return [
    ['env', 'Envelopes', [['Amp envelope', null, 1, 62, 'Keys · Drums · Break'], ['Filter envelope', 'A', 1, 48, 'Keys']]],
    ['lfo', 'LFOs', [['LFO 1', 'B', 1, 70, 'Keys'], ['LFO 2', null, 1, 0, 'Keys · Drums']]],
    ['perf', 'Performance', [['Velocity', 'C', 2, 58, 'All groups'], ['Key track', null, 1, 50, 'Keys'], ['Aftertouch', null, 1, 0, 'Keys'], ['Mod wheel', null, 1, 22, 'Keys · Break'], ['Pitch bend', null, 1, 50, 'All groups']]],
    ['macro', 'Macros', [['Macro · Tone', 'D', 2, 56, 'Keys · Drums'], ['Macro · Space', null, 1, 32, 'All groups']]],
    ['rand', 'Random', [['Random per note', null, 1, 34, 'Drums']]],
  ]; },
winPt(e, w, hgt) {
    const el = this.winRef && this.winRef.current; if (!el) return { x: 0, y: 0 };
    const wr = el.getBoundingClientRect(), sc = wr.width / el.offsetWidth;
    let x = (e.clientX - wr.left) / sc + 2, y = (e.clientY - wr.top) / sc + 2;
    if (x + (w || 264) > el.offsetWidth) x = Math.max(4, x - (w || 264) - 4);
    if (y + (hgt || 200) > el.offsetHeight) y = Math.max(4, el.offsetHeight - (hgt || 200) - 4);
    return { x, y };
  },
play(name,o) {
    const patch=this.props.store.getSnapshot().patch;
    const region=patch.regions.find(r=>r.name===name||r.id===name);
    const asset=patch.assets.find(a=>a.name===name||a.id===name||a.id===region?.assetId);
    try {
      if(asset)this.props.store.audition(asset.id,{...region,start:o?.from??region?.start??0,end:o?.to??region?.end??1});
      else if(name==='Lush'){this.props.store.auditionSource('module:'+patch.modules.find(m=>m.type==='Lush')?.id);}
      else throw new Error(name+' is unavailable');
      this.setState({pv:{name,target:o?.target??null,from:o?.from??0,to:o?.to??1},pvPos:0});
    }catch(error){this.setState({modNote:error.message});}
  },
dropInfo(id, name) {
    const patch=this.props.store.getSnapshot().patch,node=patch.nodes.find(n=>n.id===id),selector=patch.selectors[id];
    if(selector)return {target:node?.name??id,role:'alternate'};
    if(node?.kind==='group')return {target:node.name,role:'new pad'};
    if(node?.kind==='sound')return {target:node.name,role:'layer'};
    return null;
  }
};
