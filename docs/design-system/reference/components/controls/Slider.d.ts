export interface ModAssignment { /** Slot letter A–D (colour + glyph). */ slot: 'A'|'B'|'C'|'D'; /** −1…1 normalised range from the base value. */ depth: number; /** Source display name. */ source?: string; /** 0–1 current source output when actively modulating; omit when idle. */ live?: number; }
/** Linear slider, horizontal (160×24) or vertical (24×120). */
export interface SliderProps {
  value?: number; defaultValue?: number;
  orientation?: 'horizontal'|'vertical';
  /** Track length px. */
  length?: number;
  bipolar?: boolean; label?: string; valueText?: string;
  modulations?: ModAssignment[];
  assigning?: boolean; hostAutomated?: boolean; focused?: boolean; disabled?: boolean; compact?: boolean;
  onChange?: (v: number) => void; onContextMenu?: (e: any) => void;
  style?: React.CSSProperties;
}
export declare function Slider(props: SliderProps): JSX.Element;
