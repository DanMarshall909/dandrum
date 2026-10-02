/** Status banner or inline status-bar message. */
export interface StatusMessageProps { kind?: 'ok'|'warn'|'error'|'info'|'busy'; title?: React.ReactNode; children?: React.ReactNode; action?: React.ReactNode; inline?: boolean; compact?: boolean; }
export declare function StatusMessage(props: StatusMessageProps): JSX.Element;
/** Tooltip bubble. */
export interface TooltipProps { title?: string; value?: string; children?: React.ReactNode; placement?: 'top'|'bottom'|'left'|'right'; style?: React.CSSProperties; }
export declare function Tooltip(props: TooltipProps): JSX.Element;
/** Empty state. */
export interface EmptyStateProps { icon?: string; title?: string; children?: React.ReactNode; action?: React.ReactNode; compact?: boolean; }
export declare function EmptyState(props: EmptyStateProps): JSX.Element;
