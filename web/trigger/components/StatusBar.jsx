import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';

export function StatusBar({model}) {
  const telemetry=useSyncExternalStore(model.subscribeTelemetry,model.getTelemetry),asset=telemetry.playheads[0]?.asset;
  const hint=model.dragging?model.hint:asset?'Previewing '+(model.assetNames?.[asset]??asset):model.hint?.startsWith('Previewing ')?model.idleHint:model.hint;
  return (<footer style={{"height": "26px","flex": "none","display": "flex","alignItems": "center","gap": "12px","padding": "0 12px","background": "var(--dd-ink-0)","borderTop": "1px solid var(--dd-line-1)","font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)","whiteSpace": "nowrap","overflow": "hidden"}}>
    <span style={{"overflow": "hidden","textOverflow": "ellipsis"}}>{hint}</span>
    <div style={{"flex": "1"}}></div>
    <span style={{"font": "500 11px var(--font-value)"}}>{model.sizeText}</span>
  </footer>);
}
