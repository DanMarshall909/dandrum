import React from 'react';
import {ProcessorStack} from '../ProcessorStack.jsx';
import * as DD from '../design-system/index.jsx';
export function EffectsPage({model}) { return <>{model.isFx && <>
        {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","alignSelf": "flex-start"}}>Processors are ordinary DSP Modules inserted by Connection; no “sampler effect” type</span></>}
        <ProcessorStack model={model} layers={model.insertLayers} title="Group inserts" subtitle="Processed left to right before the output bus" outputs={model.outputs} countLabel="3 groups"></ProcessorStack>
        <ProcessorStack model={model} layers={model.fxLayers} title="FX busses" subtitle="Fed by sends on the Routing page" outputs={model.outputs} countLabel="2 busses"></ProcessorStack>
      </>}</>; }
