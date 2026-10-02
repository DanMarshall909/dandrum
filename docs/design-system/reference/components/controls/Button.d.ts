/** Text button. */
export interface ButtonProps {
  children?: React.ReactNode;
  /** primary = vermilion (max one per view, e.g. Reload Patch), secondary = graphite, ghost = text-only. */
  variant?: 'primary'|'secondary'|'ghost';
  size?: 'md'|'sm';
  icon?: string; selected?: boolean; disabled?: boolean; focused?: boolean; pressed?: boolean;
  onClick?: () => void; title?: string; style?: React.CSSProperties;
}
export declare function Button(props: ButtonProps): JSX.Element;
/** Square icon button 28px (24 compact). */
export interface IconButtonProps {
  icon: string; /** Required — tooltip + accessible name. */ label: string;
  size?: 'md'|'sm'; variant?: 'secondary'|'ghost'; selected?: boolean; disabled?: boolean; focused?: boolean;
  onClick?: () => void; style?: React.CSSProperties;
}
export declare function IconButton(props: IconButtonProps): JSX.Element;
