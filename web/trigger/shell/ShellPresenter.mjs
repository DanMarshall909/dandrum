import {projectReference} from './project-reference.mjs';
import {inputActions} from './input-actions.mjs';
import {modulationActions} from './modulation-actions.mjs';
import {mappingGeometry} from './mapping-geometry.mjs';
import {mappingClipboard} from './mapping-clipboard.mjs';
import {mappingModel} from './mapping-model.mjs';
import {sampleModel} from './sample-model.mjs';
import {importActions} from './import-actions.mjs';
import {libraryModel} from './library-model.mjs';
import {historyModel} from './history-model.mjs';
import {analysisActions,analysisModel} from './analysis-model.mjs';
import {slicesModel} from './slices-model.mjs';
import {layersModel} from './layers-model.mjs';
import {voiceModel} from './voice-model.mjs';
import {routingModel} from './routing-model.mjs';
import {modulationModel,destinationFor} from './modulation-model.mjs';
import {controlActions,controlMenus} from './control-menus.mjs';
import {macroModel} from './macro-model.mjs';
import {entityMenus} from './entity-menus.mjs';

/** Ephemeral selection, menus and pointer state; the Store owns patch commands. */
export class ShellPresenter {
  constructor(store) {
    this.props={store};
    this.sampleRevision=0;this.regionDefaults=new Map();this.processorDefaults=new Map();
  }
  get state() {return this.props.store.getSnapshot().editor;}
  subscribe=(listener)=>this.props.store.subscribe(listener);
  getSnapshot=()=>this.state;
  setState(update) {
    this.props.store.select(typeof update==='function'?update(this.state):update);
  }
  forceUpdate() {this.setState({});}
  command(label,method,...args){return this.props.store.operation(label,method,...args).catch(error=>this.setState({modNote:error.message}));}
  openNode(id){this.buildModel().tree.find(row=>row.id===id)?.go();}
  auditionNode(id){
    const patch=this.props.store.getSnapshot().patch,rule=patch.rules.find(rule=>rule.target===id||rule.group===id)??patch.rules[0];
    if(rule){this.props.store.noteOn(rule.root,100);this.props.store.noteOff(rule.root);}
  }
  rename(kind,id,value){this.setState({cmenu:null,dialog:{kind:'rename',entity:kind,id,value}});}
  async confirmRename(name){
    const dialog=this.state.dialog;
    await this.command('Rename '+dialog.value,dialog.entity==='node'?'renameNode':dialog.entity==='macro'?'renameMacro':'setModulator',dialog.id,dialog.entity==='modulator'?{name}:name);
    this.setState({dialog:null});
  }
  changeParameter(id, value) {
    try{return Promise.resolve(this.props.store.changeParam(id,value)).catch(error=>this.setState({modNote:error.message}));}catch(error){this.setState({modNote:error.message});}
  }
  undoStep(dir) {try{return Promise.resolve(dir<0?this.props.store.undo():this.props.store.redo()).catch(error=>this.setState({modNote:error.message}));}catch(error){this.setState({modNote:error.message});}}
  go(st) {  this.setState({ st, cmenu: null, dmenu: null, rrMode: null, kmenu: null, menu: null, editRoute: null, modInsp: false, histSel: null, modNote: null,selectedMacro:null }); }
  rng(seed) { let s = seed >>> 0; return () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; }; }
  peaks(n, f, seed) { const r = this.rng(seed), o = []; for (let i = 0; i < n; i++) o.push(Math.min(1, f(i / n, r))); return o; }
  nn(n) { const N = ['C','C♯','D','D♯','E','F','F♯','G','G♯','A','A♯','B']; return N[n % 12] + (Math.floor(n / 12) - 1); }
  auditionSelection(){
    const patch=this.props.store.getSnapshot().patch,model=this.buildModel();
    const zone=patch.rules.find(r=>r.id===this.state.zoneSel);
    let source=model.page==='slices'?model.sliceRegionId:model.page==='layers'?model.selectedCandidate:zone?.source;
    if(model.page==='layers'&&source){this.props.store.auditionSource(source,patch.rules.find(r=>r.source===source)?.root??38,100);return;}
    const seen=new Set();while(patch.selectors[source]&&!seen.has(source)){seen.add(source);source=patch.selectors[source].candidates.find(c=>!c.muted)?.id;}
    const region=patch.regions.find(r=>r.id===source)??patch.regions.find(r=>r.id===this.state.zoneSel)??patch.regions.find(r=>r.id==='k60f')??patch.regions[0];
    if(region){try{this.props.store.audition(region.assetId,region);}catch(error){this.setState({modNote:error.message});}}
  }
  requestReplace(kind,id){
    this.setState({patchMenu:false});
    if(this.props.store.dirty){this.setState({dialog:{kind:'confirm-replace',operation:kind,presetId:id}});return;}
    return this.performReplace(kind,id);
  }
  performReplace(kind,id){return this.props.store.replacePatch(kind,id).catch(error=>this.setState({modNote:error.message}));}
  confirmReplace(){const dialog=this.state.dialog;this.setState({dialog:null});return this.performReplace(dialog.operation,dialog.presetId);}
  buildModel() {
    const model=projectReference(this);
    sampleModel(this,model);
    this.importModel(model);
    libraryModel(this,model);
    mappingModel(this,model);
    analysisModel(this,model);
    slicesModel(this,model);
    historyModel(this,model);
    layersModel(this,model);
    voiceModel(this,model);
    routingModel(this,model);
    modulationModel(this,model);
    const current=this.props.store.getSnapshot().patch;
    model.routeSources=current.modulators.map(m=>({id:m.id,label:m.name}));
    model.routeDestinations=[...new Set(Object.keys(current.params).map(destinationFor).concat(['Send · Reverb','Send · Delay']))].map(id=>({id,label:id}));
    controlMenus(this,model);
    entityMenus(this,model);
    macroModel(this,model);
    model.subscribeTelemetry=this.props.store.subscribeTelemetry;
    model.getTelemetry=this.props.store.getTelemetry;
    model.dragging=this.state.dragSrc??this.state.dragMod;
    model.toggleLog=()=>this.setState({logOpen:!this.state.logOpen});
    model.scale=1; model.outW=model.W; model.outH=model.H;
    const snapshot=this.props.store.getSnapshot();
    model.patchName=snapshot.patch.name;
    if(snapshot.phase==='preparing')model.status={kind:'busy',title:'Loading',text:'Preparing patch'};
    if(snapshot.phase==='failed')model.status={kind:'error',title:'Operation failed',text:snapshot.error};
    model.openPatchMenu=event=>{const rect=event?.currentTarget?.getBoundingClientRect();this.setState({patchMenu:!this.state.patchMenu,patchMenuPosition:rect?this.winPt({clientX:rect.left,clientY:rect.bottom+2},220,150):{x:254,y:44}});};
    model.assetNames=Object.fromEntries(snapshot.patch.assets.map(a=>[a.id,a.name]));
    model.loadFeltKit=()=>this.requestReplace('load','felt-kit');
    model.newEmpty=()=>this.requestReplace('new');
    model.reload=()=>this.requestReplace('reload');
    model.macros=snapshot.patch.macros.map(m=>({...m,v:m.value,d:m.bindings.length+' destinations',off:!m.bindings.length,
      set:value=>this.props.store.operation('Change '+m.name,'setMacro',m.id,value).catch(error=>this.setState({modNote:error.message})),
      bg:this.state.selectedMacro===m.id?'var(--dd-vermilion-wash)':'transparent',edge:'none'}));
    return model;
  }
}
Object.assign(ShellPresenter.prototype,inputActions, modulationActions, mappingGeometry, mappingClipboard,importActions,analysisActions,controlActions);
