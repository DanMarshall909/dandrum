import React, { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { controls, preparedCapabilities } from "./model.mjs";
import { createNoteAudition } from "./note-audition.mjs";
import { createHostKnob } from "../../shared/host-knob.mjs";
import { admittedParameter } from "../../shared/parameter-value.mjs";
import { createHostParameters } from "../../shared/host-parameters.mjs";
import { observePanelHeight } from "../../shared/panel-height.mjs";
import "../../shared/host-knob.css";
import { createMeterTransport } from "../../shared/meter-transport.mjs";
import { meterView } from "../../shared/meter-view.mjs";
import "./styles.css";
import "../../shared/design-tokens.css";
import "../../shared/design-fonts.css";
import { iconProps } from "../../shared/design-icons.mjs";
import "../../shared/design-icons.css";

type Parameter = { id: string; name?: string; value: number };
type HostState = { generation: number; sequence: number; parameters: Parameter[] };
type PreparedDocument = { generation: number; parameters: { id: string; name: string;
  minValue: number; maxValue: number; normalisedDefaultValue: number }[] };
type NativeBackend = {
  getNativeFunction: (name: string) => (...args: unknown[]) => Promise<unknown>;
  addEventListener: (name: string, listener: (state: HostState) => void) => void;
  removeEventListener?: (name: string, listener: (state: HostState) => void) => void;
};

declare global {
  interface Window { __JUCE__?: { backend?: NativeBackend } }
}

const native = (name: string) => window.__JUCE__?.backend?.getNativeFunction(name);
const HostKnob = createHostKnob(React);
const chromaticNames = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
const blackPitches = new Set([1, 3, 6, 8, 10]);
const keyboardKeys = Array.from({ length: 20 }, (_, semitone) => ({
  number: 48 + semitone,
  name: chromaticNames[semitone % 12],
  kind: blackPitches.has(semitone % 12) ? "black" : "white",
  whiteIndex: Array.from({ length: semitone }).filter((_, index) => !blackPitches.has(index % 12)).length,
}));
const whiteKeyCount = keyboardKeys.filter(key => key.kind === "white").length;

const useHostParameters = createHostParameters(React);

function MasterMeter({ generation }: { generation: number | null }) {
  const [view, setView] = useState(() => meterView(null));
  const [meterError, setMeterError] = useState("");

  useEffect(() => {
    if (generation === null || !window.__JUCE__?.backend) return;
    let active = true;
    setView(meterView(null));
    const invoke = async (name: string, ...args: unknown[]) => {
      const call = native(name);
      if (!call) throw new Error(`Meter command ${name} is unavailable`);
      return call(...args);
    };
    const transport = createMeterTransport(invoke, (packet: Record<string, unknown>) => {
      if (active) {
        setView((current: any) => meterView(packet, current));
        setMeterError("");
      }
    });
    const visible = () => {
      void transport.setVisible(!document.hidden).catch((reason: unknown) => {
        if (active) setMeterError(String(reason));
      });
    };
    visible();
    void transport.start(generation).then((accepted: unknown) => {
      if (active && !accepted) setMeterError("Master meter unavailable");
    }).catch((reason: unknown) => { if (active) setMeterError(String(reason)); });
    const timer = window.setInterval(() => {
      void transport.tick().catch((reason: unknown) => {
        if (active) setMeterError(String(reason));
      });
    }, 1000 / 30);
    document.addEventListener("visibilitychange", visible);
    const hide = () => { void transport.setVisible(false).catch(() => {}); };
    window.addEventListener("pagehide", hide);
    return () => {
      active = false;
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", visible);
      window.removeEventListener("pagehide", hide);
    };
  }, [generation]);

  const acknowledge = async (channel: number, ticket: string) => {
    if (generation === null || ticket === "0") return;
    try {
      const accepted = await native("ackMeterClip")?.(channel, generation, ticket);
      if (accepted === true)
        setView(current => ({ ...current, channels: current.channels.map((entry, index) =>
          index === channel && current.generation === generation && entry.ticket === ticket
            ? { ...entry, clipped: false } : entry) }));
    } catch (reason) {
      setMeterError(String(reason));
    }
  };

  return <section className="master-meter" aria-label="Master output meter">
    <header className="meter-heading"><strong><svg {...iconProps('level', { size: 14 })} />MASTER OUTPUT</strong>
      <span>{meterError || (!view.valid ? "WAITING FOR AUDIO" : view.complete ? "LIVE" : "HISTORY GAP")}</span>
    </header>
    {view.channels.map((channel, index) => <div className="meter-row" key={channel.name}>
      <span className="meter-channel">{channel.name}</span>
      <div className="meter-track" role="meter" aria-label={`${channel.name} peak level`}
        aria-valuemin={0} aria-valuemax={1} aria-valuenow={channel.peak}>
        <span className="meter-peak" style={{ width: `${channel.peak * 100}%` }} />
        <span className="meter-rms" style={{ width: `${channel.rms * 100}%` }} />
      </div>
      <button className={`meter-clip ${channel.clipped ? "latched" : ""}`}
        disabled={!channel.clipped || channel.ticket === "0"}
        aria-label={`Clear ${channel.name} clip`}
        onClick={() => { void acknowledge(index, channel.ticket); }}>CLIP</button>
    </div>)}
  </section>;
}

function App() {
  const panel = useRef<HTMLElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  useEffect(() => observePanelHeight(panel.current, frame.current), []);
  const { state, document: preparedDocument, error, command, reportError } = useHostParameters(window.__JUCE__?.backend);
  const [activeKeys, setActiveKeys] = useState<number[]>([]);
  const lastAuditionGeneration = useRef<number | null>(null);
  const auditionAvailable = preparedCapabilities.noteAudition
    && Boolean(window.__JUCE__?.backend) && state !== null;
  const audition = useMemo(() => createNoteAudition(async (name: string, ...args: unknown[]) => {
    const call = native(name);
    if (!call) throw new Error("Note audition requires the Dandrum plugin host.");
    const result = await call(...args);
    if (typeof result === "string") throw new Error(result);
  }, reportError), []);
  const pressKey = (note: number) => {
    if (!state) return;
    setActiveKeys(current => current.includes(note) ? current : [...current, note]);
    void audition.press(note, 0.9, state.generation);
  };
  const releaseKey = (note: number) => {
    setActiveKeys(current => current.filter(active => active !== note));
    void audition.release(note);
  };
  const knob = (position: string) => controls.filter(control => control.position === position)
    .map(control => <div className="knob-control" key={control.id}>
      <HostKnob key={`${control.id}:${state?.generation}`} label={control.label}
        size={position === "program" ? 36 : 64}
        parameter={admittedParameter(state, preparedDocument, control.id)}
        command={command} onError={reportError} /></div>);

  useEffect(() => {
    const keepAlive = window.setInterval(() => { void audition.keepAlive(); }, 500);
    const releaseAll = () => { void audition.releaseAll(); setActiveKeys([]); };
    const releaseWhenHidden = () => { if (document.hidden) releaseAll(); };
    window.addEventListener("pagehide", releaseAll);
    document.addEventListener("visibilitychange", releaseWhenHidden);
    return () => {
      window.clearInterval(keepAlive);
      window.removeEventListener("pagehide", releaseAll);
      document.removeEventListener("visibilitychange", releaseWhenHidden);
      void audition.releaseAll();
    };
  }, [audition]);

  useEffect(() => {
    if (state && lastAuditionGeneration.current !== null
        && state.generation !== lastAuditionGeneration.current) {
      void audition.releaseAll();
      setActiveKeys([]);
    }
    lastAuditionGeneration.current = state?.generation ?? null;
  }, [state?.generation, audition]);

  return <main className="stage">
    <div className="machine-frame" ref={frame}>
      <section className="machine" ref={panel} aria-label="Dandrum TB-303 bass synthesizer">
        <div className="top-shadow" />
        <header className="brand-row">
          <div className="brand"><span className="roland">DANDRUM</span><span className="computer-controlled">BASS SYNTHESIZER</span></div>
          <div className="model-block"><span className="bass-line">ACID BASS</span><span className="model">TB-303</span></div>
        </header>
        <section className="synth-panel">
          <div className="control-section tuning">{knob("tuning")}</div>
          <div className="control-section waveform-section">
            <div className="section-label">WAVEFORM</div>
            <div className="wave-icons">
              <button className="wave-button selected" disabled aria-label="Sawtooth waveform, fixed">〽</button>
              <button className="wave-button square-wave" disabled aria-label="Square waveform unavailable">⎍</button>
            </div>
            <span className="availability">{preparedCapabilities.waveformEditing ? "EDITABLE" : "FIXED SAW"}</span>
          </div>
          <div className="knob-bank">{knob("bank")}</div>
        </section>
        <section className="middle-panel">
          <div className="left-program">
            <div className="mode-buttons"><button disabled>TRACK</button><button disabled>PATTERN</button></div>
            {knob("program")}
            <div className="run-controls"><button className="run-button" disabled>RUN / STOP</button><button className="tap-button" disabled>TAP</button></div>
          </div>
          <div className="sequencer" aria-label="Pattern editor unavailable">
            <div className="sequencer-top"><div className="status-display"><span>PATTERN</span><strong>--</strong></div>
              <div className="switch-bank"><button className="toggle-wrap" disabled>PITCH MODE</button><button className="toggle-wrap" disabled>TIME MODE</button></div></div>
            <div className="step-grid">{Array.from({ length: 16 }, (_, index) =>
              <button key={index} className="step-button" disabled>{index + 1}</button>)}</div>
            <div className="function-row">{["TRANSPOSE DOWN", "TRANSPOSE UP", "ACCENT", "SLIDE", "REST", "TIE"].map(label =>
              <button key={label} disabled>{label}</button>)}</div>
            <span className="availability">{preparedCapabilities.patternEditing || preparedCapabilities.transport ? "" : "PATTERN AND TRANSPORT UNAVAILABLE"}</span>
          </div>
        </section>
        <section className="keyboard-panel" aria-label="Note audition">
          <div className="keyboard-labels"><span><svg {...iconProps('keyboard', { size: 14 })} />NOTE AUDITION</span><span><svg {...iconProps('midi', { size: 14 })} />HOST MIDI INPUT ACTIVE</span></div>
          <div className="keyboard">
            <div className="white-keys" style={{ gridTemplateColumns: `repeat(${whiteKeyCount}, 1fr)` }}>
              {keyboardKeys.filter(key => key.kind === "white").map(key =>
                <button key={key.number} className={`key white ${activeKeys.includes(key.number) ? "active" : ""}`}
                  disabled={!auditionAvailable} aria-label={`${key.name} MIDI note ${key.number}`}
                  onPointerDown={event => { event.currentTarget.setPointerCapture(event.pointerId); pressKey(key.number); }}
                  onPointerUp={() => releaseKey(key.number)} onPointerCancel={() => releaseKey(key.number)}
                  onPointerLeave={() => releaseKey(key.number)}
                  onKeyDown={event => {
                    if (event.key === " " || event.key === "Enter") { event.preventDefault(); pressKey(key.number); }
                  }}
                  onKeyUp={event => {
                    if (event.key === " " || event.key === "Enter") { event.preventDefault(); releaseKey(key.number); }
                  }}><span className="key-led" /><span className="key-label">{key.name}</span></button>)}
            </div>
            <div className="black-keys">{keyboardKeys.filter(key => key.kind === "black").map(key =>
              <button key={key.number} className={`key black ${activeKeys.includes(key.number) ? "active" : ""}`}
                disabled={!auditionAvailable} aria-label={`${key.name} MIDI note ${key.number}`}
                style={{ left: `${(key.whiteIndex / whiteKeyCount) * 100}%`, width: `calc(100% / ${whiteKeyCount * 1.6})` }}
                onPointerDown={event => { event.currentTarget.setPointerCapture(event.pointerId); pressKey(key.number); }}
                onPointerUp={() => releaseKey(key.number)} onPointerCancel={() => releaseKey(key.number)}
                onPointerLeave={() => releaseKey(key.number)}
                onKeyDown={event => {
                  if (event.key === " " || event.key === "Enter") { event.preventDefault(); pressKey(key.number); }
                }}
                onKeyUp={event => {
                  if (event.key === " " || event.key === "Enter") { event.preventDefault(); releaseKey(key.number); }
                }}>
                <span className="key-led" /><span className="key-label">{key.name}</span></button>)}</div>
          </div>
        </section>
        <footer>
          <div className="footer-centre"><span>DANDRUM</span><strong>TB-303</strong></div>
          <MasterMeter generation={state?.generation ?? null} />
        </footer>
      </section>
    </div>
    <p className="hint">Drag a knob or use arrow keys. Values follow the host.</p>
    {error && <p className="host-error" role="alert"><svg {...iconProps('error')} />{error}</p>}
  </main>;
}

createRoot(document.getElementById("root")!).render(<App />);
