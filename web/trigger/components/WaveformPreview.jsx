import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';
export function WaveformPreview({assetId,getTelemetry,subscribeTelemetry,...props}){
  const telemetry=useSyncExternalStore(subscribeTelemetry,getTelemetry);
  const cursor=telemetry.playheads.find(p=>p.asset===assetId)?.pos??props.cursor;
  return <DD.WaveformPanel {...props} cursor={cursor}/>;
}
