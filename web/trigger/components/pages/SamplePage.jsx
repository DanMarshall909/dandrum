import {AnalysisStatus} from '../AnalysisStatus.jsx';
import {WaveformEditor} from '../WaveformEditor.jsx';
import {WaveformPreview} from '../WaveformPreview.jsx';
import React from 'react';
import * as DD from '../design-system/index.jsx';
export function SamplePage({model}) { return <>{model.isSample && <>
        <section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","display": "flex","flexDirection": "column"}}>
          <div style={{"minHeight": "34px","display": "flex","alignItems": "center","flexWrap": "wrap","gap": "8px","padding": "4px 8px 4px 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
            <span style={{"font": "700 13px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Sample</span>
            <DD.MenuButton value={model.assetId} options={model.assetChoices} onChange={model.selectAsset} compact={true} width={180}></DD.MenuButton>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Asset binding on AssetPlayer</span></>}
            <DD.IconButton icon="chevron-right" label="Audition (Space)" size="sm" onClick={model.onWaveClick}></DD.IconButton>
            <div style={{"flex": "1"}}></div>
            <DD.SegmentedControl compact={true} value={model.playMode} options={model.playOpts} onChange={model.setPlayback}></DD.SegmentedControl>
            <DD.IconButton icon="reverse" label="Reverse" size="sm" selected={model.reverse} onClick={model.toggleReverse}></DD.IconButton>
            {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Playback Policy</span></>}
          </div>
          <div style={{"padding": "8px 12px 12px","display": "flex","flexDirection": "column","gap": "6px"}}>
            <div style={{"position": "relative"}}>
              <WaveformPreview assetId={model.assetId} getTelemetry={model.getTelemetry} subscribeTelemetry={model.subscribeTelemetry} peaks={model.pianoPeaks} height={28} compact={true} showDisplayToggle={false} regionStart={0} regionEnd={1}></WaveformPreview>
              <div style={{"position": "absolute","top": "0","bottom": "0","left": (model.viewport.offset*100)+"%","width": (100/model.viewport.zoom)+"%","border": "1px solid var(--dd-paper-2)","borderRadius": "2px","pointerEvents": "none"}}></div>
            </div>
            {model.isLoading && <>
              <div style={{"position": "relative"}}>
                <WaveformPreview assetId={model.assetId} getTelemetry={model.getTelemetry} subscribeTelemetry={model.subscribeTelemetry} peaks={model.partialPeaks} height={model.waveH} showDisplayToggle={false} label={model.assetName+" · reading"}></WaveformPreview>
                <div style={{"position": "absolute","left": "12px","bottom": "12px","display": "flex","flexDirection": "column","gap": "6px","width": "280px"}}>
                  <DD.StatusMessage kind="busy" inline={true} title={"Loading "+model.assetName}>{Math.round(model.loadingProgress*100)}% · {(model.loadingProgress*model.duration).toFixed(1)} of {model.duration.toFixed(1)} s</DD.StatusMessage>
                  <div style={{"height": "4px","background": "var(--dd-ink-4)","borderRadius": "2px"}}><div style={{"width": (model.loadingProgress*100)+"%","height": "4px","background": "var(--dd-paper-2)","borderRadius": "2px"}}></div></div>
                </div>
              </div>
            </>}
            {model.waveReady && <>
              <div onDrop={model.onSampleDrop} onDragOver={model.onFileDrag} onClick={model.onWaveClick} title="Click to preview" style={{"position": "relative","cursor": "pointer"}}>
                <WaveformEditor snapPosition={model.snapPosition} viewport={model.viewport} onViewport={model.setViewport} onRegionChange={model.setRegion} assetId={model.assetId} getTelemetry={model.getTelemetry} subscribeTelemetry={model.subscribeTelemetry} peaks={model.pianoPeaks} height={model.waveH} regionStart={model.hRs} regionEnd={model.hRe} fadeIn={model.hFi} fadeOut={model.hFo} loopStart={model.loopS} loopEnd={model.loopE} crossfade={model.xfade} cursor={model.cursor} missing={model.missing} label={model.waveLabel}></WaveformEditor>
                {model.unsup && <><div style={{"position": "absolute","inset": "0","border": "1px dashed var(--dd-warn)","borderRadius": "2px","pointerEvents": "none"}}></div></>}
                {model.ann && <><span style={{"position": "absolute","top": "6px","right": "8px","font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>WaveformEditor · layers: peaks (Analysis), region, loop, playhead (Telemetry)</span></>}
              </div>
            </>}
            <div style={{"display": "flex","gap": "16px","flexWrap": "wrap","font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>
              {(model.isMin?model.meta.slice(0,3):model.meta || []).map((m,index) => <React.Fragment key={m.id ?? m.key ?? index}><span >{m.k} <b style={{"font": "500 12px var(--font-value)","color": "var(--dd-paper-2)"}}>{m.v}</b></span></React.Fragment>)}
            </div>
          </div>
        </section>

        <AnalysisStatus job={model.analysis}/>
        {model.loop && <>
          <section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px"}}>
            <div style={{"height": "30px","display": "flex","alignItems": "center","gap": "8px","padding": "0 8px 0 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)","borderRadius": "6px 6px 0 0"}}>
              <span style={{"font": "700 13px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Loop</span>
              {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Asset Region (loop) + Playback Policy</span></>}
              <div style={{"flex": "1"}}></div>
              <DD.Toggle compact={true} label="Snap to zero crossings" checked={model.snap} onChange={model.setSnap}></DD.Toggle>
            </div>
            <div style={{"display": "grid","gridTemplateColumns": "minmax(0,1fr) minmax(0,1.3fr)","gap": "16px","padding": "12px"}}>
              <div style={{"display": "flex","flexDirection": "column","gap": "8px"}}>
                <div style={{"display": "flex","gap": "8px","flexWrap": "wrap"}}>
                  <DD.NumericField label="Loop start" min={0} max={model.duration} value={model.loopS*model.duration} onChange={v=>model.setLoop('start',v/model.duration)} unit="s" step={0.001} width={96} compact={true}></DD.NumericField>
                  <DD.NumericField label="Loop end" min={0} max={model.duration} value={model.loopE*model.duration} onChange={v=>model.setLoop('end',v/model.duration)} unit="s" step={0.001} width={96} compact={true}></DD.NumericField>
                  <DD.NumericField label="Crossfade" min={0} max={model.duration*1000} value={model.xfade*model.duration*1000} onChange={v=>model.setLoop('crossfade',v/model.duration/1000)} unit="ms" width={84} compact={true}></DD.NumericField>
                </div>
                <span style={{"font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)"}}>Suggested loops · from analysis</span>
                <div style={{"display": "flex","gap": "4px","flexWrap": "wrap"}}>
                  {model.loopSuggestions.map(loop=><DD.Button key={loop.label} size="sm" selected={loop.selected} onClick={loop.select}>{loop.label}</DD.Button>)}
                  <DD.Button size="sm" onClick={model.findLoops}>Find loops</DD.Button>
                </div>
              </div>
              <div style={{"display": "flex","flexDirection": "column","gap": "4px"}}>
                <span style={{"font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)"}}>Loop seam · end → start, ±20 ms</span>
                <div style={{"position": "relative","display": "grid","gridTemplateColumns": "1fr 1fr","gap": "0"}}>
                  <WaveformPreview assetId={model.assetId} getTelemetry={model.getTelemetry} subscribeTelemetry={model.subscribeTelemetry} peaks={model.seamA} height={56} compact={true} showDisplayToggle={false}></WaveformPreview>
                  <WaveformPreview assetId={model.assetId} getTelemetry={model.getTelemetry} subscribeTelemetry={model.subscribeTelemetry} peaks={model.seamB} height={56} compact={true} showDisplayToggle={false}></WaveformPreview>
                  <div style={{"position": "absolute","top": "0","bottom": "0","left": "50%","width": "1px","background": "var(--dd-vermilion)"}}></div>
                </div>
              </div>
            </div>
          </section>
        </>}

        <section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","padding": "6px 8px 8px","display": "flex","flexWrap": "wrap","alignItems": "flex-start","gap": "12px"}}>
          {(model.sampleKnobGroups || []).map((g,index) => <React.Fragment key={g.id ?? g.key ?? index}>
            <div style={{"display": "flex","flexDirection": "column","gap": "8px"}}>
              <div style={{"display": "flex","alignItems": "center","gap": "6px"}}>
                <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>{g.title}</span>
                {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>{g.ann}</span></>}
              </div>
              <div style={{"display": "flex","gap": "0"}}>
                {(g.knobs || []).map((k,index) => <React.Fragment key={k.id ?? k.key ?? index}>
                  <div data-knob={k.key} data-reset={"knob|" + (k.key)} style={{"borderRadius": "4px","boxShadow": k.ring}}><DD.Knob {...model.kTight} size="sm" value={k.v} defaultValue={k.default} parseValue={k.parse} onCommit={k.commit} onChange={k.set} bipolar={k.bi} label={k.label} valueText={k.t} modulations={k.mods} hostAutomated={k.host} assigning={model.assigning}></DD.Knob></div>
                </React.Fragment>)}
              </div>
            </div>
          </React.Fragment>)}
          <div style={{"display": "flex","flexDirection": "column","gap": "8px"}}>
            <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Root</span>
            <DD.NumericField label="Root note" value={model.root} min={0} max={127} onChange={model.setRoot} format={model.fmtNote} width={84} compact={true}></DD.NumericField>
            <span style={{"font": "500 11px var(--font-ui)","color": "var(--dd-paper-3)"}}>{model.detectedPitch??"Pitch not analysed"}</span>
          </div>
        </section>
      </>}</>; }
