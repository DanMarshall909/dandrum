/** Source + depth chip for tooltips, menus and detail panels. */
export interface ModIndicatorProps { slot?: 'A'|'B'|'C'|'D'; depth?: number; source?: string; /** DAW-owned automation (blue, plug glyph). */ host?: boolean; active?: boolean; compact?: boolean; }
export declare function ModIndicator(props: ModIndicatorProps): JSX.Element;
export interface ModGlyphProps { slot?: 'A'|'B'|'C'|'D'; size?: number; color?: string; hollow?: boolean; }
/** A ● B ▲ C ■ D ◆ — shape pairs with colour. */
export declare function ModGlyph(props: ModGlyphProps): JSX.Element;
export declare const MOD_SLOTS: Record<'A'|'B'|'C'|'D', { color: string; glyph: string; name: string }>;
