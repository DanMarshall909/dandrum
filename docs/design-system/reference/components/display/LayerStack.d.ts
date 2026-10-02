export interface ModuleParam {
  id: string; label: string;
  /** Real value (Hz, dB, %, ms…) or the selected option. */
  value: number | string;
  min?: number; max?: number; default?: number;
  unit?: 'Hz' | 'dB' | '%' | 'ms' | string;
  /** Log taper (frequencies). */
  log?: boolean; bipolar?: boolean;
  /** Discrete choice rendered as a segmented control. */
  options?: string[];
  /** Include in the one-line summary on the module block. */
  summary?: boolean;
}
export interface LayerModule {
  id: string;
  /** Module type shown as the caps label: "Filter", "Gain", "Saturate"… */
  type: string;
  /** Fallback summary when no param has summary: true. */
  value?: string;
  /** Shown as knobs / segmented controls in the module editor. */
  params?: ModuleParam[];
  bypassed?: boolean;
}
export interface Layer {
  id: string; name: string;
  /** Source type tag: "Synth", "Sample", "Patch"… */
  source?: string;
  /** One mono line: engine settings or sample filename. */
  detail?: string;
  /** Source parameters shown in the source editor (synth engine or sample playback). */
  params?: ModuleParam[];
  /** Sample sources: WaveformPanel props for the source editor. A param with id "start" (0–100 %) drives startOffset. */
  waveform?: Record<string, any>;
  /** Send level in dB per FX bus id; null/missing = off (−∞). */
  sends?: Record<string, number | null>;
  /** Output bus id (from LayerStackProps.outputs). */
  output?: string;
  /** Inline processing chain, left to right. */
  modules?: LayerModule[];
  /** Layer output level in dB. */
  level?: number;
  muted?: boolean;
}
/**
 * Layers that all trigger together for one zone or pad. Each row: source → inline module chain → level → mute.
 * Click the source block to open its editor (sample: waveform + params; synth: engine params).
 * Click a module to open its editor below the row (knobs + choices); click again, × or Esc closes.
 * Module dot = bypass, × on hover = remove, dashed + slot = add.
 */
export interface LayerStackProps {
  layers?: Layer[]; selectedId?: string;
  onSelect?: (id: string) => void;
  onChange?: (layers: Layer[], changedId: string) => void;
  /** Called by the dashed + slot; open a module picker. */
  onAddModule?: (layerId: string) => void;
  /** FX busses: one send knob per bus on every row. */
  fxBusses?: { id: string; name: string; letter?: string }[];
  /** Output busses offered in each row's output menu. */
  outputs?: { id: string; name: string; /** Shown right-aligned in the menu, e.g. "3/4". */ note?: string }[];
  /** Override the header count text. */
  countLabel?: string;
  title?: string; subtitle?: string; editable?: boolean;
  style?: React.CSSProperties;
}
export declare function LayerStack(props: LayerStackProps): JSX.Element;
