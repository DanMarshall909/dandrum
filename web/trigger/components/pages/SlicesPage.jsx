import {AnalysisLane} from '../AnalysisLane.jsx';
import {SliceTable} from '../SliceTable.jsx';
import {SliceWaveform} from '../SliceWaveform.jsx';
import React from 'react';
import * as DD from '../design-system/index.jsx';
export function SlicesPage({model}) { return <>{model.isSlices && <>
        <section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px"}}>
          <div style={{"minHeight": "34px","display": "flex","alignItems": "center","flexWrap": "wrap","gap": "6px","padding": "4px 8px 4px 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
            <span style={{"font": "700 13px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Slices</span>
            <span style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-3)"}}>{model.sliceAssetName} · {model.sliceTotal}</span>
            <div style={{"width": "1px","height": "18px","background": "var(--dd-line-2)"}}></div>
            <DD.Button size="sm" icon="slice" selected={model.trShown} onClick={model.detectTransients} disabled={model.trRun}>Detect transients</DD.Button>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Analysis (async)</span></>}
            <DD.SegmentedControl compact={true} value={model.sliceDivision} options={model.divOpts} onChange={value=>{model.setSliceDivision(value);model.divideSlices();}}></DD.SegmentedControl>
            <DD.NumericField showLabel={false} label="Slice count" value={model.sliceCount} min={1} max={128-model.sliceMapFrom} onChange={model.setSliceCount} width={48} compact={true}></DD.NumericField>
            <div style={{"flex": "1"}}></div>
            <DD.IconButton icon="plus" label="Add marker at cursor (M)" size="sm" variant="ghost" onClick={model.addSliceMarker}></DD.IconButton>
            <DD.IconButton icon="minus" label="Delete selected markers (Delete)" size="sm" variant="ghost" onClick={model.deleteSliceMarkers} disabled={!model.sliceSelectionCount}></DD.IconButton>
            <DD.IconButton icon="reset" label="Clear all markers" size="sm" variant="ghost" onClick={model.clearSliceMarkers}></DD.IconButton>
            <DD.IconButton icon="chevron-left" label="Previous slice (←)" size="sm" variant="ghost" onClick={()=>model.stepSlice(-1)}></DD.IconButton>
            <DD.IconButton icon="chevron-right" label="Next slice (→)" size="sm" variant="ghost" onClick={()=>model.stepSlice(1)}></DD.IconButton>
          </div>
          <div style={{"padding": "8px 12px 12px","display": "flex","flexDirection": "column","gap": "6px"}}>
            <div onClick={model.onSliceWaveClick} title="Click a slice to preview it" style={{"cursor": "pointer"}}><SliceWaveform peaks={model.slicePeaks} onMarkerChange={model.moveSliceMarker} assetId={model.sliceAssetId} getTelemetry={model.getTelemetry} subscribeTelemetry={model.subscribeTelemetry} kind="break" height={model.sliceWaveH} slices={model.slices} selectedSlice={model.selSlice} cursor={model.sliceCursor} label={model.sliceAssetName+' · '+model.sliceDuration.toFixed(3)+' s'}></SliceWaveform></div>
            <AnalysisLane model={model}/>
            {model.trDone && <>
              <div style={{"display": "flex","alignItems": "center","gap": "12px","flexWrap": "wrap","padding": "6px 8px","background": "var(--dd-ink-3)","borderRadius": "4px"}}>
                <span style={{"font": "500 13px var(--font-ui)","color": "var(--dd-paper-1)"}}>{model.transientTotal} transients found</span>
                <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>Bright ticks are above the threshold. Existing slices stay until you apply.</span>
                <div style={{"flex": "1"}}></div>
                <DD.Slider compact={true} length={120} value={model.sensitivity} onChange={model.setSensitivity} label="Sensitivity" valueText={Math.round(model.sensitivity*100)+"%"}></DD.Slider>
                <DD.Button size="sm" variant="primary" onClick={model.applyTransients}>Apply as {model.applySliceTotal} slices</DD.Button>
                <DD.Button size="sm" variant="ghost" onClick={model.discardTransients}>Discard</DD.Button>
              </div>
            </>}
            {model.trRun && <>
              <DD.StatusMessage kind="busy" compact={true} title={"Detecting transients · "+Math.round(model.analysisProgress*100)+"%"}>Runs in the background. You can keep editing and playing; slices change only when you apply the result.</DD.StatusMessage><DD.Button size="sm" onClick={model.cancelTransients}>Cancel analysis</DD.Button>
            </>}
            {model.trFail && <>
              <DD.StatusMessage kind="error" compact={true} title="Transient detection failed">{model.sliceAssetName} could not be analysed ({model.analysisError}). Existing slices are unchanged. Try again, or divide evenly or by grid.</DD.StatusMessage>
            </>}
          </div>
        </section>

        <SliceTable model={model}/>
      </>}</>; }
