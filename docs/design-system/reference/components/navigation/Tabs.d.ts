export interface TabItem { id: string; label: string; icon?: string; badge?: string; disabled?: boolean; }
/** Panel/view tabs, 32px (26 compact). */
export interface TabsProps { items?: TabItem[]; value?: string; onChange?: (id: string) => void; compact?: boolean; style?: React.CSSProperties; }
export declare function Tabs(props: TabsProps): JSX.Element;
export interface SegmentOption { id: string; label: string; icon?: string; title?: string; }
/** 2–5 exclusive options in a recessed well, 24px (22 compact). */
export interface SegmentedControlProps { options?: (SegmentOption|string)[]; value?: string; onChange?: (id: string) => void; compact?: boolean; disabled?: boolean; hostAutomated?: boolean; fullWidth?: boolean; style?: React.CSSProperties; }
export declare function SegmentedControl(props: SegmentedControlProps): JSX.Element;
