import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';
export function MacroControl({model,macro:m,telemetry}){return <><div onContextMenuCapture={e=>model.openMacroMenu(e,m.id)} onClick={()=>model.selectMacro(m.id)} data-ctx={"macro|" + (m.id)} data-reset={"macro|" + (m.id)} title={(m.d) + " · right-click for destinations · middle-click resets"} style={{"display": "flex","justifyContent": "center","width": "64px","borderRadius": "4px","background": m.bg,"boxShadow": m.edge,"minWidth": "0"}}>
          <DD.Knob {...model.kTight} defaultValue={m.default} size={model.macroSize} value={telemetry.hostValues?.[m.id]??m.v} onChange={m.set} label={m.name} hostAutomated={telemetry.host[m.id]??m.host} disabled={m.off}></DD.Knob>
        </div></>;}
