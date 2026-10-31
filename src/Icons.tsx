const base = { width: 18, height: 18, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 1.8, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };

export const IconHome = () => (
  <svg {...base}>
    <path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z" />
  </svg>
);
export const IconList = () => (
  <svg {...base}>
    <path d="M8 6h13M8 12h13M8 18h13M3.5 6h.01M3.5 12h.01M3.5 18h.01" />
  </svg>
);
export const IconChart = () => (
  <svg {...base}>
    <path d="M4 20V10M10 20V4M16 20v-7M22 20H2" />
  </svg>
);
export const IconUpload = () => (
  <svg {...base}>
    <path d="M12 16V4M7 9l5-5 5 5M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3" />
  </svg>
);
export const IconDownload = () => (
  <svg {...base}>
    <path d="M12 4v12M7 11l5 5 5-5M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3" />
  </svg>
);
export const IconTrash = () => (
  <svg {...base} width={16} height={16}>
    <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
  </svg>
);
export const IconCheck = () => (
  <svg {...base} width={16} height={16}>
    <path d="m5 12 5 5 9-10" />
  </svg>
);
export const IconAlert = () => (
  <svg {...base} width={16} height={16}>
    <path d="M12 8v5M12 16.5h.01M10.3 3.9 2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
  </svg>
);
export const IconArrow = () => (
  <svg {...base} width={14} height={14}>
    <path d="M5 12h14M13 6l6 6-6 6" />
  </svg>
);
export const IconLogo = ({ size = 30 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 512 512" aria-hidden>
    <defs>
      <linearGradient id="km-bg" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stopColor="#14b8a6" />
        <stop offset="1" stopColor="#0f5f58" />
      </linearGradient>
    </defs>
    <rect width="512" height="512" rx="116" fill="url(#km-bg)" />
    <g fill="none" stroke="#fff" strokeLinecap="round" strokeLinejoin="round">
      <path d="M256 168v206" strokeWidth="30" />
      <path d="M186 390h140" strokeWidth="30" />
      <path d="M118 196h276" strokeWidth="28" />
      <path d="M148 204l-46 96M148 204l46 96M364 204l-46 96M364 204l46 96" strokeWidth="14" />
      <path d="M90 300a58 40 0 0 0 116 0z" fill="#fff" strokeWidth="12" />
      <path d="M306 300a58 40 0 0 0 116 0z" fill="#fff" strokeWidth="12" />
    </g>
    <path d="M226 132v-14a30 30 0 0 1 60 0v14" fill="none" stroke="#fff" strokeWidth="16" strokeLinecap="round" />
    <rect x="210" y="128" width="92" height="66" rx="16" fill="#fff" />
    <circle cx="256" cy="155" r="10" fill="#0f5f58" />
    <path d="M251 160h10l3 18h-16z" fill="#0f5f58" />
  </svg>
);
export const IconDoc = () => (
  <svg {...base}>
    <path d="M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8zM14 3v5h5M9 13h6M9 17h6" />
  </svg>
);
export const IconData = () => (
  <svg {...base}>
    <ellipse cx="12" cy="5.5" rx="8" ry="3" />
    <path d="M4 5.5v6.5c0 1.7 3.6 3 8 3s8-1.3 8-3V5.5M4 12v6.5c0 1.7 3.6 3 8 3s8-1.3 8-3V12" />
  </svg>
);
export const IconBell = () => (
  <svg {...base}>
    <path d="M6 9a6 6 0 1 1 12 0c0 5 2 6.5 2 6.5H4S6 14 6 9M10 19.5a2 2 0 0 0 4 0" />
  </svg>
);
