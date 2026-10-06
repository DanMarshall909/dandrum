import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';

export function AppHeader({model}) {
  const telemetry=useSyncExternalStore(model.subscribeTelemetry,model.getTelemetry);
  return (<header style={{"height": (model.headH) + "px","flex": "none","display": "flex","alignItems": "center","gap": "12px","padding": "0 12px","background": "var(--dd-ink-0)","borderBottom": "1px solid var(--dd-line-1)"}}>
    <div style={{"display": "flex","alignItems": "center","gap": "8px","whiteSpace": "nowrap"}}>
      <span style={{"font": "700 20px var(--font-display)","letterSpacing": "-.01em","color": "var(--dd-paper-1)"}}>dandrum</span>
      <span style={{"width": "6px","height": "6px","borderRadius": "6px","background": "var(--dd-vermilion)"}}></span>
      {model.notMin && <><span style={{"font": "700 13px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Trigger</span></>}
    </div>
    <div style={{"width": "1px","height": "20px","background": "var(--dd-line-1)"}}></div>
    <div style={{"display": "flex","gap": "2px"}}>
      <DD.IconButton icon="reset" label={model.undoTip} size="sm" variant="ghost" disabled={model.noUndo} onClick={model.doUndo}></DD.IconButton>
      <DD.IconButton icon="reload" label={model.redoTip} size="sm" variant="ghost" disabled={model.noRedo} onClick={model.doRedo}></DD.IconButton>
    </div>
    <DD.MenuButton label="Patch" value={model.patchName} width={model.patchW} compact={model.isMin} onClick={model.openPatchMenu}></DD.MenuButton>
    <DD.StatusMessage inline={true} compact={model.isMin} kind={model.status.kind} title={model.status.title}>{model.status.text}</DD.StatusMessage>
    <div style={{"flex": "1"}}></div>
    <div style={{"display": "flex","alignItems": "center","gap": "6px"}}>
      {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Telemetry</span></>}
      <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-3)","whiteSpace": "nowrap"}}>{`Voices ${telemetry.voices.active} / ${telemetry.voices.max}`}</span>
    </div>
    {model.notPerf && <><DD.SegmentedControl compact={true} value={model.layout} onChange={model.setLayout} options={model.layoutOpts}></DD.SegmentedControl></>}
    <DD.Button size="sm" variant="secondary" icon={model.perfIcon} onClick={model.togglePerf}>{model.perfLabel}</DD.Button>
    <DD.Button size="sm" variant="ghost" onClick={model.toggleLog}>Log</DD.Button>
    <DD.Meter levels={telemetry.meters.main??[-90,-90]} thickness={4} length={30}></DD.Meter>
  </header>);
}
