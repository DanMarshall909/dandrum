import React,{useEffect,useRef,useState,useSyncExternalStore} from 'react';
import {createPortal} from 'react-dom';
import {PresetBrowser} from './components/PresetBrowser.jsx';
import {KeyboardPlayer} from './components/KeyboardPlayer.jsx';
import {LogDrawer} from './components/LogDrawer.jsx';
import {ConfirmDialog} from './components/ConfirmDialog.jsx';
import {RenameDialog} from './components/RenameDialog.jsx';
import {ValueDialog} from './components/ValueDialog.jsx';
import {VoiceTemplateDialog} from './components/VoiceTemplateDialog.jsx';
import {ChoiceDialog} from './components/ChoiceDialog.jsx';
import {RouteDialog} from './components/RouteDialog.jsx';
import {MidiLearnDialog} from './components/MidiLearnDialog.jsx';
import {TriggerFrame} from './components/TriggerFrame.jsx';
import {ShellPresenter} from './shell/ShellPresenter.mjs';

export function TriggerController({store}) {
  const [presenter]=useState(()=>new ShellPresenter(store));
  const fileInput=useRef(null),folderInput=useRef(null);
  const {editor:state}=useSyncExternalStore(store.subscribe,store.getSnapshot);
  useEffect(()=>{presenter.attach();return()=>presenter.detach();},[presenter]);
  const model=presenter.buildModel();
  const waveKey=model.assetId+'|'+model.sliceAssetId+'|'+JSON.stringify(store.getSnapshot().patch.history);
  useEffect(()=>{const peaks={};for(const id of new Set([model.assetId,model.sliceAssetId].filter(Boolean))){try{peaks[id]=Array.from(store.engine.getPeaks(id,600));}catch(error){presenter.setState({modNote:error.message});}}
    presenter.setState({wavePeaks:peaks});},[store,presenter,waveKey]);
  model.browseSamples=()=>{presenter.setState({replaceTarget:null,relinkTarget:null});fileInput.current?.click();};
  model.replaceSample=()=>{presenter.setState({replaceTarget:model.regionId,relinkTarget:null});fileInput.current?.click();};
  presenter.replaceSample=model.replaceSample;
  presenter.replaceRegion=id=>{presenter.setState({replaceTarget:id,relinkTarget:null});fileInput.current?.click();};
  presenter.replaceSource=id=>{presenter.setState({relinkTarget:id,replaceTarget:null});fileInput.current?.click();};
  presenter.searchFolder=id=>{presenter.setState({searchFolderTarget:id});folderInput.current?.click();};
  presenter.browseSamples=model.browseSamples;
  model.logDrawer=state.logOpen?<LogDrawer getEvents={store.getLog} onClear={()=>store.clearLog()} switches={state.mockSwitches} onSwitch={(name,value)=>store.setMockSwitch(name,value)}/>:null;
  return <>
    <input ref={folderInput} type="file" multiple hidden aria-label="Search sample folder" webkitdirectory="" onChange={e=>{const id=presenter.state.searchFolderTarget,asset=store.getSnapshot().patch.assets.find(a=>a.id===id),file=Array.from(e.target.files).find(f=>f.name.toLowerCase()===asset?.name.toLowerCase());if(file)presenter.command('Find source in folder','relink',id,'mock://'+(file.webkitRelativePath||file.name));else presenter.setState({modNote:'No matching source found in this folder.'});e.target.value='';}}/>
    <input ref={fileInput} type="file" multiple hidden aria-label="Import sample files" accept=".wav,.aif,.aiff,.flac,.mp3,.ogg,.m4a,.aac" onChange={e=>{model.onFiles(e.target.files);e.target.value='';}}/>
    <KeyboardPlayer bindings={presenter.keyboardBindings}/>
    <TriggerFrame model={model}/>
    {state.patchMenu&&presenter.winRef.current&&createPortal(<div className="patch-menu" style={{left:state.patchMenuPosition?.x??254,top:state.patchMenuPosition?.y??44}}>
      <button onClick={()=>presenter.setState({patchMenu:false,dialog:{kind:'preset-browser'}})}>Browse presets…</button>
      <button onClick={model.loadFeltKit}>Load Felt Kit</button>
      <button onClick={model.newEmpty}>New empty patch</button>
      <button onClick={model.reload}>Reload Patch</button>
    </div>,presenter.winRef.current)}
    {state.dialog?.kind==='preset-browser'&&<PresetBrowser presets={[{id:'felt-kit',name:'Felt Kit',detail:'Keys · Drums · Break'},{id:'empty',name:'Empty',detail:'Start a new instrument'}]} onLoad={id=>{presenter.setState({dialog:null});presenter.requestReplace('load',id);}} onCancel={()=>presenter.setState({dialog:null})}/>}
    {state.dialog?.kind==='confirm-replace'&&<ConfirmDialog onConfirm={()=>presenter.confirmReplace()} onCancel={()=>presenter.setState({dialog:null})}/>}
    {state.dialog?.kind==='rename'&&<RenameDialog title={'Rename '+state.dialog.value} value={state.dialog.value} onSubmit={name=>presenter.confirmRename(name)} onCancel={()=>presenter.setState({dialog:null})}/>}
    {state.dialog?.kind==='slice-note'&&<ValueDialog title="Map slice to key" label="MIDI note" value={state.dialog.value} onSubmit={note=>{presenter.command('Map slice','editSlices',[state.dialog.id],{note:Math.round(note),root:Math.round(note)});presenter.setState({dialog:null});}} onCancel={()=>presenter.setState({dialog:null})}/>}
    {state.dialog?.kind==='voice-template'&&<VoiceTemplateDialog model={model.template} onClose={()=>presenter.setState({dialog:null})}/>}
    {state.dialog?.kind==='add-processor'&&<ChoiceDialog title="Add processor" choices={model.processorChoices} onSelect={model.confirmProcessor} onCancel={()=>presenter.setState({dialog:null})}/>}
    {state.dialog?.kind==='add-route'&&<RouteDialog sources={model.routeSources} destinations={model.routeDestinations} onSubmit={route=>{presenter.command('Add modulation route','addRoute',route);presenter.setState({dialog:null});}} onCancel={()=>presenter.setState({dialog:null})}/>}
    {state.dialog?.kind==='midi-learn'&&<MidiLearnDialog target={state.dialog.target} onReceive={(cc,channel)=>{presenter.command('Receive MIDI learn','receiveMidi',cc,channel);presenter.setState({dialog:null});}} onCancel={()=>{presenter.command('Cancel MIDI learn','learnMidi',null);presenter.setState({dialog:null});}}/>}
    {state.dialog?.kind==='add-modulator'&&<ChoiceDialog title="Add modulator" choices={['envelope','lfo','performance','macro','random']} onSelect={kind=>{presenter.command('Add modulator','addModulator',kind);presenter.setState({dialog:null});}} onCancel={()=>presenter.setState({dialog:null})}/>}
  </>;
}
