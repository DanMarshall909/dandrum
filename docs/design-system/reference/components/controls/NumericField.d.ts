/** Numeric entry: scrub, type, step. */
export interface NumericFieldProps {
  value?: number; min?: number; max?: number; step?: number; defaultValue?: number;
  unit?: string; label?: string; format?: (v: number) => string; width?: number; compact?: boolean;
  /** Prepared value — dashed outline, no interaction. */
  readOnly?: boolean;
  disabled?: boolean; focused?: boolean; hostAutomated?: boolean;
  onChange?: (v: number) => void; onContextMenu?: (e: any) => void;
}
export declare function NumericField(props: NumericFieldProps): JSX.Element;
