import * as DD from '../components/design-system/index.jsx';
const modules=import.meta.glob('../components/**/*.jsx',{eager:true});
const groups={chrome:['AppHeader','MacroStrip','MacroControl','WorkspaceTabs','StatusBar','TriggerFrame'],library:['InstrumentTree','SourceList','ModulatorList','AssetSidebar','AssetBrowser'],inspector:['Inspector','Breadcrumb','TypeTag','HistoryStack','PropertyEditor'],sample:['SamplePage','WaveformPreview','WaveformEditor','AnalysisStatus'],slices:['SlicesPage','SliceTable','SliceWaveform','AnalysisLane'],mapping:['MappingPage','ZoneLane'],layers:['LayersPage','SelectorModeBar','VelocityCrossfade','CandidateTable','CycleSequence'],voice:['VoicePage','VoiceChain','VoiceSection','VoicePolicyPanel','EnvelopeEditor'],modulation:['ModulationPage','ModulationTable','ModulatorEditor','ShapePreview','AmountBar'],routing:['RoutingPage','RoutingTable','RoutingGraph','OutputBusPanel','EffectsPage','ProcessorStack'],menus:['GenericMenu','ControlMenu','DestinationMenu','ChoiceMenu'],infrastructure:['DragLayer','KeyboardPlayer','LogDrawer','Workspace','ParameterKnob','EmptyPage','DropPage']};
export const stories=Object.entries(modules).filter(([path])=>!path.includes('/design-system/')).flatMap(([path,exports])=>Object.entries(exports).filter(([,component])=>typeof component==='function').map(([name,component])=>({id:name,name,component,path,group:Object.entries(groups).find(([,names])=>names.includes(name))?.[0]??'dialogs'})))
  .concat(Object.entries(DD).filter(([name,component])=>typeof component==='function'&&name[0]===name[0].toUpperCase()).map(([name,component])=>({id:'DD.'+name,name:'DD.'+name,component,path:'components/design-system/index.jsx',group:'Dandrum primitives'})));
export const states=['default','hover','selected','disabled','empty','loading','error','long text','minimum width'];
export const engineCalls={MacroStrip:'setMacro, bindMacro, learnMidi',MacroControl:'setMacro, bindMacro, learnMidi',InstrumentTree:'renameNode, duplicateNode, removeNode, assignSource',SourceList:'audition, auditionSource, assignSource',ModulatorList:'addModulator, setModulator, addRoute',AssetBrowser:'audition, replaceRegionAsset',HistoryStack:'setHistory, reorderHistory, collapseHistory',WaveformEditor:'setRegion, audition',SliceTable:'editSlices, mapSlices, splitSlice, mergeSlices, removeSlice',ZoneLane:'setRules, updateRule, noteOn, noteOff',CandidateTable:'setCandidate, reorderCandidates',VoiceSection:'setParam, setModuleParam',EnvelopeEditor:'setModulator',ModulationTable:'addRoute, updateRoute, removeRoute',ModulatorEditor:'setModulator, updateRoute',RoutingTable:'setOutput, setSend',ProcessorStack:'addProcessor, moveProcessor, bypass, setProcessorParam',KeyboardPlayer:'noteOn, noteOff, audition',LogDrawer:'setMockSwitch'};
Object.assign(engineCalls,{
  AppHeader:'newPatch, loadPreset, reload; command undo/redo',WorkspaceTabs:'None; editor navigation',
  SamplePage:'setRegion, replaceRegionAsset, analyse, audition, setParam',SlicesPage:'setSlices, splitSlice, mergeSlices, removeSlices, editSlices, mapSlices, analyse, cancel',
  MappingPage:'setRules, updateRule, mapAssets, assignSource',LayersPage:'auditionSource, setSelector, setCandidate, reorderCandidates, addCandidate, removeCandidate',
  VoicePage:'setTemplate, setModuleParam, setParam, setVoicePolicy, setModulator',ModulationPage:'setModulator, addRoute, updateRoute, removeRoute',
  RoutingPage:'setOutput, setSend, setBus',EffectsPage:'setChain, addProcessor, moveProcessor, setProcessorParam, bypass',
  Inspector:'Parent-bound row callbacks; setRegion, setHistory, setModulator, setVoicePolicy, updateRule, setSelector, setOutput, setSend, setProcessorParam, bindMacro',
  PropertyEditor:'Parent-bound field callback; the selected entity determines the operation',
  AssetSidebar:'addModulator, audition, assignSource',SelectorModeBar:'setSelector',VelocityCrossfade:'setSelector',VoiceChain:'setTemplate',VoicePolicyPanel:'setVoicePolicy',
  SliceWaveform:'moveSliceBoundary, splitSlice, audition',WaveformPreview:'audition',AnalysisStatus:'analyse, cancel',AnalysisLane:'None; read-only analysis',
  AmountBar:'updateRoute',OutputBusPanel:'setBus',RoutingGraph:'None; read-only routing',ShapePreview:'None; read-only projection',
  PresetBrowser:'loadPreset',VoiceTemplateDialog:'setTemplate',RouteDialog:'addRoute',RenameDialog:'renameNode, renameMacro, setModulator',
  ConfirmDialog:'newPatch, loadPreset, reload',ValueDialog:'Parent-bound parameter operation',ChoiceDialog:'addProcessor, addModulator',
  ParameterKnob:'Parent-bound setParam, setRegion, setModulator or setProcessorParam',NumericField:'Parent-bound entity operation',Toggle:'Parent-bound entity operation',
  ControlMenu:'addRoute, updateRoute, removeRoute, bindMacro, learnMidi, resetParam',DestinationMenu:'addRoute, bindMacro',GenericMenu:'Selected item callback; see owning control',ChoiceMenu:'Selected value callback; see owning field',
  EmptyPage:'importSamples',DropPage:'importSamples',StatusBar:'None; telemetry subscription',DragLayer:'None; pointer presentation',
});
for(const story of stories){
  if(story.name.startsWith('DD.'))engineCalls[story.name]=['DD.Knob','DD.Slider','DD.NumericField','DD.Toggle','DD.LayerStack','DD.KeyMap','DD.OutputBusses','DD.MenuButton','DD.PadCell'].includes(story.name)?'Parent-bound callbacks; setParam, setRegion, setRules, setProcessorParam, setBus or noteOn/noteOff according to binding':'None directly; supplied presentation primitive';
  engineCalls[story.name]??='None directly; presentation or parent callback composition';
}
