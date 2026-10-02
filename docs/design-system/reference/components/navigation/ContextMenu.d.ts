export interface MenuItem {
  label?: string; icon?: string; shortcut?: string; disabled?: boolean; danger?: boolean; checked?: boolean; submenu?: boolean;
  header?: string; note?: string; separator?: boolean; onSelect?: () => void;
  /** 'assignment' renders source + depth slider + remove. */
  type?: 'assignment'; slot?: 'A'|'B'|'C'|'D'; source?: string; depth?: number; onDepth?: (d: number) => void; onRemove?: () => void;
}
/** Popup/context menu, 248px wide, 26px items. */
export interface ContextMenuProps { items?: MenuItem[]; title?: string; x?: number; y?: number; width?: number; onClose?: () => void; style?: React.CSSProperties; }
export declare function ContextMenu(props: ContextMenuProps): JSX.Element;
/** Closed dropdown trigger. */
export interface MenuButtonProps { label?: string; value?: string; onClick?: () => void; compact?: boolean; disabled?: boolean; width?: number; readOnly?: boolean; }
export declare function MenuButton(props: MenuButtonProps): JSX.Element;
