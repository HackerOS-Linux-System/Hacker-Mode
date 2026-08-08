import { Component, JSX } from "solid-js";

/**
 * Minimalne, jednokolorowe ikony SVG (rysowane `currentColor`), zastępujące
 * dawne emotki w interfejsie. Brak zależności od zewnętrznej biblioteki ikon
 * (frontend nie ma żadnej w package.json) - to najlżejsza opcja, spójna z
 * resztą stosu (Solid.js, bez dodatkowych paczek).
 */

type IconProps = { size?: number; style?: JSX.CSSProperties };

const base = (size = 18) => ({
  width: size,
  height: size,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  "stroke-width": "2",
  "stroke-linecap": "round" as const,
  "stroke-linejoin": "round" as const,
});

export const IconHome: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <path d="M3 11.5 12 4l9 7.5" />
    <path d="M5 10v9.5a1 1 0 0 0 1 1h4v-6h4v6h4a1 1 0 0 0 1-1V10" />
  </svg>
);

export const IconLibrary: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <rect x="3" y="7" width="18" height="12" rx="2" />
    <circle cx="8.5" cy="13" r="1.4" />
    <circle cx="15.5" cy="13" r="1.4" />
    <path d="M7 7V6a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v1" />
  </svg>
);

export const IconStore: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <path d="M4 8 5.5 4h13L20 8" />
    <path d="M4 8v11a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1V8" />
    <path d="M4 8h16" />
    <path d="M9 20v-5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v5" />
  </svg>
);

export const IconSettings: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 13.5a7.6 7.6 0 0 0 0-3l2-1.4-2-3.4-2.3.8a7.6 7.6 0 0 0-2.6-1.5L14 2h-4l-.5 2.5a7.6 7.6 0 0 0-2.6 1.5l-2.3-.8-2 3.4 2 1.4a7.6 7.6 0 0 0 0 3l-2 1.4 2 3.4 2.3-.8a7.6 7.6 0 0 0 2.6 1.5L10 22h4l.5-2.5a7.6 7.6 0 0 0 2.6-1.5l2.3.8 2-3.4-2-1.4Z" />
  </svg>
);

export const IconPower: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <path d="M12 3v8" />
    <path d="M6.5 6.5a8 8 0 1 0 11 0" />
  </svg>
);

export const IconRefresh: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <path d="M20 11a8 8 0 0 0-14.6-4.6M4 4v5h5" />
    <path d="M4 13a8 8 0 0 0 14.6 4.6M20 20v-5h-5" />
  </svg>
);

export const IconMonitor: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <rect x="3" y="4" width="18" height="12" rx="1.5" />
    <path d="M8 20h8M12 16v4" />
  </svg>
);

export const IconMoon: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5Z" />
  </svg>
);

export const IconDownload: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <path d="M12 3v12" />
    <path d="m7 10 5 5 5-5" />
    <path d="M5 21h14" />
  </svg>
);

export const IconPlay: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style} fill="currentColor" stroke="none">
    <path d="M8 5.5v13l11-6.5-11-6.5Z" />
  </svg>
);

export const IconTrash: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <path d="M4 7h16" />
    <path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
    <path d="M6 7l1 13a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1l1-13" />
    <path d="M10 11v6M14 11v6" />
  </svg>
);

export const IconBack: Component<IconProps> = (p) => (
  <svg {...base(p.size)} style={p.style}>
    <path d="m11 5-7 7 7 7" />
    <path d="M4 12h16" />
  </svg>
);
