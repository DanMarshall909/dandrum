import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';
export function OutputBusPanel({model}){
  const telemetry=useSyncExternalStore(model.subscribeTelemetry,model.getTelemetry);
  return <>{model.meteringAvailable===false&&<p>Meters unavailable</p>}<DD.OutputBusses style={{minWidth:0}} title="Output busses" busses={model.busses.map(b=>({...b,levels:model.meteringAvailable===false?[]:telemetry.meters[b.id]??[-90,-90]}))} onChange={model.changeBusses}/></>;
}
