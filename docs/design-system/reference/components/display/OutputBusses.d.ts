export interface OutputBus {
  id: string; name: string;
  /** Plugin output channel pair, e.g. "1/2", "3/4". */
  channels: string;
  /** The default output (plugin Main 1/2). */
  main?: boolean;
  /** Names of layers / FX busses routed here (computed by the host editor). */
  feeds?: string[];
  /** Live dBFS per channel for the meter. */
  levels?: number[];
  level?: number; muted?: boolean;
}
/**
 * Output busses mapped to multi-output plugin channel pairs. Layers and FX busses choose one as their output.
 */
export interface OutputBussesProps {
  busses?: OutputBus[];
  /** Available plugin output pairs. Default 1/2 … 15/16. */
  channelOptions?: string[];
  onChange?: (busses: OutputBus[], changedId: string) => void;
  title?: string;
  style?: React.CSSProperties;
}
export declare function OutputBusses(props: OutputBussesProps): JSX.Element;
