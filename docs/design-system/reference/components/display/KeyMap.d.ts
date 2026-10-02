export interface KeyMapZone {
  id: string;
  /** Target name: a sample region ("C2 soft") or a Dandrum patch ("TB-303 Bass"). */
  name: string;
  /** What the zone triggers, shown as a tag: "Patch", "Sample", "Kit"… Any compatible Dandrum patch can be a target. */
  source?: string;
  /** MIDI key range, inclusive. */
  lo: number; hi: number;
  /** Velocity range 1–127, inclusive. */
  velLo: number; velHi: number;
  /** Root key for pitched playback. */
  root?: number;
}
/**
 * Key × velocity map with an aligned, zoomable piano keyboard. Splits and layers samples or whole patches.
 * Drag zones to move, edges/corners to resize; arrows nudge, Shift+arrows resize, Alt steps by 8.
 * Zoom with − / + / Fit or Ctrl/Cmd + wheel; scrolls horizontally when zoomed.
 */
export interface KeyMapProps {
  zones?: KeyMapZone[]; selectedId?: string;
  onSelect?: (id: string) => void;
  onChange?: (zones: KeyMapZone[], changedId: string) => void;
  onNoteOn?: (note: number, velocity: number) => void; onNoteOff?: (note: number) => void;
  /** Visible key range. Default 24–96 (C1–C6). */
  lowNote?: number; highNote?: number;
  /** Px per semitone at zoom 1. Omit to fit the available width. */
  keyWidth?: number; gridHeight?: number; keyboardHeight?: number;
  /** Controlled zoom (1 = fit). Omit for internal state. */
  zoom?: number; onZoomChange?: (zoom: number) => void; minZoom?: number; maxZoom?: number;
  /** false = prepared/read-only: dashed zones, selection and playing only. */
  editable?: boolean; title?: string;
  style?: React.CSSProperties;
}
export declare function KeyMap(props: KeyMapProps): JSX.Element;
