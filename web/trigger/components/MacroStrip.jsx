import {MacroControl} from './MacroControl.jsx';
import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';

export function MacroStrip({model}) {
  const telemetry=useSyncExternalStore(model.subscribeTelemetry,model.getTelemetry);
  return (<section style={{"position": "relative","flex": "none","display": "flex","alignItems": "center","justifyContent": "center","padding": model.macroPad,"background": "var(--dd-ink-2)","borderBottom": "1px solid var(--dd-line-1)"}}>
    <div style={{"position": "absolute","left": "12px","top": "50%","transform": "translateY(-50%)","display": "flex","flexDirection": "column","gap": "2px"}}>
      <span style={{"font": "700 11px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Macros</span>
      {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","alignSelf": "flex-start"}}>Binding</span></>}
    </div>
    <div style={{"display": "flex","justifyContent": "center","gap": "6px"}}>
      {(model.macros || []).map((m,index) => <React.Fragment key={m.id ?? m.key ?? index}>
        <MacroControl model={model} macro={m} telemetry={telemetry}/>
      </React.Fragment>)}
    </div>
  </section>);
}
