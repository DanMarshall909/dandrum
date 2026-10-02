/** Peak meter. */
export interface MeterProps { /** dBFS per channel. */ levels?: number[]; peak?: number | number[]; clip?: boolean; orientation?: 'vertical'|'horizontal'; length?: number; thickness?: number; compact?: boolean; label?: string; onResetClip?: () => void; }
export declare function Meter(props: MeterProps): JSX.Element;
