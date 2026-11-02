// A single glyph, large, with its anchor points, bezier handles and metrics.
import { useEffect, useState } from "react";
import { api } from "./api";
import type { GlyphOutline, OutlinePoint } from "./types";

export type GlyphRef = { cp: number } | { gid: number; name: string | null };

const hex = (cp: number) => "U+" + cp.toString(16).toUpperCase().padStart(4, "0");

/** SVG path data for the outline (y flipped: font units point up, SVG down). */
function pathData(o: GlyphOutline): string {
  let d = "";
  for (const c of o.contours) {
    if (!c.length) continue;
    d += `M${c[0].x} ${-c[0].y}`;
    let i = 1;
    const n = c.length;
    const at = (k: number) => c[k % n];
    while (i <= n) {
      const p = at(i);
      if (p.on) {
        if (i < n) d += `L${p.x} ${-p.y}`;
        i += 1;
      } else if (o.kind === "quadratic") {
        const e = at(i + 1);
        d += `Q${p.x} ${-p.y} ${e.x} ${-e.y}`;
        i += 2;
      } else {
        const p2 = at(i + 1);
        const e = at(i + 2);
        d += `C${p.x} ${-p.y} ${p2.x} ${-p2.y} ${e.x} ${-e.y}`;
        i += 3;
      }
    }
    d += "Z";
  }
  return d;
}

/** Handle lines: each control point joined to the anchor it belongs to. */
function handles(o: GlyphOutline): [OutlinePoint, OutlinePoint][] {
  const out: [OutlinePoint, OutlinePoint][] = [];
  for (const c of o.contours) {
    const n = c.length;
    for (let i = 0; i < n; i++) {
      const p = c[i];
      if (p.on) continue;
      const prev = c[(i - 1 + n) % n];
      const next = c[(i + 1) % n];
      if (o.kind === "quadratic") {
        out.push([prev, p], [p, next]);
      } else {
        if (prev.on) out.push([prev, p]);
        if (next.on) out.push([p, next]);
      }
    }
  }
  return out;
}

/** An anchor is smooth when its two handles (or neighbours) line up. */
function isSmooth(c: OutlinePoint[], i: number): boolean {
  const n = c.length;
  const a = c[(i - 1 + n) % n];
  const p = c[i];
  const b = c[(i + 1) % n];
  if (a.on && b.on) return false;
  const v1 = [p.x - a.x, p.y - a.y];
  const v2 = [b.x - p.x, b.y - p.y];
  const l1 = Math.hypot(v1[0], v1[1]);
  const l2 = Math.hypot(v2[0], v2[1]);
  if (!l1 || !l2) return false;
  return (v1[0] * v2[0] + v1[1] * v2[1]) / (l1 * l2) > 0.995;
}

export function GlyphZoom(props: {
  path: string;
  index: number;
  fontFamily: string;
  list: GlyphRef[];
  at: number;
  onAt: (i: number) => void;
  onClose: () => void;
  onCopy: (cp: number) => void;
}) {
  const { path, index, fontFamily, list, at, onAt, onClose, onCopy } = props;
  const ref = list[at];
  const [o, setO] = useState<GlyphOutline | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [points, setPoints] = useState(true);

  useEffect(() => {
    let live = true;
    setErr(null);
    const cp = "cp" in ref ? ref.cp : null;
    const gid = "gid" in ref ? ref.gid : null;
    api
      .glyphOutline(path, index, cp, gid)
      .then((g) => live && setO(g))
      .catch(() => live && setErr("This glyph's outline couldn't be read"));
    return () => {
      live = false;
    };
  }, [path, index, ref]);

  useEffect(() => {
    function key(e: KeyboardEvent) {
      if (e.key === "Escape" || e.key === " ") {
        e.preventDefault();
        e.stopImmediatePropagation();
        onClose();
      } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        e.stopImmediatePropagation();
        onAt(Math.min(list.length - 1, at + 1));
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        e.stopImmediatePropagation();
        onAt(Math.max(0, at - 1));
      }
    }
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [at, list.length, onAt, onClose]);

  const title = "cp" in ref ? `${String.fromCodePoint(ref.cp)}  ${hex(ref.cp)}` : ref.name ?? `Glyph ${ref.gid}`;

  let svg = null;
  let info = null;
  if (o) {
    const top = Math.max(o.ascender, o.yMax, o.capHeight ?? 0);
    const bottom = Math.min(o.descender, o.yMin);
    const left = Math.min(0, o.xMin);
    const right = Math.max(o.advance, o.xMax);
    const padX = (right - left) * 0.12 + o.unitsPerEm * 0.05;
    const padY = (top - bottom) * 0.06;
    const vb = [left - padX, -top - padY, right - left + padX * 2, top - bottom + padY * 2];
    const u = vb[3] / 600; // one "screen pixel" in font units, roughly
    const lines: [string, number][] = [
      ["Ascender", o.ascender],
      ["Cap height", o.capHeight ?? NaN],
      ["x-height", o.xHeight ?? NaN],
      ["Baseline", 0],
      ["Descender", o.descender],
    ];
    const count = o.contours.reduce((a, c) => a + c.length, 0);
    svg = (
      <svg className="gz-svg" viewBox={vb.join(" ")} preserveAspectRatio="xMidYMid meet" aria-label={`Outline of ${title}`}>
        {lines
          .filter(([, y]) => Number.isFinite(y))
          .map(([name, y]) => (
            <g key={name}>
              <line x1={vb[0]} x2={vb[0] + vb[2]} y1={-y} y2={-y} className={name === "Baseline" ? "gz-base" : "gz-metric"} />
              <text x={vb[0] + 6 * u} y={-y - 4 * u} fontSize={11 * u} className="gz-label">
                {name}
              </text>
            </g>
          ))}
        <line x1={0} x2={0} y1={vb[1]} y2={vb[1] + vb[3]} className="gz-metric" />
        <line x1={o.advance} x2={o.advance} y1={vb[1]} y2={vb[1] + vb[3]} className="gz-metric" />
        <path d={pathData(o)} className={points ? "gz-fill gz-fill-light" : "gz-fill"} fillRule="nonzero" />
        {points && (
          <>
            <path d={pathData(o)} className="gz-stroke" />
            {handles(o).map(([a, b], i) => (
              <line key={i} x1={a.x} y1={-a.y} x2={b.x} y2={-b.y} className="gz-handle" />
            ))}
            {o.contours.map((c, ci) =>
              c.map((p, i) =>
                p.on ? (
                  isSmooth(c, i) ? (
                    <circle key={`${ci}-${i}`} cx={p.x} cy={-p.y} r={4 * u} className="gz-anchor" />
                  ) : (
                    <rect key={`${ci}-${i}`} x={p.x - 4 * u} y={-p.y - 4 * u} width={8 * u} height={8 * u} className="gz-anchor" />
                  )
                ) : (
                  <circle key={`${ci}-${i}`} cx={p.x} cy={-p.y} r={3 * u} className="gz-control" />
                ),
              ),
            )}
          </>
        )}
      </svg>
    );
    info = (
      <dl className="gz-info">
        <dt>Glyph</dt>
        <dd>{o.name ?? `#${o.glyphId}`}</dd>
        <dt>Width</dt>
        <dd>{o.advance}</dd>
        <dt>Left side</dt>
        <dd>{o.xMin}</dd>
        <dt>Right side</dt>
        <dd>{o.advance - o.xMax}</dd>
        <dt>Points</dt>
        <dd>{count}</dd>
        <dt>Curves</dt>
        <dd>{o.kind === "cubic" ? "Cubic (two handles, like Illustrator)" : "Quadratic (TrueType, one handle per curve)"}</dd>
        <dt>Units per em</dt>
        <dd>{o.unitsPerEm}</dd>
      </dl>
    );
  }

  return (
    <div className="gz" role="dialog" aria-label={`Glyph ${title}`}>
      <div className="gz-bar">
        <button type="button" className="btn" onClick={onClose}>
          ← Back
        </button>
        <b className="gz-title" style={{ fontFamily }}>
          {"cp" in ref ? String.fromCodePoint(ref.cp) : ""}
        </b>
        <span className="muted">{title}</span>
        <span style={{ flex: "1 1 auto" }} />
        <button type="button" className="chip-btn light" aria-pressed={points} onClick={() => setPoints(!points)}>
          Points and handles
        </button>
        {"cp" in ref && (
          <button type="button" className="btn" onClick={() => onCopy(ref.cp)}>
            Copy
          </button>
        )}
        <button type="button" className="icon-round" aria-label="Previous" disabled={at === 0} onClick={() => onAt(at - 1)}>
          ←
        </button>
        <button type="button" className="icon-round" aria-label="Next" disabled={at === list.length - 1} onClick={() => onAt(at + 1)}>
          →
        </button>
      </div>
      <div className="gz-body">
        {err ? <p className="empty">{err}</p> : svg ?? <p className="muted">Reading the outline…</p>}
        {info}
      </div>
    </div>
  );
}
