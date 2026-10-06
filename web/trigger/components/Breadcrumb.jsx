import React from 'react';
import {PropertyEditor} from './PropertyEditor.jsx';
import * as DD from './design-system/index.jsx';
export function Breadcrumb({model}){return <><div style={{"display": "flex","alignItems": "center","gap": "4px","flexWrap": "wrap","font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>
          {(model.crumbs || []).map((c,index) => <React.Fragment key={c.id ?? c.key ?? index}>
            <span style={{"display": "flex","alignItems": "center","gap": "4px"}}><span style={{"color": c.c}}>{c.label}</span>{c.sep && <><span style={{"color": "var(--dd-paper-4)"}}>›</span></>}</span>
          </React.Fragment>)}
        </div></>;}
