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
    <path d="M12 4v10M8 10l4 4 4-4M4 16v4h16v-4" />
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

export function Star({ on, size = 20 }: { on: boolean; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill={on ? "#FFBF00" : "none"} stroke={on ? "#C99700" : "var(--star-off)"} strokeWidth="1.8" strokeLinejoin="round" aria-hidden="true">
      <path d="M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z" />
    </svg>
  );
}

export function Logo({ size = 36 }: P) {
  return (
    <svg width={size} height={size} viewBox="0 0 56.69 56.69" role="img" aria-label="Buni Font Manager logo" style={{ display: "block", borderRadius: 6, flex: "0 0 auto" }}>
      <rect fill="#FFBF00" width="56.69" height="56.69" />
      <g fill="#343434">
        <path d="M23.74,29.79c-.16-.16-.37-.25-.61-.25h-8.72c-.26,0-.38.13-.38.38v19.71c0,.26.13.38.38.38h4.35c.26,0,.38-.13.38-.38v-16h2.83v16c0,.26.13.38.38.38h4.35c.26,0,.38-.13.38-.38v-16.16c0-.24-.09-.44-.26-.61l-3.07-3.07Z" />
        <rect x="29.62" y="29.54" width="5.11" height="20.47" rx=".38" ry=".38" />
        <path d="M42.29,44.3h-4.91c-.26,0-.38.14-.38.41v4.88c0,.28.13.41.38.41h4.91c.26,0,.38-.14.38-.41v-4.88c0-.28-.13-.41-.38-.41Z" />
        <path d="M14.4,27.15h8.72c.25,0,.45-.08.62-.25l3.07-3.07c.17-.17.26-.38.26-.61v-2.87c0-.23-.09-.46-.26-.62l-2.19-2.19c-.34-.34-.34-.9,0-1.24l2.19-2.19c.16-.16.26-.39.26-.62v-2.87c0-.24-.09-.44-.26-.62l-3.07-3.07c-.16-.16-.37-.25-.62-.25h-8.72c-.26,0-.38.13-.38.38v19.71c0,.21.17.38.38.38ZM19.13,10.76h2.83v3.15l-2.38,2.38c-.34.34-.34.9,0,1.24l2.38,2.38v3.15h-2.83v-12.3Z" />
        <path d="M42.29,6.68h-4.35c-.26,0-.38.13-.38.38v16h-2.83V7.06c0-.26-.13-.38-.38-.38h-4.35c-.26,0-.38.13-.38.38v16.16c0,.24.09.44.26.61l3.07,3.07c.16.16.37.25.61.25h8.72c.26,0,.38-.13.38-.38V7.06c0-.26-.13-.38-.38-.38Z" />
      </g>
    </svg>
  );
}
