/** List row 26px (22 compact). */
export interface ListRowProps { label?: React.ReactNode; detail?: React.ReactNode; value?: React.ReactNode; icon?: string; leading?: React.ReactNode; trailing?: React.ReactNode; selected?: boolean; active?: boolean; disabled?: boolean; compact?: boolean; onClick?: () => void; }
export declare function ListRow(props: ListRowProps): JSX.Element;
/** Label/value row; prepared = read-only patch data. */
export interface PropertyRowProps { label?: string; value?: React.ReactNode; unit?: string; prepared?: boolean; compact?: boolean; hint?: string; }
export declare function PropertyRow(props: PropertyRowProps): JSX.Element;
/** Stacked parameter label + value. */
export interface ParamLabelProps { label?: string; value?: React.ReactNode; unit?: string; align?: 'left'|'center'|'right'; hostAutomated?: boolean; size?: 'sm'|'md'|'lg'; }
export declare function ParamLabel(props: ParamLabelProps): JSX.Element;
