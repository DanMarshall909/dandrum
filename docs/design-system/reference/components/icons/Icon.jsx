import React from 'react';

const ICON_PATHS = {
 "reload": "<path d=\"M19 12a7 7 0 1 1-2.05-4.95\"/><path d=\"M19 4v4.5h-4.5\"/>",
 "warning": "<path d=\"M12 4 21 19.5H3Z\"/><path d=\"M12 10v4.5\"/><path d=\"M12 17.2v.3\"/>",
 "error": "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"m9 9 6 6M15 9l-6 6\"/>",
 "ok": "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"m8.5 12.2 2.4 2.4 4.6-4.8\"/>",
 "info": "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"M12 11v5\"/><path d=\"M12 8v.3\"/>",
 "lock": "<rect x=\"5.5\" y=\"10.5\" width=\"13\" height=\"9\" rx=\"1.5\"/><path d=\"M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5\"/>",
 "chevron-down": "<path d=\"m7 10 5 5 5-5\"/>",
 "chevron-right": "<path d=\"m10 7 5 5-5 5\"/>",
 "chevron-left": "<path d=\"m14 7-5 5 5 5\"/>",
 "close": "<path d=\"m7 7 10 10M17 7 7 17\"/>",
 "plus": "<path d=\"M12 6v12M6 12h12\"/>",
 "minus": "<path d=\"M6 12h12\"/>",
 "more": "<path d=\"M6 12h.01M12 12h.01M18 12h.01\" stroke-width=\"2.5\"/>",
 "modulate": "<path d=\"M3 12c2.2-5 4.4-5 6.6 0s4.4 5 6.6 0c1.2-2.7 2.4-3.9 3.8-3.6\"/>",
 "host": "<path d=\"M9 4v5M15 4v5\"/><path d=\"M6.5 9h11v3a5.5 5.5 0 0 1-11 0Z\"/><path d=\"M12 17.5V21\"/>",
 "choke": "<path d=\"M4 8h6l2 4 2-4h6\"/><path d=\"M4 16h16\"/>",
 "alternate": "<path d=\"M5 9h11.5\"/><path d=\"m14 6 3 3-3 3\"/><path d=\"M19 15H7.5\"/><path d=\"m10 12-3 3 3 3\"/>",
 "layers": "<path d=\"M5 16h14M7 12h10M9 8h6\"/>",
 "reverse": "<path d=\"M18 7v10\"/><path d=\"M15 12H5\"/><path d=\"m8.5 8.5-3.5 3.5 3.5 3.5\"/>",
 "loop": "<path d=\"M7 9h9a3 3 0 0 1 0 6h-1.5\"/><path d=\"M16.5 15H8a3 3 0 0 1-3-3\"/><path d=\"m10 6-3 3 3 3\"/>",
 "one-shot": "<path d=\"M4 18V6\"/><path d=\"M4 7c5 0 7 3 9 7 1.2 2.4 3 4 7 4\"/>",
 "gate": "<path d=\"M3 18h3V7h12v11h3\"/>",
 "slice": "<path d=\"M6 4v16M12 4v16M18 4v16\" stroke-dasharray=\"2 2.5\"/>",
 "keyboard": "<rect x=\"3.5\" y=\"6\" width=\"17\" height=\"12\" rx=\"1.5\"/><path d=\"M8 6v7M12 6v7M16 6v7\"/>",
 "folder": "<path d=\"M3.5 7.5a1.5 1.5 0 0 1 1.5-1.5h4l2 2h8a1.5 1.5 0 0 1 1.5 1.5V17a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 17Z\"/>",
 "file-missing": "<path d=\"M14 3.5H7A1.5 1.5 0 0 0 5.5 5v14A1.5 1.5 0 0 0 7 20.5h10a1.5 1.5 0 0 0 1.5-1.5V8Z\"/><path d=\"M14 3.5V8h4.5\"/><path d=\"m10 12 4 4M14 12l-4 4\"/>",
 "reset": "<path d=\"M5 12a7 7 0 1 0 2.05-4.95\"/><path d=\"M5 4v4.5h4.5\"/>",
 "midi": "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"M8.5 12h.01M15.5 12h.01M10 9h.01M14 9h.01M12 8.2h.01\" stroke-width=\"2\"/>",
 "pan": "<path d=\"M3 12h18\"/><path d=\"m6 9-3 3 3 3M18 9l3 3-3 3\"/><path d=\"M12 8v8\"/>",
 "pitch": "<path d=\"M12 20V5\"/><path d=\"m8 9 4-4 4 4\"/><path d=\"M8 20h8\"/>",
 "level": "<path d=\"M5 19 19 5v14Z\"/>",
 "variation": "<rect x=\"4\" y=\"4\" width=\"16\" height=\"16\" rx=\"2\"/><path d=\"M9 9h.01M15 15h.01M15 9h.01M9 15h.01\" stroke-width=\"2.5\"/>",
 "settings": "<path d=\"M5 7h9M18 7h1M5 17h3M12 17h7\"/><circle cx=\"16\" cy=\"7\" r=\"2\"/><circle cx=\"10\" cy=\"17\" r=\"2\"/>"
};

export const ICON_NAMES = Object.keys(ICON_PATHS);

export function Icon({ name, size = 16, color = 'currentColor', strokeWidth = 1.5, style, title }) {
  const inner = ICON_PATHS[name];
  if (!inner) return null;
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={strokeWidth}
      strokeLinecap="round" strokeLinejoin="round" style={{ display: 'block', flex: 'none', ...style }}
      role={title ? 'img' : undefined} aria-hidden={title ? undefined : true} aria-label={title}
      dangerouslySetInnerHTML={{ __html: inner }} />
  );
}
