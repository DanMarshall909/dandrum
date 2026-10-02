/**
 * Waveform view for a prepared sample region (not an editor).
 * @startingPoint section="Sampler" subtitle="Region waveform with fades, loop, slices, cursor" viewport="700x340"
 */
export interface WaveformPanelProps {
  kind?: 'kick'|'snare'|'snare-soft'|'hat-closed'|'hat-open'|'break'; peaks?: number[]; height?: number;
  regionStart?: number; regionEnd?: number; /** fraction of region */ fadeIn?: number; fadeOut?: number;
  loopStart?: number; loopEnd?: number; crossfade?: number;
  /** Slice start positions (0–1), or { pos, name } — names come from the patch (prepared data). */
  slices?: (number | { pos: number; name?: string })[]; selectedSlice?: number;
  /** 0–1 playback position (live). */ cursor?: number | null;
  /** 0–1 live Start Offset within the region. */ startOffset?: number;
  startModulation?: { depth: number; color?: string };
  /** 'wave' = min/max amplitude; 'spectral' = log-frequency spectrogram. Controlled when set. */
  display?: 'wave'|'spectral'; defaultDisplay?: 'wave'|'spectral'; onDisplayChange?: (d: 'wave'|'spectral') => void;
  /** Small Wave/Spectral toggle at the bottom-right of the well. Default true. */
  showDisplayToggle?: boolean;
  reversed?: boolean; hostAutomated?: boolean; compact?: boolean; missing?: boolean; label?: string; style?: React.CSSProperties;
}
export declare function WaveformPanel(props: WaveformPanelProps): JSX.Element;
export declare function makePeaks(kind?: string, n?: number, seed?: number): number[];
