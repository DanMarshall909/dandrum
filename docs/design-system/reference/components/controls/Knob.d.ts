export interface ModAssignment { /** Slot letter A–D (colour + glyph). */ slot: 'A'|'B'|'C'|'D'; /** −1…1 normalised range from the base value. */ depth: number; /** Source display name. */ source?: string; /** 0–1 current source output when actively modulating; omit when idle. */ live?: number; }
/**
 * Dandrum rotary knob — the system's signature control.
 * @startingPoint section="Controls" subtitle="Rotary knob with modulation rings" viewport="700x420"
 */
export interface KnobProps {
  /** Normalised 0–1 (JUCE: RangedAudioParameter::getValue). */
  value?: number;
  defaultValue?: number;
  /** Arc grows from 12 o'clock (pan, pitch, fine tune). */
  bipolar?: boolean;
  /** lg 64 · md 48 · sm 36 · xs 28, or a px number. */
  size?: 'lg'|'md'|'sm'|'xs'|number;
  label?: string;
  /** Pre-formatted value text (e.g. "1.000×", "L12"). */
  valueText?: string;
  unit?: string;
  /** Up to 2 rings drawn; more shows a +N chip. */
  modulations?: ModAssignment[];
  /** Eligible destination while an assignment is in progress. */
  assigning?: boolean;
  /** DAW is writing the parameter (decays ~400ms after last host change). */
  hostAutomated?: boolean;
  focused?: boolean;
  disabled?: boolean;
  hovered?: boolean;
  /** Force the thick value arc (default: dragging, keyboard/wheel nudge for 600 ms, or hostAutomated). */
  active?: boolean;
  /** Slot whose depth is being edited (e.g. from the context menu) — only that ring thickens. */
  activeSlot?: 'A'|'B'|'C'|'D';
  /** 0–1 highest / lowest effective (modulated) value since reset. Omit to let the knob track them from live modulation. */
  peak?: number; trough?: number;
  /** Pre-formatted peak/trough text (e.g. "1.122×"). */
  peakText?: string; troughText?: string;
  /** Ticks on the inner arc + "▴ peak ▾ trough" read-out under the value (md/lg). Default true. */
  showPeakTrough?: boolean;
  onResetPeaks?: () => void;
  /** 'hover' (default): value, unit, peak/trough and modulation list live in an editable popup shown on hover, focus, drag or nudge; the panel shows only the label + CLIP tag. 'always': inline read-out. */
  valueDisplay?: 'hover'|'always';
  /** Typed text → normalised 0–1 (default: number ÷ 100). */
  parseValue?: (text: string) => number | null;
  /** Force the popup open (documentation / screenshots). */
  popupOpen?: boolean;
  showLabel?: boolean;
  labelPosition?: 'top'|'bottom';
  onChange?: (v: number) => void;
  /** Right-click, Shift+F10, Menu key or M. */
  onContextMenu?: (e: any) => void;
  onFocus?: (e: any) => void;
  style?: React.CSSProperties;
}
export declare function Knob(props: KnobProps): JSX.Element;
