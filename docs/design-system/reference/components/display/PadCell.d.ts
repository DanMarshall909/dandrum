/**
 * Playable pad cell, 76px (52 compact, 44 min).
 * @startingPoint section="Sampler" subtitle="Playable pad with velocity, layers, alternates, choke" viewport="700x300"
 */
export interface PadCellProps {
  note?: number; name?: string; mapped?: boolean; selected?: boolean;
  /** 0–1 live activity brightness; the editor decays it per frame. */
  level?: number;
  /** 0–1 last-hit velocity. */
  velocity?: number;
  alternates?: number; activeAlternate?: number; layers?: number; activeLayer?: number;
  chokeGroup?: number; choked?: boolean; missing?: boolean; focused?: boolean; disabled?: boolean; compact?: boolean; size?: number;
  onTrigger?: (note: number, velocity: number) => void; onSelect?: (note: number) => void; onRelease?: (note: number) => void;
  style?: React.CSSProperties;
}
export declare function PadCell(props: PadCellProps): JSX.Element;
export declare function noteName(n: number): string;
