// The font page: one family, its sample text and every character it has.
import { useEffect, useMemo, useState } from "react";
import { api } from "./api";
import type { FontRecord, GlyphMap } from "./types";
import { fontFaceName, weightName, type Family } from "./model";
import { IS_MAC } from "./platform";

// Windows-1252 characters 0x80–0x9F, typed with Alt+0128 … Alt+0159.
const CP1252_HIGH: Record<number, number> = {
  0x20ac: 0x80, 0x201a: 0x82, 0x0192: 0x83, 0x201e: 0x84, 0x2026: 0x85, 0x2020: 0x86, 0x2021: 0x87, 0x02c6: 0x88,
  0x2030: 0x89, 0x0160: 0x8a, 0x2039: 0x8b, 0x0152: 0x8c, 0x017d: 0x8e, 0x2018: 0x91, 0x2019: 0x92, 0x201c: 0x93,
  0x201d: 0x94, 0x2022: 0x95, 0x2013: 0x96, 0x2014: 0x97, 0x02dc: 0x98, 0x2122: 0x99, 0x0161: 0x9a, 0x203a: 0x9b,
  0x0153: 0x9c, 0x017e: 0x9e, 0x0178: 0x9f,
};

// Mac (US and most Latin layouts): Option shortcuts and dead keys for accents.
const MAC_OPT: Record<string, string> = {
  "©": "G", "®": "R", "™": "2", "€": "Shift 2", "•": "8", "–": "-", "—": "Shift -", "“": "[", "”": "Shift [", "‘": "]", "’": "Shift ]",
  "…": ";", "°": "Shift 8", "±": "Shift =", "÷": "/", "≠": "=", "≤": ",", "≥": ".", "¢": "4", "£": "3", "¥": "Y", "§": "6", "¶": "7",
  "ß": "S", "ø": "O", "Ø": "Shift O", "å": "A", "Å": "Shift A", "æ": "'", "Æ": "Shift '", "œ": "Q", "Œ": "Shift Q", "ç": "C",
  "Ç": "Shift C", "¡": "1", "¿": "Shift /", "«": "\\", "»": "Shift \\", "π": "P", "∞": "5", "µ": "M", "√": "V", "∑": "W", "ƒ": "F",
  "∆": "J", "†": "T", "‡": "Shift 7", "‰": "Shift R", "¬": "L", "∂": "D", "Ω": "Z", "≈": "X", "º": "0", "ª": "9", "·": "Shift 9",
  "‹": "Shift 3", "›": "Shift 4", "„": "Shift W", "‚": "Shift 0", "ﬁ": "Shift 5", "ﬂ": "Shift 6", "\uf8ff": "Shift K",
};
const MAC_DEAD: Record<string, [string, string]> = {
  "\u0301": ["E", "acute"], "\u0300": ["`", "grave"], "\u0302": ["I", "circumflex"], "\u0308": ["U", "umlaut"], "\u0303": ["N", "tilde"],
};

function howToTypeMac(cp: number): string {
  if (cp === 0x20) return "Space bar";
  if (cp >= 0x21 && cp <= 0x7e) return `Keyboard key ${String.fromCodePoint(cp)}`;
  const ch = String.fromCodePoint(cp);
  if (MAC_OPT[ch]) return `Option + ${MAC_OPT[ch]}`;
  // Accented letters: Option + accent key, then the letter.
  const parts = ch.normalize("NFD");
  if (parts.length === 2 && /[a-zA-Z]/.test(parts[0]) && MAC_DEAD[parts[1]]) {
    const [key, name] = MAC_DEAD[parts[1]];
    return `Option + ${key}, then ${parts[0]} (${name})`;
  }
  return "No keyboard shortcut: click to copy it, use the Character Viewer (Ctrl + Cmd + Space) or Illustrator's Glyphs panel";
}

/** How to type a character on this computer. */
export function howToType(cp: number): string {
  if (IS_MAC) return howToTypeMac(cp);
  if (cp === 0x20) return "Space bar";
  if (cp >= 0x21 && cp <= 0x7e) return `Keyboard key ${String.fromCodePoint(cp)}`;
  const b = cp >= 0xa0 && cp <= 0xff ? cp : CP1252_HIGH[cp];
  if (b !== undefined) return `Alt + 0${b} on the number pad`;
  return "No keyboard shortcut: click to copy it, or use Illustrator's Glyphs panel";
}

const hex = (cp: number) => "U+" + cp.toString(16).toUpperCase().padStart(4, "0");
const MAX_CHARS = 3000;

export function FontPage(props: {
  fam: Family;
  initialId: string;
  text: string;
  caps: boolean;
  align: "left" | "center";
  pad: number;
  onBack: () => void;
  say: (msg: string) => void;
}) {
  const { fam, initialId, text, caps, align, pad, onBack, say } = props;
  const [fontId, setFontId] = useState(fam.fonts.some((f) => f.id === initialId) ? initialId : fam.rep.id);
  const font: FontRecord = fam.fonts.find((f) => f.id === fontId) ?? fam.rep;
  const [glyphs, setGlyphs] = useState<GlyphMap | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);

  useEffect(() => {
    let live = true;
    setGlyphs(null);
    setError(null);
    api
      .fontGlyphs(font.path, font.index ?? 0)
      .then((g) => live && setGlyphs(g))
      .catch((e) => live && setError(typeof e === "string" ? e : "Could not read this font's characters"));
    return () => {
      live = false;
    };
  }, [font.path, font.index]);

  // Control characters have nothing to show.
  const chars = useMemo(() => (glyphs?.chars ?? []).filter((cp) => cp >= 0x20 && !(cp >= 0x7f && cp < 0xa0) && !(cp >= 0xd800 && cp < 0xe000)), [glyphs]);
  const shownChars = showAll ? chars : chars.slice(0, MAX_CHARS);
  const family = `"${fontFaceName(font.id)}", "Segoe UI", sans-serif`;
  const cell = `calc(var(--pv-size) * 1.5 + ${Math.round(16 * pad)}px)`;

  function copy(ch: string, label: string) {
    navigator.clipboard
      .writeText(ch)
      .then(() => say(`Copied ${label}`))
      .catch(() => say("Couldn't copy"));
  }

  return (
    <div className="fontpage">
      <div className="fp-head">
        <button type="button" className="btn" onClick={onBack} title="Back (Esc)">
          ← Back
        </button>
        <div style={{ minWidth: 0 }}>
          <p className="kicker">{fam.origin}</p>
          <h1>{fam.family}</h1>
        </div>
      </div>

      {fam.fonts.length > 1 && (
        <div className="fp-styles" role="group" aria-label="Weights and styles">
          {fam.fonts.map((f) => (
            <button key={f.id} type="button" className="chip-btn light" aria-pressed={f.id === font.id} onClick={() => setFontId(f.id)}>
              {f.style || weightName(f.weight)}
            </button>
          ))}
        </div>
      )}

      <div
        className="fp-sample"
        style={{ fontFamily: family, fontSize: "var(--pv-size)", textTransform: caps ? "uppercase" : "none", textAlign: align, padding: `${Math.round(20 * pad)}px ${Math.round(24 * pad)}px` }}
      >
        {text}
      </div>

      <div className="fp-meta">
        <span>
          {font.style} · {weightName(font.weight)} {font.weight}
        </span>
        {glyphs && (
          <span>
            {chars.length.toLocaleString()} characters
            {glyphs.unmapped.length + glyphs.unmappedMore > 0 ? ` · ${(glyphs.unmapped.length + glyphs.unmappedMore).toLocaleString()} alternates and other glyphs` : ""}
          </span>
        )}
        <span>Hover a character to see how to type it, click to copy it</span>
      </div>

      {error && <p className="empty">{error}</p>}
      {!glyphs && !error && <p className="muted">Reading the characters…</p>}

      {glyphs && (
        <>
          <div className="glyphs" style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${cell}, 1fr))` }}>
            {shownChars.map((cp) => {
              const ch = String.fromCodePoint(cp);
              const tip = `${cp === 0x20 ? "Space" : ch}  ${hex(cp)}\n${howToType(cp)}`;
              return (
                <button key={cp} type="button" className="glyph" data-tip={tip} aria-label={`${hex(cp)}: ${howToType(cp)}`} onClick={() => copy(ch, cp === 0x20 ? "a space" : `${ch} (${hex(cp)})`)}>
                  <span style={{ fontFamily: family, fontSize: "var(--pv-size)" }}>{ch}</span>
                  <small>{hex(cp)}</small>
                </button>
              );
            })}
          </div>
          {chars.length > MAX_CHARS && !showAll && (
            <button type="button" className="btn" style={{ alignSelf: "center" }} onClick={() => setShowAll(true)}>
              Show all {chars.length.toLocaleString()} characters
            </button>
          )}

          {glyphs.unmapped.length > 0 && (
            <>
              <h3 style={{ margin: "8px 0 0" }}>Alternates and other glyphs</h3>
              <p className="muted" style={{ margin: 0, fontSize: 12 }}>
                These have no keyboard key or character code. Use them from Illustrator's Glyphs panel or with OpenType features.
              </p>
              <div className="glyphs" style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${cell}, 1fr))` }}>
                {glyphs.unmapped.map((g) => {
                  const top = glyphs.ascender;
                  const h = glyphs.ascender - glyphs.descender;
                  const w = Math.max(g.advance, glyphs.unitsPerEm * 0.3);
                  return (
                    <button
                      key={g.id}
                      type="button"
                      className="glyph"
                      data-tip={`${g.name ?? `Glyph ${g.id}`}\nNo character code: use Illustrator's Glyphs panel`}
                      aria-label={g.name ?? `Glyph ${g.id}`}
                      onClick={() => say("This glyph has no character code, so it can't be copied as text. Use Illustrator's Glyphs panel.")}
                    >
                      <svg viewBox={`0 ${-top} ${w} ${h}`} style={{ height: "calc(var(--pv-size) * 1.15)", width: "auto", maxWidth: "100%", overflow: "visible" }} aria-hidden="true">
                        <path d={g.path} fill="currentColor" />
                      </svg>
                      <small>{g.name ?? `#${g.id}`}</small>
                    </button>
                  );
                })}
              </div>
              {glyphs.unmappedMore > 0 && <p className="muted">…and {glyphs.unmappedMore.toLocaleString()} more.</p>}
            </>
          )}
        </>
      )}
    </div>
  );
}
