import React,{useSyncExternalStore} from 'react';
import * as DD from './design-system/index.jsx';
export function SelectorModeBar({model}){return <><div style={{"display": "flex","gap": "2px","padding": "2px","background": "var(--dd-ink-0)","borderRadius": "4px"}}>
                {(model.selModes || []).map((m,index) => <React.Fragment key={m.id ?? m.key ?? index}>
                  <DD.Button size="sm" variant="ghost" selected={m.sel} onClick={m.go}>{m.label}</DD.Button>
                </React.Fragment>)}
              </div></>;}
