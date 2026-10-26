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
export const IconLogo = () => (
  <svg width={28} height={28} viewBox="0 0 28 28" aria-hidden>
    <rect width="28" height="28" rx="8" fill="var(--accent)" />
    <path d="M8 19V13M12.5 19V9M17 19v-4" stroke="#fff" strokeWidth="2.4" strokeLinecap="round" />
  </svg>
);
export const IconDoc = () => (
  <svg {...base}>
    <path d="M14 3H6a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8zM14 3v5h5M9 13h6M9 17h6" />
  </svg>
);
