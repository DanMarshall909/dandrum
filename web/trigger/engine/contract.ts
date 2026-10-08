export type Id = string;
export type Json = null | boolean | number | string | Json[] | {[key:string]:Json};
export type Config = {[key:string]:Json};
export type ParamValue = number | boolean | string;
export interface FileRef {name:string;uri?:string;duration?:number;type?:string}
export interface ImportOptions {policy?:'sequential'|'root-velocity'|'stack'|'rr';startNote?:number;group?:Id;groupName?:string;replace?:Id;target?:Id}
export interface Node {id:Id;name:string;kind:'instrument'|'group'|'sound';parent:Id|null;output?:Id|'inherit';sends?:Record<Id,number>;note?:number;root?:number;choke?:number}
export interface Asset {id:Id;name:string;uri:string;state:'loading'|'loaded'|'missing'|'unsupported';progress?:number;duration:number;sampleRate:number;bits:number;channels:number;cached:boolean;group?:Id;analysis?:{note?:number;cents?:number;peak?:number;lufs?:number}}
export interface Region {id:Id;assetId:Id;name:string;kind:'sample'|'slice';start:number;end:number;root?:number;note?:number;fades?:{in:number;out:number};loop?:{start:number;end:number;crossfade:number}|null;playback?:string;reverse?:boolean;tune?:number;gain?:number;pan?:number;choke?:number}
export interface Rule {id?:Id;group:Id;target?:Id;name:string;source:Id|null;noteLo:number;noteHi:number;velLo:number;velHi:number;root:number}
export interface Candidate {id:Id;name:string;weight:number;muted:boolean;solo:boolean;plays?:number;gain?:number;pan?:number;tune?:number;output?:Id;velLo?:number;velHi?:number}
export interface Selector {policy:'stack'|'velocity'|'rr'|'alt'|'random'|'weighted';reset?:string;softHi?:number;hardLo?:number;curve?:string;candidates:Candidate[]}
export interface Module {id:Id;type:string;params:Record<string,ParamValue>;paramIds?:Record<string,Id>;bypassed?:boolean}
export interface Parameter {id:Id;name:string;value:number;default:number;min:number;max:number;unit:string}
export interface Modulator {id:Id;name:string;kind:string;slot:string|null;shape:string;config:Config;locked?:boolean}
export interface Route {id?:Id;source:Id;destination:Id;amount:number;polarity:string;curve:string;on:boolean;locked?:boolean}
export interface Macro {id:Id;name:string;value:number;default:number;host:boolean;midi:{cc:number;channel:number}|null;bindings:{destination:Id;range:[number,number]}[]}
export interface Bus {id:Id;name:string;channels:number[];level?:number;muted?:boolean}
export interface Chain {id:Id;name:string;output:Id;modules:Module[];level?:number;muted?:boolean}
export interface HistoryOperation {id:Id;name:string;on:boolean;locked?:boolean;config:Config;regionId?:Id}
export interface VoicePolicy {mode:'poly'|'mono'|'legato';limit:number;steal:string;sameNote:string;glide:number;exclusiveGroup?:number}
export interface Patch {id:Id;name:string;nodes:Node[];assets:Asset[];regions:Region[];rules:Rule[];selectors:Record<Id,Selector>;modules:Module[];params:Record<Id,Parameter>;modulators:Modulator[];routes:Route[];macros:Macro[];buses:Bus[];chains:Chain[];history:HistoryOperation[];midiLearn:{target:Id;state:'waiting'}|null;voicePolicy:VoicePolicy}
export type AnalysisKind = 'transients'|'pitch'|'loops'|'loudness';
export interface Job {id:Id;assetId:Id;kind:AnalysisKind;state:'running'|'done'|'failed'|'cancelled';progress:number;result?:Json;message?:string}
export interface Telemetry {voices:{active:number;max:number};notes:{note:number;vel:number;layer:Id}[];playheads:{asset:Id;pos:number}[];selectorPos:Record<Id,{last:Id;next:Id;index?:number}>;mod:Record<Id,number>;meters:Record<Id,[number,number]>;host:Record<Id,boolean>;hostValues?:Record<Id,number>}
export interface EventMap {patch:{phase:'preparing'|'ready'|'failed';patch?:Patch;message?:string};asset:Asset;job:Job;param:{id:Id;value:number;gesture?:'begin'|'end'};telemetry:Telemetry;host:{id:Id;value:number;host:boolean}}
export type EntityCollection = 'nodes'|'assets'|'regions'|'rules'|'modules'|'modulators'|'routes'|'macros'|'buses'|'chains'|'history';
export type DictionaryCollection = 'params'|'selectors';
export type OperandPath = (string|{id:Id})[];
/** Stable-identity operands, validated and published atomically by the adapter. */
export type UndoOperand =
  | {kind:'entity';collection:EntityCollection;id:Id;value:Json;index:number}
  | {kind:'order';collection:EntityCollection;ids:Id[]}
  | {kind:'dictionary';collection:DictionaryCollection;id:Id;value:Json}
  | {kind:'property';collection:EntityCollection|DictionaryCollection;id:Id;path:OperandPath;value:Json;present:boolean;order?:boolean}
  | {kind:'field';key:keyof Patch;value:Json};
export interface LogEntry {time:number;kind:'call'|'event'|'result'|'error';name:string;args?:unknown;result?:unknown}
/** Optional standalone diagnostics; this is not a native EngineAdapter capability. */
export interface MockDiagnostics {getLog():LogEntry[];clearLog():void;setMockSwitch(name:string,on:boolean):void}

/** Prepared edits publish only after validation succeeds; live writes are synchronous.
 * Telemetry subscriptions own their periodic stream; clients never start an internal simulator. */
export interface EngineAdapter {
  applyDelta(operands:UndoOperand[]):Promise<void>;
  getPatch():Patch;newPatch():Promise<void>;loadPreset(id:string):Promise<void>;reload():Promise<void>;
  addNode(parent:Id,kind:'group'|'sound'):Promise<Id>;removeNode(id:Id):Promise<void>;renameNode(id:Id,name:string):Promise<void>;
  duplicateNode(id:Id):Promise<Id>;moveNode(id:Id,parent:Id,index:number):Promise<void>;
  importAssets(files:FileRef[]):Promise<Asset[]>;relink(id:Id,uri:string):Promise<void>;getPeaks(id:Id,bins:number):Float32Array;
  mapAssets(assets:Id[],options?:ImportOptions):Promise<Id|null>;importSamples(files:FileRef[],options?:ImportOptions):Promise<{assets:Asset[];rule:Id|null}>;
  setRegion(id:Id,delta:Partial<Region>):Promise<void>;splitSlice(id:Id,at:number):Promise<Id>;mergeSlices(a:Id,b:Id):Promise<void>;removeSlice(id:Id):Promise<void>;
  setSlices(asset:Id,markers:number[],options?:{group?:Id;startNote?:number;method?:string}):Promise<Id[]>;moveSliceBoundary(id:Id,start:number):Promise<void>;mapSlices(asset:Id,from:number):Promise<void>;editSlices(ids:Id[],delta:Partial<Region>):Promise<void>;removeSlices(ids:Id[]):Promise<void>;
  analyse(asset:Id,kind:AnalysisKind):Job;cancel(job:Id):void;
  addRule(rule:Rule):Promise<Id>;updateRule(id:Id,delta:Partial<Rule>):Promise<void>;removeRule(id:Id):Promise<void>;setRules(rules:Rule[]):Promise<void>;
  setSelector(target:Id,delta:Partial<Selector>):Promise<void>;reorderCandidates(target:Id,order:Id[]):Promise<void>;setCandidate(target:Id,id:Id,delta:Partial<Candidate>):Promise<void>;
  addCandidate(target:Id,source:Id):Promise<void>;removeCandidate(target:Id,id:Id):Promise<void>;assignSource(target:Id,source:Id,policy?:string):Promise<Id|void>;
  setModuleParam(module:Id,param:string,value:ParamValue):Promise<void>;setVoicePolicy(delta:Partial<VoicePolicy>):Promise<void>;setTemplate(modules:Module[]):Promise<void>;
  setParam(id:Id,value:number,gesture?:'begin'|'end'):void;resetParam(id:Id):void;
  addRoute(route:Route):Promise<Id>;updateRoute(id:Id,delta:Partial<Route>):Promise<void>;removeRoute(id:Id):Promise<void>;
  setModulator(id:Id,delta:Partial<Modulator>):Promise<void>;addModulator(kind:string):Promise<Id>;removeModulator(id:Id):Promise<void>;
  setMacro(id:Id,value:number):void;renameMacro(id:Id,name:string):Promise<void>;bindMacro(id:Id,destination:Id,range:[number,number]):Promise<void>;learnMidi(target:Id|null):Promise<void>;receiveMidi(cc:number,channel:number):Promise<void>;
  setOutput(node:Id,bus:Id|'inherit'):Promise<void>;setSend(node:Id,fx:Id,gain:number):Promise<void>;setBus(id:Id,delta:Partial<Bus>):Promise<void>;
  setChain(id:Id,delta:Partial<Chain>):Promise<void>;
  addProcessor(chain:Id,type:string,index?:number):Promise<Id>;moveProcessor(id:Id,chain:Id,index:number):Promise<void>;bypass(id:Id,on:boolean):Promise<void>;
  setProcessorParam(id:Id,param:string,value:number):Promise<void>;removeProcessor(id:Id):Promise<void>;
  setHistory(id:Id,delta:Partial<HistoryOperation>):Promise<void>;reorderHistory(order:Id[]):Promise<void>;addHistory(type:string,config?:Config,region?:Id):Promise<Id>;removeHistory(id:Id):Promise<void>;collapseHistory(id:Id):Promise<Id>;
  removeAsset(id:Id):Promise<void>;replaceRegionAsset(id:Id,asset:Id):Promise<void>;
  noteOn(note:number,velocity:number):void;noteOff(note:number):void;audition(asset:Id,region?:Region):void;
  auditionSource(source:Id,note?:number,velocity?:number):void;
  on<E extends keyof EventMap>(event:E,listener:(payload:EventMap[E])=>void):()=>void;
}
