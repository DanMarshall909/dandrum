export type IconName = 'reload' | 'warning' | 'error' | 'ok' | 'info' | 'lock' | 'chevron-down' | 'chevron-right' | 'chevron-left' | 'close' | 'plus' | 'minus' | 'more' | 'modulate' | 'host' | 'choke' | 'alternate' | 'layers' | 'reverse' | 'loop' | 'one-shot' | 'gate' | 'slice' | 'keyboard' | 'folder' | 'file-missing' | 'reset' | 'midi' | 'pan' | 'pitch' | 'level' | 'variation' | 'settings';
export interface IconProps {
  /** Icon from the dd-icon set (assets/icons/dd-icon-<name>.svg). */
  name: IconName;
  /** Rendered px size. 16 default, 14 compact. */
  size?: number;
  color?: string;
  strokeWidth?: number;
  style?: React.CSSProperties;
  /** Accessible label; omit for decorative icons. */
  title?: string;
}
export declare function Icon(props: IconProps): JSX.Element;
export declare const ICON_NAMES: IconName[];
