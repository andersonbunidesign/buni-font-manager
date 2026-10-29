// Small stroke icons and the logo, drawn inline so they follow the theme colour.

type P = { size?: number };

const S = ({ size = 18, children, fill = "none", sw = 1.8 }: P & { children: React.ReactNode; fill?: string; sw?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill={fill} stroke="currentColor" strokeWidth={sw} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {children}
  </svg>
);

export const IconGrid = (p: P) => (
  <S {...p}>
    <rect x="4" y="4" width="7" height="7" rx="1" />
    <rect x="13" y="4" width="7" height="7" rx="1" />
    <rect x="4" y="13" width="7" height="7" rx="1" />
    <rect x="13" y="13" width="7" height="7" rx="1" />
  </S>
);
export const IconList = (p: P) => (
  <S {...p}>
    <path d="M4 6h16M4 12h16M4 18h16" />
  </S>
);
export const IconUpload = (p: P) => (
  <S {...p} sw={2}>
    <path d="M12 16V4M6 10l6-6 6 6M4 20h16" />
  </S>
);
export const IconFocus = (p: P) => (
  <S {...p}>
    <path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5" />
    <rect x="9" y="9" width="6" height="6" rx="1" />
  </S>
);
export const IconSun = (p: P) => (
  <S {...p}>
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
  </S>
);
export const IconMoon = (p: P) => (
  <S {...p}>
    <path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z" />
  </S>
);
export const IconGear = (p: P) => (
  <S {...p}>
    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
    <circle cx="12" cy="12" r="3" />
  </S>
);
export const IconSearch = (p: P) => (
  <S {...p} sw={2}>
    <circle cx="11" cy="11" r="6.5" />
    <path d="M16 16l4.5 4.5" />
  </S>
);
export const IconPlus = (p: P) => (
  <S {...p} sw={2}>
    <path d="M12 5v14M5 12h14" />
  </S>
);
export const IconDots = ({ size = 16 }: P) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
    <circle cx="6" cy="12" r="1.6" />
    <circle cx="12" cy="12" r="1.6" />
    <circle cx="18" cy="12" r="1.6" />
  </svg>
);
export const IconTrash = ({ size = 16 }: P) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="#FF4242" strokeWidth="2" aria-hidden="true">
    <path d="M5 7h14M10 11v6M14 11v6M7 7l1 13h8l1-13M9 7V4h6v3" />
  </svg>
);
export const IconMinusCircle = ({ size = 16 }: P) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="#FF4242" strokeWidth="2" aria-hidden="true">
    <path d="M7 12h10" />
    <circle cx="12" cy="12" r="9" />
  </svg>
);
export const IconFolder = (p: P) => (
  <S {...p}>
    <path d="M3 6.5A1.5 1.5 0 0 1 4.5 5h4.6l2 2.2h8.4A1.5 1.5 0 0 1 21 8.7v9.8a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z" />
  </S>
);
export const IconArrowIn = (p: P) => (
  <S {...p}>
    <path d="M4 12h10M10 8l4 4-4 4M16 4h4v16h-4" />
  </S>
);
export const IconCheckCircle = (p: P) => (
  <S {...p}>
    <circle cx="12" cy="12" r="9" />
    <path d="M8 12.3l2.8 2.8L16.2 9.6" />
  </S>
);

export const IconAlignLeft = (p: P) => (
  <S {...p}>
    <path d="M4 6h16M4 12h10M4 18h13" />
  </S>
);
export const IconAlignCenter = (p: P) => (
  <S {...p}>
    <path d="M4 6h16M7 12h10M5.5 18h13" />
  </S>
);
export const IconClock = (p: P) => (
  <S {...p}>
    <circle cx="12" cy="12" r="8" />
    <path d="M12 8v4l3 2" />
  </S>
);
export const IconX = (p: P) => (
  <S {...p}>
    <path d="M7 7l10 10M17 7L7 17" />
  </S>
);

export const IconSwap = (p: P) => (
  <S {...p}>
    <path d="M7 7h11l-3-3M17 17H6l3 3" />
  </S>
);
export const IconReset = (p: P) => (
  <S {...p}>
    <path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3L4.5 9" />
    <path d="M4.5 4.5V9H9" />
  </S>
);
export const IconWidth = (p: P) => (
  <S {...p}>
    <path d="M4 5v14M20 5v14M7 12h10M10 9l-3 3 3 3M14 9l3 3-3 3" />
  </S>
);
export const IconCopy = (p: P) => (
  <S {...p}>
    <rect x="8" y="8" width="12" height="12" rx="2" />
    <path d="M16 8V5.5A1.5 1.5 0 0 0 14.5 4h-9A1.5 1.5 0 0 0 4 5.5v9A1.5 1.5 0 0 0 5.5 16H8" />
  </S>
);
export const IconArtwork = (p: P) => (
  <S {...p}>
    <rect x="5" y="3" width="14" height="18" rx="1.5" />
    <path d="M8 8h8M8 12h8M8 16h5" />
  </S>
);

export function Star({ on, size = 20 }: { on: boolean; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill={on ? "#FFBF00" : "none"} stroke={on ? "#C99700" : "var(--star-off)"} strokeWidth="1.8" strokeLinejoin="round" aria-hidden="true">
      <path d="M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z" />
    </svg>
  );
}

export const LOGO_PATH =
  "M48.41,64.99c1.26-1.37,2.4-2.63,3.55-3.66l-1.83-5.15c-13.05-36.17,8.13-47.28,23.01-46.82,6.07.23,11.22,3.43,19.46,3.43,6.3,0,8.13-5.27,8.13-5.27l.8,2.29s-.57,5.38-6.75,6.53c-7.1,1.37-13.28-3.09-17.74-3.89-14.19-2.98-36.97,7.44-24.38,43.5.34,1.26.8,2.4,1.15,3.55,1.14-1.03,2.17-1.72,2.86-2.17,1.26-14.77,8.59-24.61,21.98-34.68,0,0,8.81,14.65,20.03,17.97,0,0-14.31,9.04-21.75,19.92,0,0,5.15-.8,13.74-.8,14.08,0,24.15,5.84,24.15,17.4,0,19.23-41.44,47.85-41.44,47.85,0,0-2.4-16.71-17.4-22.55-2.86,13.05-12.36,21.86-23.81,21.86-10.65,0-14.19-4.58-14.19-10.99,0-9.27,8.93-20.15,30.33-20.83-1.6-5.38-4.35-11.45-8.47-18.2-14.08-22.55-16.94-34-4.69-44.41,4.35-3.66,8.13-4.58,8.13-4.58l.92,1.49c-10.42,6.07-12.82,14.42,2.75,38,5.49,8.47,8.47,16.71,9.5,24.27.34-5.15.46-10.19-1.95-19.23-3.09-2.52-6.07-4.81-6.07-4.81ZM44.63,100.48c-15.57,0-24.04,5.04-24.04,12.93,0,5.04,3.89,8.7,11.68,8.7,10.87,0,18.66-7.67,17.63-21.29-1.72-.23-3.43-.34-5.27-.34ZM82.52,114.33s22.09-16.14,22.09-29.88c0-12.02-5.84-15-18.32-15-11.45,0-16.37,1.26-16.37,1.26l-2.17-1.72s5.61-9.84,18.77-23.01c0,0-6.87-2.63-14.77-13.39,0,0-11.1,6.87-12.94,24.96,2.86,2.29,8.13,6.18,8.13,6.18-4.24,3.09-6.87,5.5-8.24,6.87.23,7.78,0,15-.92,22.67,18.2,3.89,24.73,21.06,24.73,21.06Z";

export function Logo({ size = 36 }: P) {
  return (
    <svg width={size} height={size} viewBox="0 0 136.64 136.64" role="img" aria-label="Buni Font Manager logo" style={{ display: "block", borderRadius: 6, flex: "0 0 auto" }}>
      <rect fill="#FFBF00" width="136.64" height="136.64" />
      <path fill="#343434" d={LOGO_PATH} />
    </svg>
  );
}
