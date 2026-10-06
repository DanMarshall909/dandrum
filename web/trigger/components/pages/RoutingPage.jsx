import {RoutingTable} from '../RoutingTable.jsx';
import React from 'react';
import {OutputBusPanel} from '../OutputBusPanel.jsx';
import {RoutingGraph} from '../RoutingGraph.jsx';
import * as DD from '../design-system/index.jsx';
export function RoutingPage({model}) { return <>{model.isRouting && <>
        <div style={{"display": "grid","gridTemplateColumns": model.routeCols,"gap": "4px","alignItems": "start"}}>
          <RoutingTable model={model}/>
          <div style={{"display": "flex","flexDirection": "column","gap": "4px","minWidth": "0"}}>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","alignSelf": "flex-start"}}>Bus → plugin output Connection</span></>}
            <div className="output-buses"><OutputBusPanel model={model}/></div>
          </div>
        </div>
      </>}</>; }
