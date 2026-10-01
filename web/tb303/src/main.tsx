import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { acceptState, controls, displayValue, preparedCapabilities } from "./model.mjs";
import "./styles.css";

type Parameter = { id: string; name?: string; value: number };
type HostState = { generation: number; sequence: number; parameters: Parameter[] };
type NativeBackend = {
  getNativeFunction: (name: string) => (...args: unknown[]) => Promise<unknown>;
  addEventListener: (name: string, listener: (state: HostState) => void) => void;
  removeEventListener?: (name: string, listener: (state: HostState) => void) => void;
};

declare global {
  interface Window { __JUCE__?: { backend?: NativeBackend } }
}

const native = (name: string) => window.__JUCE__?.backend?.getNativeFunction(name);
const clamp = (value: number) => Math.max(0, Math.min(1, value));
const chromaticNames = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
const blackPitches = new Set([1, 3, 6, 8, 10]);
const keyboardKeys = Array.from({ length: 20 }, (_, semitone) => ({
  name: chromaticNames[semitone % 12],
  kind: blackPitches.has(semitone % 12) ? "black" : "white",
  whiteIndex: Array.from({ length: semitone }).filter((_, index) => !blackPitches.has(index % 12)).length,
}));
const whiteKeyCount = keyboardKeys.filter(key => key.kind === "white").length;

function useHostParameters() {
  const [state, setState] = useState<HostState | null>(null);
  const [error, setError] = useState("");
  const current = useRef<HostState | null>(null);
  const admitted = useRef(0);
  const pendingWrites = useRef(0);

  const applyState = (incoming: HostState) => {
    if (current.current && incoming?.generation > current.current.generation) {
      admitted.current = 0;
      pendingWrites.current = 0;
    } else if (pendingWrites.current > 0) {
      return;
    }
    const next = acceptState(current.current, incoming, admitted.current);
    if (next !== current.current) {
      current.current = next;
      setState(next);
    }
  };

  const requestState = async () => {
    const get = native("getParameterState");
    if (get) applyState(await get() as HostState);
  };

  useEffect(() => {
    const backend = window.__JUCE__?.backend;
    if (!backend) {
      setError("This panel requires the Dandrum plugin host.");
      return;
    }
    backend.addEventListener("parameterStateChanged", applyState);
    void requestState().catch(reason => setError(String(reason)));
    return () => backend.removeEventListener?.("parameterStateChanged", applyState);
  }, []);

  const command = async (name: string, id: string, value?: number, requestedGeneration?: number) => {
    const generation = requestedGeneration ?? current.current?.generation;
    const call = native(name);
    if (!call || generation === undefined) return;
    const write = name === "setParameter";
    if (write) pendingWrites.current++;
    try {
      const result = await (write ? call(id, value, generation) : call(id, generation));
      if (typeof result === "string") throw new Error(result);
      const reply = result as { status?: string; generation?: number; sequence?: number } | null;
      if (reply?.status === "accepted" && reply.generation === current.current?.generation)
        admitted.current = Math.max(admitted.current, reply.sequence ?? 0);
      setError("");
    } catch (reason) {
      setError(String(reason));
    } finally {
      if (write) pendingWrites.current = Math.max(0, pendingWrites.current - 1);
      void requestState().catch(reason => setError(String(reason)));
    }
  };

  return { state, error, command };
}

type KnobProps = {
  id: string;
  label: string;
  value: number | null;
  generation: number | null;
  size?: "small" | "large";
  command: (name: string, id: string, value?: number, generation?: number) => Promise<void>;
};

function Knob({ id, label, value, generation, size = "large", command }: KnobProps) {
  const drag = useRef<{ pointer: number; startY: number; startValue: number; generation: number; latest: number | null } | null>(null);
  const pumpPromise = useRef<Promise<void> | null>(null);
  const beginPromise = useRef<Promise<void> | null>(null);
  const finishing = useRef(false);
  const editGeneration = useRef<number | null>(null);
  const cancelEdit = useRef(false);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const angle = -135 + (value ?? 0) * 270;

  const commitDraft = () => {
    const next = Number(draft);
    setEditing(false);
    if (!cancelEdit.current && editGeneration.current === generation
        && draft.trim() !== "" && Number.isFinite(next) && next >= 0 && next <= 1)
      void command("setParameter", id, next, generation ?? undefined);
  };

  const pump = () => {
    if (pumpPromise.current) return pumpPromise.current;
    pumpPromise.current = (async () => {
      await beginPromise.current;
      while (drag.current?.latest != null) {
        const next = drag.current.latest;
        drag.current.latest = null;
        await command("setParameter", id, next, drag.current?.generation);
      }
    })().finally(() => { pumpPromise.current = null; });
    return pumpPromise.current;
  };

  const finish = async () => {
    if (!drag.current || finishing.current) return;
    finishing.current = true;
    try {
      await pump();
      await command("endGesture", id, undefined, drag.current.generation);
    } finally {
      drag.current = null;
      finishing.current = false;
    }
  };

  return (
    <div className={`knob-control ${size}`}>
      <div className="knob-scale">
        {Array.from({ length: 7 }).map((_, index) => (
          <span className="knob-tick" key={index} style={{ transform: `rotate(${-135 + index * 45}deg)` }} />
        ))}
        <div className="knob" role="slider" tabIndex={value === null ? -1 : 0}
          aria-label={label} aria-valuemin={0} aria-valuemax={1}
          aria-valuenow={value ?? undefined} aria-disabled={value === null}
          onPointerDown={event => {
            if (value === null || generation === null) return;
            event.currentTarget.setPointerCapture(event.pointerId);
            drag.current = { pointer: event.pointerId, startY: event.clientY,
              startValue: value, generation, latest: null };
            beginPromise.current = command("beginGesture", id, undefined, generation);
          }}
          onPointerMove={event => {
            if (!drag.current || drag.current.pointer !== event.pointerId) return;
            drag.current.latest = clamp(drag.current.startValue + (drag.current.startY - event.clientY) / 170);
            void pump();
          }}
          onPointerUp={() => { void finish(); }}
          onPointerCancel={() => { void finish(); }}
          onKeyDown={event => {
            if (value === null) return;
            const delta = event.key === "ArrowUp" || event.key === "ArrowRight" ? 0.01
              : event.key === "ArrowDown" || event.key === "ArrowLeft" ? -0.01
              : event.key === "PageUp" ? 0.1 : event.key === "PageDown" ? -0.1 : null;
            const next = event.key === "Home" ? 0 : event.key === "End" ? 1
              : delta === null ? null : clamp(value + delta);
            if (next === null) return;
            event.preventDefault();
            void command("setParameter", id, next);
          }}>
          <span className="knob-indicator" style={{ transform: `rotate(${angle}deg)` }} />
        </div>
      </div>
      <div className="knob-label">{label}</div>
      <input className="knob-value" type="number" min="0" max="1" step="0.001"
        aria-label={`${label} normalized value`} disabled={value === null}
        value={editing ? draft : value === null ? "" : value.toFixed(3)}
        onFocus={() => {
          setEditing(true); setDraft(value === null ? "" : value.toFixed(3));
          editGeneration.current = generation; cancelEdit.current = false;
        }}
        onChange={event => setDraft(event.target.value)}
        onBlur={commitDraft}
        onKeyDown={event => {
          if (event.key === "Enter") event.currentTarget.blur();
          if (event.key === "Escape") { cancelEdit.current = true; event.currentTarget.blur(); }
        }} />
    </div>
  );
}

function App() {
  const frame = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);
  const { state, error, command } = useHostParameters();
  const knob = (position: string) => controls.filter(control => control.position === position)
    .map(control => <Knob key={control.id} {...control}
      value={displayValue(state, control.id)} generation={state?.generation ?? null} command={command} />);

  useEffect(() => {
    if (!frame.current) return;
    const update = () => setScale(Math.min(1.12, frame.current!.clientWidth / 1120));
    update();
    const observer = new ResizeObserver(update);
    observer.observe(frame.current);
    return () => observer.disconnect();
  }, []);

  return <main className="stage">
    <div className="machine-frame" ref={frame} style={{ height: `${650 * scale}px` }}>
      <section className="machine" aria-label="Dandrum TB-303 bass synthesizer" style={{ transform: `scale(${scale})` }}>
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
            {controls.filter(control => control.position === "program").map(control =>
              <Knob key={control.id} {...control} size="small" value={displayValue(state, control.id)}
                generation={state?.generation ?? null} command={command} />)}
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
        <section className="keyboard-panel" aria-label="Note audition unavailable">
          <div className="keyboard-labels"><span>NOTE AUDITION UNAVAILABLE</span><span>HOST MIDI INPUT REMAINS ACTIVE</span></div>
          <div className="keyboard">
            <div className="white-keys" style={{ gridTemplateColumns: `repeat(${whiteKeyCount}, 1fr)` }}>
              {keyboardKeys.filter(key => key.kind === "white").map((key, index) =>
                <button key={index} className="key white" disabled><span className="key-label">{key.name}</span></button>)}
            </div>
            <div className="black-keys">{keyboardKeys.filter(key => key.kind === "black").map((key, index) =>
              <button key={index} className="key black" disabled
                style={{ left: `${(key.whiteIndex / whiteKeyCount) * 100}%`, width: `calc(100% / ${whiteKeyCount * 1.6})` }}>
                <span className="key-label">{key.name}</span></button>)}</div>
          </div>
        </section>
        <footer><div className="footer-centre"><span>DANDRUM</span><strong>TB-303</strong></div></footer>
      </section>
    </div>
    <p className="hint">Drag a knob or use arrow keys. Values follow the host.</p>
    {error && <p className="host-error" role="alert">{error}</p>}
  </main>;
}

createRoot(document.getElementById("root")!).render(<App />);
