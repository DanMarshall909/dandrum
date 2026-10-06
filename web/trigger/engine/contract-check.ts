import {MockEngine} from './mock-engine.mjs';
import {Store} from './store.mjs';
import type {EngineAdapter,EventMap,Region,MockDiagnostics} from './contract';

// Compile the concrete public surface against the handoff boundary.
const adapter:EngineAdapter=new MockEngine({preparationMs:0});
const diagnostics:MockDiagnostics=new MockEngine({preparationMs:0});
const store=new Store(adapter,{diagnostics});
void store.operation('Rename','renameNode','root','Instrument');
void store.undo();void store.redo();store.subscribeTelemetry(()=>{});
const patch=adapter.getPatch();
const region:Region|undefined=patch.regions[0];
if(region)void adapter.setRegion(region.id,{start:.1});
adapter.on('telemetry',(event:EventMap['telemetry'])=>event.voices.active);
void adapter.addNode('root','group');
void adapter.setModuleParam('filter','type','lp');
void adapter.setSend('keys','rev',.35);
void adapter.bindMacro('Tone','Cutoff',[0,1]);
