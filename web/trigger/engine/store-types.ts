import type {EngineAdapter,Patch,EventMap,Job} from './contract';
export interface EditorCommand {
  id:string;label:string;mergeKey?:string;
  do(engine:EngineAdapter):unknown;undo(engine:EngineAdapter):unknown;
  merge?(next:EditorCommand):EditorCommand;
}
export interface CommandEntry {command:EditorCommand;time:number;gesture:number|null}
export interface StoreSnapshot {patch:Patch;editor:Record<string,any>;busy:boolean;phase:EventMap['patch']['phase'];error:string|null;jobs:Record<string,Job>}
