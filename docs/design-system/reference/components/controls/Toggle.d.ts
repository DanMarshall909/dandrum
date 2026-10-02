/** Pill on/off toggle for binary settings. */
export interface ToggleProps { checked?: boolean; label?: string; disabled?: boolean; focused?: boolean; hostAutomated?: boolean; compact?: boolean; onChange?: (v: boolean) => void; }
export declare function Toggle(props: ToggleProps): JSX.Element;
/** Instrument-style latching key with LED bar. */
export interface SwitchProps { on?: boolean; label?: string; disabled?: boolean; focused?: boolean; compact?: boolean; width?: number; onChange?: (v: boolean) => void; }
export declare function Switch(props: SwitchProps): JSX.Element;
