Peak meter for output and per-pad level; 6×96 stereo vertical default.

```jsx
<Meter levels={[-9, -11]} peak={[-4, -6]} label="OUT" />
<Meter orientation="horizontal" levels={[-24]} length={80} thickness={4} />
```

- Scale −60…+3 dBFS. Green below −12, amber −12…0, red above 0; clip LED latches until clicked.
