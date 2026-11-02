// Space bar: one font at a large size, filling the window. Scroll zooms, drag pans,
// arrows move to the next or previous font, Space or Esc closes.
import { useEffect, useRef, useState } from "react";
import { fontFaceName, type Family } from "./model";
import type { FontRecord } from "./types";

export function QuickLook(props: {
  fams: Family[];
  index: number;
  fontOf: (f: Family) => FontRecord;
  /** Width of the sample at 100px, to fit it to the window. */
  widthAt100: (f: Family) => number | undefined;
  text: string;
  caps: boolean;
  colors: { text: string; bg: string } | null;
  onIndex: (i: number) => void;
  onClose: () => void;
}) {
  const { fams, index, fontOf, widthAt100, text, caps, colors, onIndex, onClose } = props;
  const fam = fams[index];
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const drag = useRef<{ x: number; y: number; px: number; py: number } | null>(null);
  const [vw, setVw] = useState(window.innerWidth);
  const [vh, setVh] = useState(window.innerHeight);

  useEffect(() => {
    const r = () => {
      setVw(window.innerWidth);
      setVh(window.innerHeight);
    };
    window.addEventListener("resize", r);
    return () => window.removeEventListener("resize", r);
  }, []);

  // A new font starts fitted and centred.
  useEffect(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, [index]);

  useEffect(() => {
    function key(e: KeyboardEvent) {
      if (e.key === "Escape" || e.key === " ") {
        e.preventDefault();
        e.stopImmediatePropagation();
        onClose();
      } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        e.stopImmediatePropagation();
        onIndex(Math.min(fams.length - 1, index + 1));
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        e.stopImmediatePropagation();
        onIndex(Math.max(0, index - 1));
      } else if (e.key === "0") {
        setZoom(1);
        setPan({ x: 0, y: 0 });
      }
    }
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [fams.length, index, onClose, onIndex]);

  if (!fam) return null;
  const f = fontOf(fam);
  const w100 = widthAt100(fam) ?? 400;
  // Fit: 80% of the window width, at most 45% of its height.
  const base = Math.max(24, Math.min((vw * 0.8 * 100) / Math.max(w100, 1), vh * 0.45));
  const size = base * zoom;

  return (
    <div
      className="quicklook"
      style={colors ? { background: colors.bg, color: colors.text } : undefined}
      onWheel={(e) => {
        const k = Math.exp(-e.deltaY * 0.0015);
        setZoom((z) => Math.min(20, Math.max(0.1, z * k)));
      }}
      onPointerDown={(e) => {
        if ((e.target as HTMLElement).closest("button")) return;
        (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
        drag.current = { x: e.clientX, y: e.clientY, px: pan.x, py: pan.y };
      }}
      onPointerMove={(e) => {
        const d = drag.current;
        if (d) setPan({ x: d.px + e.clientX - d.x, y: d.py + e.clientY - d.y });
      }}
      onPointerUp={() => (drag.current = null)}
      role="dialog"
      aria-label={`${fam.family}, large preview`}
    >
      <div className="ql-bar">
        <div>
          <b>{fam.family}</b> <span>{f.style}</span>
          <small>{fam.origin}</small>
        </div>
        <div className="ql-hint">
          {index + 1} / {fams.length} · Scroll to zoom, drag to move, arrows for next and previous, 0 to reset, Space or Esc to close
        </div>
        <button type="button" className="icon-round" aria-label="Close" onClick={onClose}>
          ✕
        </button>
      </div>
      <div className="ql-stage">
        <div
          className="ql-text"
          style={{
            fontFamily: `"${fontFaceName(f.id)}", "Segoe UI", sans-serif`,
            fontSize: size,
            textTransform: caps ? "uppercase" : "none",
            transform: `translate(${pan.x}px, ${pan.y}px)`,
          }}
        >
          {text}
        </div>
      </div>
      <div className="ql-nav">
        <button type="button" className="icon-round" aria-label="Previous font" disabled={index === 0} onClick={() => onIndex(index - 1)}>
          ←
        </button>
        <button type="button" className="icon-round" aria-label="Next font" disabled={index === fams.length - 1} onClick={() => onIndex(index + 1)}>
          →
        </button>
      </div>
    </div>
  );
}
