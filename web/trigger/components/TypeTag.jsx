import {Breadcrumb} from './Breadcrumb.jsx';
import React from 'react';
import {PropertyEditor} from './PropertyEditor.jsx';
import * as DD from './design-system/index.jsx';
export function TypeTag({model}){return <><span style={{"font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-2)","border": "1px solid var(--dd-line-3)","borderRadius": "2px","padding": "1px 5px"}}>{model.insp.type}</span></>;}
