import type { FontRecord, Library } from "./types";

export const ALL = "All fonts";

/** One card in the grid: a family from one origin, with all its files. */
export interface Family {
  key: string;
  family: string;
  origin: string;
  fonts: FontRecord[];
  /** The file used for the preview: closest to Regular, upright. */
  rep: FontRecord;
  ids: string[];
  active: boolean;
  addedAt: number;
  licence: string | null;
  /** Style and spacing of the family as a whole (most common among its upright styles). */
  classification: string;
  spacing: string;
}

export interface TagInfo {
  label: string;
  /** Fixed tags come from the file or the import and can't be removed. */
  fixed: boolean;
  source: string;
}

export const STYLE_CHIPS = ["Serif", "Sans serif", "Script", "Decorative", "Blackletter", "Symbols", "Duo"];
export const SPACING_CHIPS = ["Normal", "Monospace", "Condensed", "Expanded"];
export const WEIGHT_CHIPS = ["Thin", "Light", "Regular", "Medium", "Bold", "Black"];
export const LANG_CHIPS = ["Swedish (å ä ö)", "Portuguese (ã ç)", "Cyrillic", "Greek"];

const WEIGHT_RANGES: Record<string, [number, number]> = {
  Thin: [1, 249],
  Light: [250, 349],
  Regular: [350, 449],
  Medium: [450, 549],
  Bold: [550, 749],
  Black: [750, 1000],
};

const WEIGHT_NAMES: [number, string][] = [
  [100, "Thin"], [200, "ExtraLight"], [300, "Light"], [400, "Regular"], [500, "Medium"],
  [600, "SemiBold"], [700, "Bold"], [800, "ExtraBold"], [900, "Black"],
];

export function weightName(w: number): string {
  let best = WEIGHT_NAMES[3];
  for (const e of WEIGHT_NAMES) if (Math.abs(e[0] - w) < Math.abs(best[0] - w)) best = e;
  return best[1];
}

export function familyKey(f: FontRecord): string {
  return `${f.origin}|${f.family}`;
}

export function buildFamilies(fonts: FontRecord[]): Family[] {
  const map = new Map<string, FontRecord[]>();
  for (const f of fonts) {
    const k = familyKey(f);
    const list = map.get(k);
    if (list) list.push(f);
    else map.set(k, [f]);
  }
  const out: Family[] = [];
  for (const [key, list] of map) {
    list.sort((a, b) => Number(a.italic) - Number(b.italic) || a.weight - b.weight || a.style.localeCompare(b.style));
    const upright = list.filter((f) => !f.italic);
    const pool = upright.length ? upright : list;
    const rep = pool.reduce((best, f) => (Math.abs(f.weight - 400) < Math.abs(best.weight - 400) ? f : best), pool[0]);
    const vote = (pick: (f: FontRecord) => string) => {
      const n = new Map<string, number>();
      for (const f of pool) n.set(pick(f), (n.get(pick(f)) ?? 0) + 1);
      let best = pick(rep);
      for (const [k, c] of n) if (c > (n.get(best) ?? 0)) best = k;
      return best;
    };
    out.push({
      key,
      family: list[0].family,
      origin: list[0].origin,
      fonts: list,
      rep,
      ids: list.map((f) => f.id),
      active: list.some((f) => f.active),
      addedAt: Math.max(...list.map((f) => f.addedAt)),
      licence: list.find((f) => f.licence)?.licence ?? null,
      classification: vote((f) => (f.classification === "Handwritten" ? "Script" : f.classification)),
      spacing: vote((f) => f.spacing),
    });
  }
  return out;
}

function uniq(xs: string[]): string[] {
  return Array.from(new Set(xs));
}

export function aiTags(fam: Family): string[] {
  return uniq(fam.fonts.flatMap((f) => f.aiTags.filter((t) => !f.hiddenAiTags.includes(t))));
}

export function customTags(fam: Family): string[] {
  return uniq(fam.fonts.flatMap((f) => f.customTags));
}

/** Tags read from the file or added by the import. */
export function fixedTags(fam: Family): string[] {
  const t = [fam.classification, fam.spacing];
  if (fam.fonts.some((f) => f.duo)) t.push("Duo");
  if (fam.fonts.some((f) => f.variable)) t.push("Variable");
  if (fam.fonts.some((f) => f.italic)) t.push("Italic");
  return uniq(t);
}

export function tagList(fam: Family): TagInfo[] {
  const out: TagInfo[] = fixedTags(fam).map((label) => ({
    label,
    fixed: true,
    source: label === "Duo" ? "From the import: came in the same zip as another family" : "Read from the font file",
  }));
  for (const t of aiTags(fam)) if (!out.some((o) => o.label === t)) out.push({ label: t, fixed: false, source: "From AI · right-click to remove" });
  for (const t of customTags(fam)) if (!out.some((o) => o.label === t)) out.push({ label: t, fixed: false, source: "Your tag · right-click to remove" });
  return out;
}

/** Tags that go in the "Other Tags" filter: everything outside the fixed groups. */
export function otherTags(fam: Family): string[] {
  const meta: string[] = [];
  if (fam.fonts.some((f) => f.variable)) meta.push("Variable");
  if (fam.fonts.some((f) => f.italic)) meta.push("Italic");
  return uniq([...aiTags(fam), ...meta, ...customTags(fam)]);
}

export type Filters = Record<string, boolean>;

export interface FilterGroup {
  key: string;
  label: string;
  chips: string[];
}

export function filterGroups(families: Family[]): FilterGroup[] {
  const origins = uniq(families.map((f) => f.origin)).sort();
  const others = uniq(families.flatMap(otherTags)).sort((a, b) => a.localeCompare(b));
  return [
    { key: "origin", label: "Sources", chips: origins },
    { key: "style", label: "Style", chips: STYLE_CHIPS },
    { key: "spacing", label: "Spacing", chips: SPACING_CHIPS },
    { key: "weight", label: "Weight", chips: WEIGHT_CHIPS },
    { key: "lang", label: "Languages", chips: LANG_CHIPS },
    { key: "other", label: "Other Tags", chips: others },
  ].filter((g) => g.chips.length > 0);
}

function chipMatches(group: string, chip: string, fam: Family): boolean {
  switch (group) {
    case "origin":
      return fam.origin === chip;
    case "style":
      return chip === "Duo" ? fam.fonts.some((f) => f.duo) : fam.classification === chip;
    case "spacing":
      return fam.spacing === chip;
    case "weight": {
      const [lo, hi] = WEIGHT_RANGES[chip];
      return fam.fonts.some((f) => f.weight >= lo && f.weight <= hi);
    }
    case "lang":
      return fam.rep.langs.includes(chip);
    case "other":
      return otherTags(fam).includes(chip);
  }
  return false;
}

/** The style shown on the card: with weight filters on, a style of that weight. */
export function previewFont(fam: Family, filters: Filters): FontRecord {
  const ranges = WEIGHT_CHIPS.filter((c) => filters[`weight:${c}`]).map((c) => WEIGHT_RANGES[c]);
  if (!ranges.length) return fam.rep;
  const inRange = fam.fonts.filter((f) => ranges.some(([lo, hi]) => f.weight >= lo && f.weight <= hi));
  if (!inRange.length) return fam.rep;
  const upright = inRange.filter((f) => !f.italic);
  const pool = upright.length ? upright : inRange;
  // Several weights picked: show the heaviest, it differs most from the default preview.
  return pool.reduce((a, b) => (b.weight > a.weight ? b : a), pool[0]);
}

/** Within a group chips are OR; across groups they are AND. */
export function matchesFilters(fam: Family, groups: FilterGroup[], filters: Filters): boolean {
  return groups.every((g) => {
    const on = g.chips.filter((c) => filters[`${g.key}:${c}`]);
    return on.length === 0 || on.some((c) => chipMatches(g.key, c, fam));
  });
}

/** Deep copy so undo snapshots never share arrays with the live state. */
export function cloneLibrary(lib: Library): Library {
  return JSON.parse(JSON.stringify(lib)) as Library;
}

export function fontFaceName(id: string): string {
  return `bfm-${id}`;
}
