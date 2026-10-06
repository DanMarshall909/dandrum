// @ts-check
import {telemetryReader} from './telemetry-reader.mjs';
import {parameterCommand} from './commands.mjs';
import {operationCommand} from './operation-command.mjs';
export class Store {
  /** @param {import('./contract').EngineAdapter} engine
   * @param {{now?:()=>number,diagnostics?:import('./contract').MockDiagnostics|null}} options */
  constructor(engine,{now=()=>Date.now(),diagnostics=null}={}) {
    this.engine=engine;this.diagnostics=diagnostics;this.now=now;
    this.undoStack=/** @type {import('./store-types').CommandEntry[]} */([]);this.redoStack=/** @type {import('./store-types').CommandEntry[]} */([]);
    this.listeners=/** @type {Set<()=>void>} */(new Set());this.gesture=/** @type {{id:string,sequence:number,start:number}|null} */(null);this.gestureSequence=0;
    this.telemetryReader=telemetryReader(engine);
    this.pending=/** @type {Promise<unknown>|null} */(null);this.closed=false;
    this.phase=/** @type {import('./contract').EventMap['patch']['phase']} */('ready');this.error=/** @type {string|null} */(null);this.jobs=/** @type {Record<string,import('./contract').Job>} */({});
    this.editor=/** @type {Record<string,any>} */({page:'sample',st:'empty',ann:false,layout:'default',browser:false,
      selection:null,zoneSel:'k60f',zones:null,rrMode:null});
    this.snapshot=/** @type {import('./store-types').StoreSnapshot} */({patch:engine.getPatch(),editor:this.editor,busy:false,phase:this.phase,error:null,jobs:this.jobs});
    this.unsubscribers=[engine.on('patch',payload=>{this.phase=payload.phase;this.error=payload.message??null;this.notify();}),
      engine.on('param',()=>this.notify()),engine.on('asset',()=>this.notify()),
      engine.on('job',payload=>{this.jobs={...this.jobs,[payload.id]:payload};this.notify();})];
  }
  /** @param {()=>void} fn */
  subscribe=(fn)=>{this.listeners.add(fn);return()=>this.listeners.delete(fn);};
  getSnapshot=()=>this.snapshot;
  notify() {
    if(this.closed)return;
    this.snapshot={patch:this.engine.getPatch(),editor:this.editor,busy:!!this.pending,phase:this.phase,error:this.error,jobs:this.jobs};
    for(const listener of this.listeners)listener();
  }
  /** @param {Record<string,any>} delta */
  select(delta){this.editor={...this.editor,...delta};this.notify();}
  getTelemetry=()=>this.telemetryReader.getSnapshot();
  /** @param {()=>void} listener */
  subscribeTelemetry=(listener)=>this.telemetryReader.subscribe(listener);
  getLog=()=>this.diagnostics?.getLog()??[];
  clearLog(){this.diagnostics?.clearLog();this.notify();}
  /** @param {string} name @param {boolean} on */
  setMockSwitch(name,on){if(!this.diagnostics)throw new Error('Mock diagnostics are unavailable');this.diagnostics.setMockSwitch(name,on);this.select({mockSwitches:{...this.editor.mockSwitches,[name]:on}});}
  /** @param {number} note @param {number} velocity */
  noteOn(note,velocity){this.engine.noteOn(note,velocity);}
  /** @param {number} note */
  noteOff(note){this.engine.noteOff(note);}
  /** @param {string} id @param {import('./contract').Region} [region] */
  audition(id,region){this.engine.audition(id,region);}
  /** @param {string} id @param {number} [note] @param {number} [velocity] */
  auditionSource(id,note,velocity){this.engine.auditionSource(id,note,velocity);}
  /** @param {string} id @param {import('./contract').AnalysisKind} kind */
  analyse(id,kind){return this.engine.analyse(id,kind);}
  /** @param {string} id */
  cancelJob(id){this.engine.cancel(id);}
  /** @param {string} id @param {number} value */
  changeParam(id,value){return this.dispatch(parameterCommand(this.engine,id,value));}
  /** @param {string} label @param {string} method @param {...any} args */
  operation(label,method,...args){return this.dispatch(operationCommand(label,method,args));}
  get dirty(){return this.undoStack.length>0;}
  /** @param {'new'|'load'|'reload'} kind @param {string} [id] */
  replacePatch(kind,id){
    return this.run(async()=>{
      try {
        await (kind==='load'?this.engine.loadPreset(id??''):kind==='reload'?this.engine.reload():this.engine.newPatch());
        if(this.closed)throw new Error('Store is closed');
        const patch=this.engine.getPatch();this.undoStack=[];this.redoStack=[];this.gesture=null;this.jobs={};
        this.editor={...this.editor,st:patch.rules.length?'sample':'empty',selection:null,zoneSel:patch.rules.find(r=>r.id==='k60f')?.id??patch.rules[0]?.id,
          sampleDraft:null,selectorDraft:null,waveViewport:null,sliceSelection:null,selectedMacro:null,selectedModule:null,selectedRoute:null,selectorTarget:null,analysisJob:null,lastImportError:null,importDialog:null,dragSrc:null,dragMod:null,zones:null,menu:null,kmenu:null,cmenu:null,dmenu:null,patchMenu:false,dialog:null,histSel:null,modInsp:false,editRoute:null,modNote:null};
        this.error=null;this.phase='ready';this.notify();
      }catch(error){this.phase='failed';this.error=error instanceof Error?error.message:String(error);this.notify();throw error;}
    });
  }
  /** @param {string} id */
  beginGesture(id){this.gesture={id,sequence:++this.gestureSequence,start:this.undoStack.length};}
  cancelGesture(){const gesture=this.gesture;this.gesture=null;if(!gesture)return;return this.run(async()=>{while(true){const entry=this.undoStack.at(-1);if(!entry||entry.gesture!==gesture.sequence)break;await entry.command.undo(this.engine);this.undoStack.pop();}this.notify();});}
  /** @param {string} id */
  endGesture(id){if(this.gesture?.id===id)this.gesture=null;}
  /** @param {()=>any} action */
  run(action) {
    const execute=()=>{if(this.closed)throw new Error('Store is closed');return action();};
    const result=this.pending?this.pending.catch(()=>{}).then(execute):execute();
    if(!(result instanceof Promise))return result;
    const tracked=result.finally(()=>{if(this.pending===tracked)this.pending=null;this.notify();});
    this.pending=tracked;this.notify();return tracked;
  }
  /** @param {import('./store-types').EditorCommand} command */
  dispatch(command) {
    const time=this.now(),gesture=this.gesture?.sequence??null;
    return this.run(()=>{
      const result=command.do(this.engine);
      const record=()=>{
        if(this.closed)throw new Error('Store is closed');
        const last=this.undoStack.at(-1);
        const merge=last&&command.mergeKey&&last.command.mergeKey===command.mergeKey
          &&(gesture!==null?last.gesture===gesture:last.gesture===null&&time-last.time<=600);
        if(merge&&last.command.merge){last.command=last.command.merge(command);last.time=time;}
        else this.undoStack.push({command,time,gesture});
        if(this.undoStack.length>100)this.undoStack.shift();
        this.redoStack=[];this.notify();return result;
      };
      return result instanceof Promise?result.then(record):record();
    });
  }
  undo() {
    return this.run(()=>{
      const entry=this.undoStack.at(-1);if(!entry)return;
      const result=entry.command.undo(this.engine);
      const finish=()=>{this.undoStack.pop();this.redoStack.push(entry);this.notify();};
      return result instanceof Promise?result.then(finish):finish();
    });
  }
  redo() {
    return this.run(()=>{
      const entry=this.redoStack.at(-1);if(!entry)return;
      const result=entry.command.do(this.engine);
      const finish=()=>{
        this.redoStack.pop();this.undoStack.push({...entry,time:this.now(),gesture:null});this.notify();
      };
      return result instanceof Promise?result.then(finish):finish();
    });
  }
  get canUndo(){return !this.pending&&this.undoStack.length>0;}
  get canRedo(){return !this.pending&&this.redoStack.length>0;}
  get undoCount(){return this.undoStack.length;}
  get undoLabel(){return this.undoStack.at(-1)?.command.label||'';}
  get redoLabel(){return this.redoStack.at(-1)?.command.label||'';}
  close(){this.closed=true;this.telemetryReader.close();for(const unsubscribe of this.unsubscribers)unsubscribe();this.listeners.clear();}
}
