import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';
export function WaveformPreview({assetId,getTelemetry,subscribeTelemetry,available=true,...props}){
  const telemetry=useSyncExternalStore(subscribeTelemetry,getTelemetry);
  const cursor=telemetry.playheads.find(p=>p.asset===assetId)?.pos??props.cursor;
  if(!available)return <div role="img" aria-label="Waveform unavailable" style={{height:props.height,display:'flex',alignItems:'center',justifyContent:'center'}}>Waveform unavailable</div>;
  return <DD.WaveformPanel {...props} cursor={cursor}/>;
}
