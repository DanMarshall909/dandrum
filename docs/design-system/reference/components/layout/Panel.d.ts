/**
 * Functional panel with caps header.
 * @startingPoint section="Layout" subtitle="Instrument panel with header, tag and actions" viewport="700x260"
 */
export interface PanelProps {
  title?: React.ReactNode; tag?: string; prepared?: boolean; actions?: React.ReactNode; children?: React.ReactNode; compact?: boolean;
  padding?: number|string; style?: React.CSSProperties; bodyStyle?: React.CSSProperties; tone?: 'default'|'sunken';
  /** Header click / Enter / Space collapses to the header. Default true. */
  collapsible?: boolean; collapsed?: boolean; defaultCollapsed?: boolean; onToggle?: (collapsed: boolean) => void;
  /** One-line mono summary shown in the header while collapsed. */
  summary?: string;
}
export declare function Panel(props: PanelProps): JSX.Element;
/** Collapsible region inside a panel (3ds Max-style rollout). */
export interface RolloutProps { title?: React.ReactNode; summary?: string; children?: React.ReactNode; collapsed?: boolean; defaultCollapsed?: boolean; onToggle?: (collapsed: boolean) => void; prepared?: boolean; compact?: boolean; actions?: React.ReactNode; }
export declare function Rollout(props: RolloutProps): JSX.Element;
/** Caps section heading. */
export interface SectionHeadingProps { children?: React.ReactNode; compact?: boolean; prepared?: boolean; color?: string; }
export declare function SectionHeading(props: SectionHeadingProps): JSX.Element;
