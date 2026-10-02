import React, { useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { preparedPads, selectedRegion, visibleParameters, normalizedDraft,
  needsDocumentRefresh, padReleaseHandlers, auditionFocusRelease } from './model.mjs';
import { createParameterGesture } from './parameter-gesture.mjs';
import { createNoteAudition } from '../../tb303/src/note-audition.mjs';
import { createMeterTransport } from '../../shared/meter-transport.mjs';
import { meterView } from '../../shared/meter-view.mjs';
import { createWaveformTransport } from '../../shared/waveform-transport.mjs';
import { preparedWaveformView, paintPreparedWaveform } from '../../shared/prepared-waveform.mjs';
import './styles.css';
import '../../shared/design-tokens.css';
import '../../shared/design-fonts.css';
import { iconProps } from '../../shared/design-icons.mjs';
import '../../shared/design-icons.css';

type HostParameter = { id: string; name: string; value: number };
type HostState = { generation: number; sequence: number; parameters: HostParameter[] };
type PreparedParameter = { id: string; name: string; scope: string; controlGroup?: number };
type PreparedRegion = { id: string };
type PreparedSource = { id: string; sampleRateHz: number; regions: PreparedRegion[] };
type PreparedDocument = {
  generation: number;
  instrumentId: string;
  sources: PreparedSource[];
  maps: unknown[];
  parameters: PreparedParameter[];
  capabilities: { sampleKeyMap: boolean; preparedWaveform: boolean;
    synthLayer: boolean; nestedPatchLayer: boolean; moduleChain: boolean };
};
type Backend = {
  getNativeFunction: (name: string) => (...args: unknown[]) => Promise<unknown>;
  addEventListener: (name: string, listener: (state: HostState) => void) => void;
  removeEventListener?: (name: string, listener: (state: HostState) => void) => void;
};
type WaveStatus = { state: string; generation?: number; result?: unknown; error?: string };

declare global {
  interface Window { __JUCE__?: { backend?: Backend } }
}

async function invoke(name: string, ...args: unknown[]): Promise<any> {
  const native = window.__JUCE__?.backend?.getNativeFunction(name);
  if (!native) throw new Error(`Host command ${name} is unavailable`);
  const reply = await native(...args);
  if (typeof reply === 'string') throw new Error(reply);
  return reply;
}

function useHost() {
  const [state, setState] = useState<HostState | null>(null);
  const [document, setDocument] = useState<PreparedDocument | null>(null);
  const [error, setError] = useState('');
  const latestGeneration = useRef(0);
  const documentGeneration = useRef(-1);
  const documentRequest = useRef<Promise<void> | null>(null);

  const acceptState = (next: HostState) => {
    if (!next || !Number.isInteger(next.generation) || !Array.isArray(next.parameters))
      return;
    latestGeneration.current = Math.max(latestGeneration.current, next.generation);
    setState(current => current && (next.generation < current.generation
      || (next.generation === current.generation && next.sequence < current.sequence))
      ? current : next);
  };
  const refreshDocument = (): Promise<void> => {
    if (documentRequest.current) return documentRequest.current;
    const requestedGeneration = latestGeneration.current;
    const request = invoke('getPreparedDocument').then((next: PreparedDocument | null) => {
      if (next && Number.isInteger(next.generation)
          && next.generation >= latestGeneration.current) {
        documentGeneration.current = next.generation;
        setDocument(current => current && current.generation > next.generation ? current : next);
      }
    }).finally(() => {
      documentRequest.current = null;
      if (latestGeneration.current > requestedGeneration
          && needsDocumentRefresh(latestGeneration.current, documentGeneration.current))
        void refreshDocument().catch(reason => setError(String(reason)));
    });
    documentRequest.current = request;
    return request;
  };
  const refresh = async () => {
    const next = await invoke('getParameterState') as HostState;
    acceptState(next);
    await refreshDocument();
  };

  useEffect(() => {
    const backend = window.__JUCE__?.backend;
    if (!backend) {
      setError('Open this panel in the Dandrum plugin host.');
      return;
    }
    const changed = (next: HostState) => {
      acceptState(next);
      if (needsDocumentRefresh(latestGeneration.current, documentGeneration.current))
        void refreshDocument().catch(reason => setError(String(reason)));
    };
    backend.addEventListener('parameterStateChanged', changed);
    void refresh().catch(reason => setError(String(reason)));
    return () => backend.removeEventListener?.('parameterStateChanged', changed);
  }, []);

  const command = async (name: string, ...args: unknown[]) => {
    try {
      const reply = await invoke(name, ...args);
      if (reply?.status && reply.status !== 'accepted')
        throw new Error(`Host rejected ${name}: ${reply.status}`);
      setError('');
      if (name === 'setParameter')
        acceptState(await invoke('getParameterState') as HostState);
      return reply;
    } catch (reason) {
      setError(String(reason));
      if (name === 'setParameter')
        void invoke('getParameterState').then(next => acceptState(next as HostState))
          .catch(() => {});
      throw reason;
    }
  };
  return { state, document, error, command, reportError: (reason: unknown) => setError(String(reason)) };
}

type SliderProps = {
  descriptor: PreparedParameter;
  value: number;
  generation: number;
  command: (name: string, ...args: unknown[]) => Promise<any>;
};

function HostSlider({ descriptor, value, generation, command }: SliderProps) {
  const [local, setLocal] = useState(value);
  const [draft, setDraft] = useState(value.toFixed(3));
  const dragging = useRef(false);
  const cancelDraft = useRef(false);
  const authoritative = useRef(value);
  authoritative.current = value;
  const gesture = useMemo(() => createParameterGesture(
    (name: string, next: number | undefined, editGeneration: number) =>
      command(name, descriptor.id, ...(next === undefined ? [] : [next]), editGeneration),
    () => {
      const current = authoritative.current;
      setLocal(current);
      setDraft(current.toFixed(3));
    }), [descriptor.id, generation]);
  useEffect(() => () => { void gesture.end(); }, [gesture]);
  useEffect(() => {
    if (!dragging.current) {
      setLocal(value);
      setDraft(value.toFixed(3));
    }
  }, [value, generation]);
  const finish = () => {
    if (!dragging.current) return;
    dragging.current = false;
    void gesture.end();
  };
  const commitDraft = () => {
    const next = normalizedDraft(draft, cancelDraft.current);
    cancelDraft.current = false;
    if (next !== null) {
      setLocal(next);
      gesture.commit(next, generation);
    } else setDraft(value.toFixed(3));
  };
  return <label className="control">
    <span>{descriptor.name || descriptor.id}</span>
    <input type="range" min="0" max="1" step="0.001" value={local}
      aria-label={descriptor.name || descriptor.id}
      onPointerDown={event => {
        if (!dragging.current) {
          event.currentTarget.setPointerCapture(event.pointerId);
          dragging.current = true;
          gesture.begin(generation);
        }
      }}
      onChange={event => {
        const next = Number(event.currentTarget.value);
        setLocal(next);
        setDraft(next.toFixed(3));
        gesture.change(next, generation);
      }}
      onPointerUp={finish} onPointerCancel={finish} onBlur={finish} />
    <input className="control-value" type="number" min="0" max="1" step="0.001"
      value={draft} aria-label={`${descriptor.name || descriptor.id} normalized value`}
      onFocus={() => { cancelDraft.current = false; }}
      onChange={event => setDraft(event.currentTarget.value)}
      onBlur={commitDraft}
      onKeyDown={event => {
        if (event.key === 'Enter') event.currentTarget.blur();
        if (event.key === 'Escape') {
          cancelDraft.current = true;
          setDraft(value.toFixed(3));
          event.currentTarget.blur();
        }
      }} />
  </label>;
}

function MasterMeter({ generation, reportError }: {
  generation: number; reportError: (reason: unknown) => void;
}) {
  const [display, setDisplay] = useState(() => meterView(null));
  useEffect(() => {
    const transport = createMeterTransport(invoke, (packet: any) => setDisplay(meterView(packet)));
    const visible = () => void transport.setVisible(!window.document.hidden).catch(reportError);
    visible();
    void transport.start(generation).catch(reportError);
    const timer = window.setInterval(() => void transport.tick().catch(reportError), 1000 / 30);
    window.document.addEventListener('visibilitychange', visible);
    return () => {
      window.clearInterval(timer);
      window.document.removeEventListener('visibilitychange', visible);
      void transport.setVisible(false).catch(reportError);
    };
  }, [generation]);
  return <section className="panel meter" aria-label="Master output meter">
    <div className="section-heading"><h2><svg {...iconProps('level')} />Master output</h2>
      <span>{display.valid ? display.complete ? 'LIVE' : 'HISTORY GAP' : 'WAITING FOR AUDIO'}</span></div>
    {display.channels.map((channel, index) => <div className="meter-row" key={channel.name}>
      <strong>{channel.name}</strong>
      <div className="meter-track" role="meter" aria-label={`${channel.name} peak level`}
        aria-valuemin={0} aria-valuemax={1} aria-valuenow={channel.peak}>
        <i className="meter-peak" style={{ width: `${channel.peak * 100}%` }} />
        <i className="meter-rms" style={{ width: `${channel.rms * 100}%` }} />
      </div>
      <button className={channel.clipped ? 'clip latched' : 'clip'}
        disabled={!channel.clipped || channel.ticket === '0'}
        onClick={() => void invoke('ackMeterClip', index, generation, channel.ticket)
          .then(() => setDisplay(current => ({ ...current, channels: current.channels.map(
            (item, position) => position === index ? { ...item, clipped: false } : item) })))
          .catch(reportError)}>CLIP</button>
    </div>)}
  </section>;
}

function PreparedWaveform({ document, sourceId, regionId, reportError }: {
  document: PreparedDocument; sourceId: string; regionId: string;
  reportError: (reason: unknown) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [status, setStatus] = useState<WaveStatus | null>(null);
  const [width, setWidth] = useState(1);
  useEffect(() => {
    setStatus(null);
    const transport = createWaveformTransport(invoke, (packet: { status: WaveStatus }) =>
      setStatus(packet.status));
    void transport.select({ sourceId, regionId, channel: 0, buckets: 512,
      generation: document.generation }).catch(reportError);
    const timer = window.setInterval(() => void transport.poll().catch(reportError), 1000 / 30);
    return () => { window.clearInterval(timer); void transport.close().catch(reportError); };
  }, [document.generation, sourceId, regionId]);
  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    const resize = () => setWidth(Math.max(1, Math.round(element.clientWidth)));
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const view = useMemo(() => preparedWaveformView(document, sourceId, regionId,
    status, width, 180), [document, sourceId, regionId, status, width]);
  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext('2d');
    if (!element || !context) return;
    const ratio = window.devicePixelRatio || 1;
    element.width = Math.round(width * ratio);
    element.height = Math.round(180 * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    if (view) paintPreparedWaveform(context, view);
    else { context.fillStyle = '#111916'; context.fillRect(0, 0, width, 180); }
  }, [view, width]);
  return <section className="panel waveform" aria-label="Prepared sample waveform">
    <div className="section-heading"><h2>{sourceId}.{regionId}</h2>
      <span>{view ? `${view.sampleRateHz} Hz · ${view.durationSeconds.toFixed(3)} s`
        : status?.state === 'failed' ? status.error || 'WAVEFORM UNAVAILABLE'
        : 'PREPARING WAVEFORM'}</span></div>
    <canvas ref={canvas} aria-label={`Signed waveform for ${sourceId}.${regionId}`} />
  </section>;
}

function SamplerApp() {
  const { state, document, error, command, reportError } = useHost();
  const pads = useMemo(() => preparedPads(document), [document]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = pads.find(pad => pad.id === selectedId) ?? pads[0] ?? null;
  const region = selectedRegion(document, selected);
  const parameters = visibleParameters(document, selected) as PreparedParameter[];
  const audition = useMemo(() => createNoteAudition(invoke, reportError), []);
  const [pressed, setPressed] = useState<string[]>([]);
  useEffect(() => {
    void audition.releaseAll();
    setPressed([]);
  }, [document?.generation]);
  useEffect(() => {
    const heartbeat = window.setInterval(() => void audition.keepAlive(), 500);
    const release = auditionFocusRelease(audition, () => setPressed([]));
    const visibility = () => { if (window.document.hidden) release(); };
    window.addEventListener('pagehide', release);
    window.addEventListener('blur', release);
    window.document.addEventListener('visibilitychange', visibility);
    return () => {
      window.clearInterval(heartbeat);
      window.removeEventListener('pagehide', release);
      window.removeEventListener('blur', release);
      window.document.removeEventListener('visibilitychange', visibility);
      void audition.releaseAll();
    };
  }, [audition]);

  const press = (pad: typeof pads[number]) => {
    if (!document) return;
    setSelectedId(pad.id);
    setPressed(current => current.includes(pad.id) ? current : [...current, pad.id]);
    void audition.press(pad.keyLow, pad.velocity, document.generation);
  };
  const release = (pad: typeof pads[number]) => {
    setPressed(current => current.filter(id => id !== pad.id));
    void audition.release(pad.keyLow);
  };
  const current = state && document && state.generation === document.generation;
  return <main className="sampler">
    <header className="top"><div><p className="eyebrow">DANDRUM · PREPARED INSTRUMENT</p>
      <h1>Drum Sampler</h1><p>{current ? document.instrumentId : 'Loading prepared instrument…'}</p></div>
      <span className="generation">{current ? `GEN ${document.generation}` : 'WAITING'}</span></header>
    {error && <p className="error" role="alert"><svg {...iconProps('error')} />{error}</p>}
    {current && <>
      <section className="panel keymap" aria-label="Prepared key map">
        <div className="section-heading"><h2><svg {...iconProps('keyboard')} />Key map</h2><span>PREPARED · READ ONLY</span></div>
        <div className="pads">{pads.map(pad => <button key={pad.id}
          className={`pad ${selected?.id === pad.id ? 'selected' : ''} ${pressed.includes(pad.id) ? 'pressed' : ''}`}
          aria-label={`${pad.label}, MIDI ${pad.keyLow}, velocity ${pad.velocityLow} to ${pad.velocityHigh}`}
          onPointerDown={event => { event.currentTarget.setPointerCapture(event.pointerId); press(pad); }}
          {...padReleaseHandlers(() => release(pad))}
          onClick={() => setSelectedId(pad.id)}
          onKeyDown={event => { if (!event.repeat && (event.key === ' ' || event.key === 'Enter')) {
            event.preventDefault(); press(pad); } }}>
          <strong>{pad.label}</strong>
          <small>NOTE {pad.keyLow}{pad.keyHigh !== pad.keyLow ? `–${pad.keyHigh}` : ''}
            {' · '}VEL {pad.velocityLow}–{pad.velocityHigh}</small>
          {pad.zoneIds.length > 1 && <small><svg {...iconProps('alternate', { size: 12 })} />{pad.zoneIds.length} round robin alternatives</small>}
          {pad.chokeGroup && <small><svg {...iconProps('choke', { size: 12 })} />CHOKE {pad.chokeGroup}</small>}
        </button>)}</div>
        {selected && <p className="details">{selected.selectionMode} selection · {selected.zoneIds.join(', ')}
          {selected.controlGroup != null ? ` · control group ${selected.controlGroup}` : ''}</p>}
      </section>
      <div className="lower">
        <div className="left-column">
          <section className="panel controls" aria-label="Host modulatable controls">
            <div className="section-heading"><h2><svg {...iconProps('host')} />Public controls</h2>
              <span>{parameters.length} SHARED / SELECTED PAD</span></div>
            <div className="control-list">{parameters.map(descriptor => <HostSlider
              key={`${descriptor.id}:${state.generation}`} descriptor={descriptor}
              value={state.parameters.find(parameter => parameter.id === descriptor.id)?.value
                ?? 0} generation={state.generation} command={command} />)}</div>
          </section>
          <MasterMeter generation={document.generation} reportError={reportError} />
        </div>
        <div className="right-column">
          {region && document.capabilities.preparedWaveform
            ? <PreparedWaveform document={document} sourceId={region.sourceId}
                regionId={region.regionId} reportError={reportError} />
            : <section className="panel">Prepared waveform unavailable</section>}
          <section className="panel availability" aria-label="Prepared capabilities">
            <div className="section-heading"><h2><svg {...iconProps('lock')} />Structure</h2><span>INSPECT ONLY</span></div>
            <p>Source and region assignments are prepared outside the plugin.</p>
            <p>Layer details: {document.capabilities.synthLayer || document.capabilities.nestedPatchLayer
              || document.capabilities.moduleChain ? 'Available where prepared' : 'Unavailable'}</p>
            <p>Output bus details unavailable.</p>
          </section>
        </div>
      </div>
    </>}
  </main>;
}

createRoot(document.getElementById('root')!).render(<SamplerApp />);
