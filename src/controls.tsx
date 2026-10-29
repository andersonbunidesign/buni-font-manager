// Small controls: a slider that stays fast with hundreds of previews, and a colour picker.
import { useEffect, useRef, useState } from "react";
import { IconReset } from "./icons";

/**
 * A range slider. While dragging it only calls `onLive` (cheap, e.g. a CSS variable);
 * the expensive `onCommit` runs when the thumb is released.
 */
export function Slider(props: {
  id: string;
  label: (v: number) => string;
  min: number;
  max: number;
  step?: number;
  value: number;
  onLive?: (v: number) => void;
  onCommit: (v: number) => void;
}) {
  const { id, label, min, max, step = 1, value, onLive, onCommit } = props;
  const [v, setV] = useState(value);
  const dragging = useRef(false);
  useEffect(() => {
    if (!dragging.current) setV(value);
  }, [value]);
  const commit = () => {
    dragging.current = false;
    if (v !== value) onCommit(v);
  };
  const pct = ((v - min) / (max - min)) * 100;
  return (
    <>
      <label htmlFor={id}>{label(v)}</label>
      <input
        id={id}
        type="range"
        className="slider"
        min={min}
        max={max}
        step={step}
        value={v}
        style={{ "--fill": `${pct}%` } as React.CSSProperties}
        onPointerDown={() => (dragging.current = true)}
        onChange={(e) => {
          const nv = Number(e.target.value);
          setV(nv);
          onLive?.(nv);
          // Keyboard changes commit right away.
          if (!dragging.current) onCommit(nv);
        }}
        onPointerUp={commit}
        onPointerCancel={commit}
        onBlur={commit}
      />
    </>
  );
}

// ---------- colour maths ----------
export function hexToRgb(hex: string): [number, number, number] | null {
  const m = hex.trim().replace(/^#/, "");
  const full = m.length === 3 ? m.split("").map((c) => c + c).join("") : m;
  if (!/^[0-9a-fA-F]{6}$/.test(full)) return null;
  return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16)) as [number, number, number];
}

export function normHex(hex: string): string | null {
  const rgb = hexToRgb(hex);
  return rgb ? "#" + rgb.map((c) => c.toString(16).padStart(2, "0")).join("").toUpperCase() : null;
}

function rgbToHsv([r, g, b]: [number, number, number]): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  return [h, max ? d / max : 0, max];
}

function hsvToHex(h: number, s: number, v: number): string {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return Math.round(255 * (v - v * s * Math.max(0, Math.min(k, 4 - k, 1))));
  };
  return "#" + [f(5), f(3), f(1)].map((c) => c.toString(16).padStart(2, "0")).join("").toUpperCase();
}

/** Colour picker: saturation/brightness square, hue bar, hex field and recent colours. */
export function ColorPicker(props: {
  label: string;
  value: string;
  recent: string[];
  onChange: (hex: string) => void;
  /** Called when a colour is settled (released, typed, picked), to remember it. */
  onPick: (hex: string) => void;
  onReset: () => void;
  onClose: () => void;
}) {
  const { label, value, recent, onChange, onPick, onReset, onClose } = props;
  const [hsv, setHsv] = useState<[number, number, number]>(() => rgbToHsv(hexToRgb(value) ?? [0, 0, 0]));
  const [hexText, setHexText] = useState(value);
  const box = useRef<HTMLDivElement>(null);
  const ref = useRef<HTMLDivElement>(null);
  const last = useRef(value);

  useEffect(() => {
    if (value !== last.current) {
      last.current = value;
      setHexText(value);
      const rgb = hexToRgb(value);
      if (rgb) setHsv(rgbToHsv(rgb));
    }
  }, [value]);

  useEffect(() => {
    function away(e: PointerEvent) {
      if (ref.current && !ref.current.contains(e.target as Node) && !(e.target as HTMLElement).closest?.("[data-swatch]")) onClose();
    }
    window.addEventListener("pointerdown", away);
    return () => window.removeEventListener("pointerdown", away);
  }, [onClose]);

  const set = (h: number, s: number, v: number) => {
    setHsv([h, s, v]);
    const hex = hsvToHex(h, s, v);
    last.current = hex;
    setHexText(hex);
    onChange(hex);
    return hex;
  };

  function drag(e: React.PointerEvent, kind: "sv" | "hue") {
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    let hex = value;
    const move = (ev: PointerEvent | React.PointerEvent) => {
      const r = el.getBoundingClientRect();
      const x = Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width));
      const y = Math.min(1, Math.max(0, (ev.clientY - r.top) / r.height));
      hex = kind === "sv" ? set(hsv[0], x, 1 - y) : set(x * 360, hsv[1], hsv[2]);
    };
    move(e);
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      onPick(hex);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  }

  const [h, s, v] = hsv;
  return (
    <div className="picker" ref={ref} role="dialog" aria-label={label}>
      <div className="picker-head">
        <b>{label}</b>
        <button type="button" className="link" onClick={onReset} title="Back to the default colours">
          <IconReset size={14} /> Default
        </button>
      </div>
      <div ref={box} className="sv" style={{ background: `hsl(${h} 100% 50%)` }} onPointerDown={(e) => drag(e, "sv")}>
        <div className="sv-white" />
        <div className="sv-black" />
        <i style={{ left: `${s * 100}%`, top: `${(1 - v) * 100}%` }} />
      </div>
      <div className="hue" onPointerDown={(e) => drag(e, "hue")}>
        <i style={{ left: `${(h / 360) * 100}%` }} />
      </div>
      <div className="picker-row">
        <span className="swatch" style={{ background: value }} />
        <input
          aria-label="Hex colour code"
          value={hexText}
          spellCheck={false}
          onChange={(e) => {
            setHexText(e.target.value);
            const n = normHex(e.target.value);
            if (n) {
              last.current = n;
              setHsv(rgbToHsv(hexToRgb(n)!));
              onChange(n);
            }
          }}
          onBlur={() => {
            const n = normHex(hexText);
            if (n) onPick(n);
            else setHexText(value);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          }}
        />
      </div>
      {recent.length > 0 && (
        <div className="recent" aria-label="Recent colours">
          {recent.map((c) => (
            <button
              key={c}
              type="button"
              className="swatch"
              title={c}
              aria-label={`Use ${c}`}
              style={{ background: c }}
              onClick={() => {
                onChange(c);
                onPick(c);
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
}
