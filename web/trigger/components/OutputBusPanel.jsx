import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';
export function OutputBusPanel({model}){
  const telemetry=useSyncExternalStore(model.subscribeTelemetry,model.getTelemetry);
  return <DD.OutputBusses style={{minWidth:0}} title="Output busses" busses={model.busses.map(b=>({...b,levels:telemetry.meters[b.id]??[-90,-90]}))} onChange={model.changeBusses}/>;
}
