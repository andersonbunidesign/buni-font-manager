import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { openPath, openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { enable as enableAutostart, disable as disableAutostart, isEnabled as autostartEnabled } from "@tauri-apps/plugin-autostart";
import { api } from "./api";
import type { DesignFileFonts, ExternalMeta, FontRecord, ImportReport, Library, Settings } from "./types";
import {
  ALL, buildFamilies, cloneLibrary, filterGroups, fontFaceName, matchesFilters, previewFont, tagList, weightName,
  type Family, type Filters,
} from "./model";
import {
  IconAlignCenter, IconAlignLeft, IconArrowIn, IconCheckCircle, IconClock, IconX, IconDots, IconFocus, IconFolder, IconGear, IconGrid, IconList, IconMinusCircle,
  IconMoon, IconPlus, IconSearch, IconSun, IconTrash, IconUpload, Logo, Star, IconSwap, IconReset, IconWidth, IconCopy, IconArtwork,
} from "./icons";
import { ColorPicker, Slider, normHex } from "./controls";
import { FontPage } from "./FontPage";

/** Sets the preview size without re-rendering the cards. */
function setPvSize(v: number) {
  document.documentElement.style.setProperty("--pv-size", `${v}px`);
}

type UndoEntry = { kind: "meta"; snapshot: Library } | { kind: "delete"; snapshot: Library; removed: FontRecord[] };
type Modal = null | "import" | "report" | "settings" | "adobe" | "design";

const isDesignFile = (p: string) => /\.(ai|eps)$/i.test(p);

const ADOBE = "Adobe Fonts";

/** Tags editable on any font; for system and Adobe fonts they live in the library's externalMeta. */
type TagHolder = Pick<FontRecord, "customTags" | "hiddenAiTags" | "aiTags">;

function editTags(l: Library, ids: string[], fn: (h: TagHolder) => void) {
  const want = new Set(ids);
  for (const f of l.fonts) if (want.has(f.id)) fn(f);
  const meta = (l.externalMeta ??= {});
  for (const id of ids) {
    if (l.fonts.some((f) => f.id === id)) continue;
    const m: ExternalMeta = meta[id] ?? { customTags: [], hiddenAiTags: [], aiTags: [] };
    fn(m);
    if (m.customTags.length || m.hiddenAiTags.length || m.aiTags.length) meta[id] = m;
    else delete meta[id];
  }
}

const DEFAULT_ORIGINS = ["Envato", "Google Fonts", "Creative Market", "Client"];
const EMPTY: Library = { version: 1, fonts: [], collections: [], origins: [] };

function usePref<T>(key: string, initial: T): [T, (v: T) => void] {
  const [v, setV] = useState<T>(() => {
    try {
      const raw = localStorage.getItem(`bfm.${key}`);
      return raw === null ? initial : (JSON.parse(raw) as T);
    } catch {
      return initial;
    }
  });
  const set = useCallback(
    (nv: T) => {
      setV(nv);
      try {
        localStorage.setItem(`bfm.${key}`, JSON.stringify(nv));
      } catch {
        /* preferences are a convenience only */
      }
    },
    [key],
  );
  return [v, set];
}

function plural(n: number) {
  return n === 1 ? "1 font" : `${n} fonts`;
}

function errorText(e: unknown) {
  return typeof e === "string" ? e : e instanceof Error ? e.message : "Something went wrong";
}

export default function App() {
  const [lib, setLib] = useState<Library>(EMPTY);
  const [libraryDir, setLibraryDir] = useState("");
  const [version, setVersion] = useState("");
  const [loaded, setLoaded] = useState(false);

  const [dark, setDark] = usePref("dark", false);
  const [view, setView] = usePref<"grid" | "list">("view", "grid");
  const [size, setSize] = usePref("size", 44);
  const [sameWidth, setSameWidth] = usePref("sameWidth", false);
  const [colors, setColors] = usePref<{ on: boolean; text: string; bg: string }>("focusColors", { on: false, text: "#343434", bg: "#EDEBE8" });
  const [recentColors, setRecentColors] = usePref<string[]>("recentColors", []);
  const [picker, setPicker] = useState<null | "text" | "bg">(null);
  const [design, setDesign] = useState<{ files: DesignFileFonts[]; activated: number } | null>(null);
  const [artBusy, setArtBusy] = useState(false);
  const [artMenu, setArtMenu] = useState(false);
  const [detail, setDetail] = useState<{ key: string; id: string; scroll: number } | null>(null);
  const [pad, setPad] = usePref("pad", 1);
  const [caps, setCaps] = usePref("caps", false);
  const [allText, setAllText] = usePref("allText", "Aa Bb Cc");
  const [focus, setFocus] = useState(false);
  const [align, setAlign] = usePref<"left" | "center">("align", "left");
  const [external, setExternal] = useState<FontRecord[] | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [adobeRange, setAdobeRange] = useState<"today" | "week">("today");

  const [coll, setColl] = useState<string>(ALL);
  const [target, setTarget] = useState<string>("");
  const [sel, setSel] = useState<string[]>([]);
  const [primary, setPrimary] = useState<string | null>(null);
  const [anchor, setAnchor] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<"name" | "recent">("name");
  const [filters, setFilters] = useState<Filters>({});

  const [ctx, setCtx] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameVal, setRenameVal] = useState("");
  const [tagCtx, setTagCtx] = useState<string | null>(null);
  const [tagging, setTagging] = useState(false);
  const [tagVal, setTagVal] = useState("");

  const [modal, setModal] = useState<Modal>(null);
  const [pending, setPending] = useState<string[]>([]);
  const [importName, setImportName] = useState("");
  const [importOrigin, setImportOrigin] = useState("Envato");
  const [customOrigin, setCustomOrigin] = useState("");
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<{ report: ImportReport; collection: string | null; origin: string; dropped: string } | null>(null);
  const [startup, setStartup] = useState(true);

  const [flash, setFlash] = useState<{ msg: string; undo: boolean } | null>(null);
  const [history, setHistory] = useState<UndoEntry[]>([]);
  const [fileDrag, setFileDrag] = useState(false);
  const [ghost, setGhost] = useState<{ x: number; y: number; label: string } | null>(null);
  const [dropOn, setDropOn] = useState<string | null>(null);

  const libRef = useRef(lib);
  libRef.current = lib;
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const flashTimer = useRef<number | undefined>(undefined);
  const saveTimer = useRef<number | undefined>(undefined);
  const tagTimer = useRef<number | undefined>(undefined);
  const dragRef = useRef<{ x: number; y: number; keys: string[]; active: boolean } | null>(null);
  const suppressClick = useRef(false);

  // ---------- helpers ----------
  const say = useCallback((msg: string, undo = false) => {
    setFlash({ msg, undo });
    window.clearTimeout(flashTimer.current);
    // Long messages (errors, notes) stay up long enough to read.
    flashTimer.current = window.setTimeout(() => setFlash(null), Math.max(undo ? 6000 : 2800, msg.length * 55));
  }, []);

  /** Runs backend calls one after another so saves never overtake each other. */
  const run = useCallback(
    <T,>(fn: () => Promise<T>): Promise<T | undefined> => {
      const p = queue.current.then(fn).catch((e) => {
        say(errorText(e));
        return undefined;
      });
      queue.current = p;
      return p;
    },
    [say],
  );

  const persist = useCallback(
    (next: Library, debounce = false) => {
      window.clearTimeout(saveTimer.current);
      if (debounce) {
        saveTimer.current = window.setTimeout(() => run(() => api.saveLibrary(next)), 600);
      } else {
        run(() => api.saveLibrary(next));
      }
    },
    [run],
  );

  /** Edits collections/tags/favourites, saves, and makes the change undoable. */
  const commit = useCallback(
    (mutate: (l: Library) => void, msg?: string, undoable = true) => {
      const before = cloneLibrary(libRef.current);
      const next = cloneLibrary(libRef.current);
      mutate(next);
      setLib(next);
      persist(next);
      if (undoable) setHistory((h) => [...h.slice(-29), { kind: "meta", snapshot: before }]);
      if (msg) say(msg, undoable);
    },
    [persist, say],
  );

  const undo = useCallback(async () => {
    const last = history[history.length - 1];
    if (!last) {
      say("Nothing to undo");
      return;
    }
    setHistory((h) => h.slice(0, -1));
    if (last.kind === "meta") {
      setLib(last.snapshot);
      persist(last.snapshot);
    } else {
      const restored = await run(() => api.restoreFonts(last.removed, last.snapshot));
      if (restored) setLib(restored);
    }
    say("Undone");
  }, [history, persist, run, say]);

  // ---------- load ----------
  useEffect(() => {
    api
      .getState()
      .then((s) => {
        setLib(s.library);
        setLibraryDir(s.libraryDir);
        setVersion(s.version);
        if (s.library.origins[0]) setImportOrigin(s.library.origins[0]);
        setLoaded(true);
      })
      .catch((e) => {
        say(errorText(e));
        setLoaded(true);
      });
    autostartEnabled()
      .then((on) => {
        // Start with Windows is on by default: it re-activates fonts after a restart.
        if (!on && localStorage.getItem("bfm.startupSet") === null) {
          enableAutostart().then(() => setStartup(true)).catch(() => setStartup(false));
          localStorage.setItem("bfm.startupSet", "1");
        } else setStartup(on);
      })
      .catch(() => setStartup(false));
  }, [say]);

  // System and Adobe fonts are listed in the background; the list arrives when ready.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const load = () => api.getExternal().then((x) => x && setExternal(x)).catch(() => {});
    listen("external-fonts", load).then((u) => (unlisten = u)).catch(() => {});
    load();
    api.getSettings().then(setSettings).catch(() => {});
    return () => unlisten?.();
  }, []);

  useEffect(() => setPvSize(size), [size]);

  useEffect(() => {
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  }, [dark]);

  // ---------- files dropped from Windows ----------
  const openImport = useCallback(
    (paths: string[]) => {
      if (!paths.length) return;
      const designs = paths.filter(isDesignFile);
      if (designs.length && designs.length < paths.length) {
        say("Drop fonts or Illustrator files, not both at once. Nothing was imported.");
        return;
      }
      if (designs.length) {
        inspectRef.current(designs);
        return;
      }
      setPending(paths);
      setImportName(coll !== ALL ? coll : "");
      setCustomOrigin("");
      setModal("import");
    },
    [coll],
  );

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((e) => {
        const p = e.payload;
        if (p.type === "enter" || p.type === "over") setFileDrag(true);
        else if (p.type === "leave") setFileDrag(false);
        else if (p.type === "drop") {
          setFileDrag(false);
          openImport(p.paths);
        }
      })
      .then((u) => (unlisten = u))
      .catch(() => {});
    return () => unlisten?.();
  }, [openImport]);

  // ---------- Illustrator files: which fonts they use ----------
  const inspectRef = useRef<(paths: string[]) => void>(() => {});
  /** Finds the font a file asks for: by PostScript name, then by family and style. */
  function matchUsed(u: { postscript: string; family: string | null; style: string | null }): FontRecord | undefined {
    const ps = u.postscript.toLowerCase().replace(/\s+/g, "");
    const byPs = allFonts.find((f) => f.postscriptName.toLowerCase().replace(/\s+/g, "") === ps);
    if (byPs) return byPs;
    if (u.family) {
      const fam = u.family.toLowerCase();
      const st = (u.style ?? "Regular").toLowerCase();
      return allFonts.find((f) => f.family.toLowerCase() === fam && f.style.toLowerCase() === st);
    }
    return undefined;
  }
  inspectRef.current = async (paths: string[]) => {
    say(paths.length === 1 ? "Reading the Illustrator file…" : `Reading ${paths.length} Illustrator files…`);
    const files = await run(() => api.inspectDesignFiles(paths));
    if (!files) return;
    // Turn on the fonts that are in the library but switched off.
    const off = new Set<string>();
    for (const f of files) for (const u of f.fonts) {
      const m = matchUsed(u);
      if (m && !m.external && !m.active) off.add(m.id);
    }
    if (off.size) await setActive([...off], true);
    setDesign({ files, activated: off.size });
    setModal("design");
  };

  async function generateArtwork(format: "ai" | "pdf") {
    setArtMenu(false);
    if (!inColl || artBusy) return;
    const ext = format === "ai" ? "ai" : "pdf";
    const path = await saveDialog({
      defaultPath: `Buni Font Manager_${coll.replace(/[\\/:*?"<>|]/g, "-")}.${ext}`,
      filters: [format === "ai" ? { name: "Illustrator", extensions: ["ai"] } : { name: "PDF", extensions: ["pdf"] }],
    });
    if (!path) return;
    setArtBusy(true);
    say(format === "ai" ? "Building the artwork in Illustrator… (Illustrator opens if it isn't running)" : "Generating the artwork…");
    const res = await run(() => api.generateArtwork(coll, path, format));
    setArtBusy(false);
    if (!res) return;
    if (format === "ai") {
      // Illustrator may have turned on fonts from this collection.
      api.getState().then((st) => setLib(st.library)).catch(() => {});
    }
    const notes = [];
    if (res.outlined.length) notes.push(`${plural(res.outlined.length)} drawn as outlines (their licence doesn't allow embedding)`);
    if (res.failed.length) notes.push(`${plural(res.failed.length)} couldn't be read`);
    if (res.missing.length) notes.push(`Illustrator didn't find ${res.missing.join(", ")}`);
    say(`Artwork saved: ${res.pages === 1 ? "1 page" : `${res.pages} pages`}${notes.length ? ". " + notes.join(". ") : ""}`);
    if (format === "pdf") openPath(path).catch(() => {});
  }

  async function pickFiles() {
    const picked = await openDialog({ multiple: true, filters: [{ name: "Fonts and zips", extensions: ["zip", "otf", "ttf"] }] });
    if (!picked) return;
    openImport(Array.isArray(picked) ? picked : [picked]);
  }

  async function runImport() {
    const origin = importOrigin === "__new" ? customOrigin.trim() || "Other" : importOrigin;
    const name = importName.trim();
    setBusy(true);
    const res = await run(() => api.importPaths(pending, name || null, origin));
    setBusy(false);
    if (!res) return;
    setLib(res.library);
    setReport({ report: res.report, collection: name || null, origin, dropped: describePending(pending) });
    setModal("report");
    if (name) {
      setColl(name);
      setSel([]);
    }
  }

  // ---------- derived ----------
  const allFonts = useMemo(() => {
    if (!external?.length) return lib.fonts;
    const meta = lib.externalMeta ?? {};
    const ext = external.map((f) => (meta[f.id] ? { ...f, ...meta[f.id] } : f));
    return [...lib.fonts, ...ext];
  }, [lib.fonts, lib.externalMeta, external]);
  const families = useMemo(() => buildFamilies(allFonts), [allFonts]);
  const famByKey = useMemo(() => new Map(families.map((f) => [f.key, f])), [families]);
  const collection = lib.collections.find((c) => c.name === coll) ?? null;
  const inColl = coll !== ALL && !!collection;
  const groups = useMemo(() => filterGroups(families), [families]);
  const collNames = lib.collections.map((c) => c.name);

  useEffect(() => {
    if (coll !== ALL && !collNames.includes(coll)) setColl(ALL);
    if (!collNames.includes(target)) setTarget(collNames[0] ?? "");
  }, [coll, target, collNames]);

  const isFav = (f: Family) => !!collection && f.ids.some((id) => collection.favourites.includes(id));

  const shown = useMemo(() => {
    const q = search.trim().toLowerCase();
    let list = families.filter((f) => {
      if (inColl && !f.ids.some((id) => collection!.fontIds.includes(id))) return false;
      if (q && !f.family.toLowerCase().includes(q)) return false;
      return matchesFilters(f, groups, filters);
    });
    list = [...list].sort((a, b) => (sort === "recent" ? b.addedAt - a.addedAt : a.family.localeCompare(b.family)));
    if (inColl) list = [...list.filter(isFav), ...list.filter((f) => !isFav(f))];
    return list;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [families, inColl, collection, search, sort, filters, groups]);

  const selected = sel.map((k) => famByKey.get(k)).filter((f): f is Family => !!f);
  const detailFam = detail ? famByKey.get(detail.key) ?? null : null;
  const primaryFam = (primary && famByKey.get(primary)) || selected[0] || null;
  const selectedIds = selected.flatMap((f) => f.ids);

  const text = inColl ? collection!.sampleText ?? collection!.name : allText;
  const previewText = text.length > 50 ? text.slice(0, 50).trimEnd() + "…" : text;
  const cellH = `calc(var(--pv-size) * ${sameWidth ? 1.6 : 1.3} + (var(--pv-size) * 0.4 + 60px) * ${pad})`;
  const padV = Math.round(22 * pad);
  const padH = Math.round(24 * pad);
  // The left inset stays put; the Padding slider only changes the space above, below and to the right.
  const padL = 24;
  const padB = Math.round(14 * pad);
  const gap = Math.max(4, Math.round(14 * pad));

  // Every preview is measured once at 100px (widths scale with the size), so moving
  // the Size slider never re-measures. Measured again only when text or fonts change.
  const measureRef = useRef<HTMLDivElement>(null);
  const [widths, setWidths] = useState<Map<string, number>>(new Map());
  const measureKey = `${previewText}|${caps}|${shown.map((f) => previewFont(f, filters).id).join(",")}`;
  const measure = useCallback(() => {
    const el = measureRef.current;
    if (!el) return;
    const m = new Map<string, number>();
    el.querySelectorAll<HTMLElement>("span[data-k]").forEach((sp) => m.set(sp.dataset.k!, sp.getBoundingClientRect().width));
    setWidths(m);
  }, []);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useLayoutEffect(measure, [measureKey]);
  useEffect(() => {
    const fonts = document.fonts;
    let t: number | undefined;
    const later = () => {
      window.clearTimeout(t);
      t = window.setTimeout(measure, 150);
    };
    fonts.addEventListener("loadingdone", later);
    fonts.ready.then(measure).catch(() => {});
    return () => fonts.removeEventListener("loadingdone", later);
  }, [measure]);
  // Card width follows a typical preview (70% of previews fit, plus some air), so one very
  // wide font doesn't stretch every card; wider previews end in "…". Cards then fill the row.
  const mainRef = useRef<HTMLElement>(null);
  const [mainW, setMainW] = useState(1000);
  useEffect(() => {
    const el = mainRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setMainW(el.clientWidth - 48));
    ro.observe(el);
    return () => ro.disconnect();
  }, [loaded]);
  const { medianW, typicalRaw } = useMemo(() => {
    const v = shown.map((f) => widths.get(f.key)).filter((x): x is number => !!x).sort((a, b) => a - b);
    if (!v.length) return { medianW: 0, typicalRaw: 0 };
    return { medianW: v[Math.floor(v.length / 2)], typicalRaw: v[Math.min(v.length - 1, Math.floor(v.length * 0.7))] };
  }, [shown, widths]);
  const sidePad = focus ? padL + 52 + 4 : padL + padH + 4;
  const typicalW = Math.min(Math.ceil(((sameWidth ? medianW : typicalRaw) * size) / 100 * (sameWidth ? 1 : 1.08)), Math.max(60, mainW - sidePad));
  const cellW = Math.max(focus ? 140 : 230, typicalW + sidePad);
  /** In same-width mode every preview is scaled to exactly the text width of a card (capped so condensed fonts don't explode). */
  const scaleOf = (fam: Family) => {
    if (!sameWidth) return 1;
    const w = widths.get(fam.key);
    if (!w || !medianW) return 1;
    const target = typicalW / ((medianW * size) / 100);
    return Math.min(2.2 * target, (medianW / w) * target);
  };

  const faces = useMemo(
    () =>
      allFonts
        .map((f) => {
          // System and Adobe fonts are installed already, so the installed copy is used first.
          const local = f.external ? `local("${f.postscriptName.replace(/"/g, "")}")` : "";
          const ttc = /\.(ttc|otc)$/i.test(f.path) || (f.index ?? 0) > 0;
          const url = ttc ? "" : `url("${convertFileSrc(f.path)}")`;
          const src = [local, url].filter(Boolean).join(",");
          return src ? `@font-face{font-family:"${fontFaceName(f.id)}";src:${src};font-display:block;}` : "";
        })
        .join("\n"),
    [allFonts],
  );
  const anyFilter = Object.values(filters).some(Boolean);

  // ---------- selection ----------
  function pick(fam: Family, e: React.MouseEvent) {
    if (suppressClick.current) {
      suppressClick.current = false;
      return;
    }
    const order = shown.map((f) => f.key);
    if (e.ctrlKey || e.metaKey) {
      setSel((s) => (s.includes(fam.key) ? s.filter((k) => k !== fam.key) : [...s, fam.key]));
      setPrimary(fam.key);
      setAnchor(fam.key);
    } else if (e.shiftKey && anchor && order.includes(anchor)) {
      const a = order.indexOf(anchor);
      const b = order.indexOf(fam.key);
      setSel(order.slice(Math.min(a, b), Math.max(a, b) + 1));
      setPrimary(fam.key);
    } else {
      setSel([fam.key]);
      setPrimary(fam.key);
      setAnchor(fam.key);
      setCtx(null);
      setTagCtx(null);
      setTagging(false);
    }
  }

  /** Double-click on a preview: open the font page. */
  function openDetail(fam: Family) {
    setDetail({ key: fam.key, id: previewFont(fam, filters).id, scroll: mainRef.current?.scrollTop ?? 0 });
    setSel([fam.key]);
    setPrimary(fam.key);
    mainRef.current?.scrollTo({ top: 0 });
  }
  function closeDetail() {
    const back = detail?.scroll ?? 0;
    setDetail(null);
    requestAnimationFrame(() => mainRef.current?.scrollTo({ top: back }));
  }

  function clearSel() {
    setSel([]);
    setCtx(null);
    setTagCtx(null);
  }

  // ---------- collection edits ----------
  function addToCollection(name: string, ids: string[]) {
    const c = libRef.current.collections.find((x) => x.name === name);
    const fresh = ids.filter((id) => !c?.fontIds.includes(id));
    if (!fresh.length) {
      say(`Already in ${name}`);
      return;
    }
    const n = new Set(fresh.map((id) => familyOf(id))).size;
    commit((l) => {
      const col = l.collections.find((x) => x.name === name)!;
      col.fontIds.push(...fresh);
    }, `Added ${plural(n)} to ${name}`);
  }

  function familyOf(id: string) {
    const f = libRef.current.fonts.find((x) => x.id === id) ?? external?.find((x) => x.id === id);
    return f ? `${f.origin}|${f.family}` : id;
  }

  function removeFromCollection(fams: Family[]) {
    if (!inColl || !fams.length) return;
    const ids = fams.flatMap((f) => f.ids);
    commit((l) => {
      const col = l.collections.find((x) => x.name === coll)!;
      col.fontIds = col.fontIds.filter((id) => !ids.includes(id));
      col.favourites = col.favourites.filter((id) => !ids.includes(id));
    }, `Removed ${plural(fams.length)} from ${coll}`);
    setSel([]);
  }

  function toggleFav(fam: Family) {
    if (!inColl) return;
    const on = isFav(fam);
    commit(
      (l) => {
        const col = l.collections.find((x) => x.name === coll)!;
        col.favourites = on ? col.favourites.filter((id) => !fam.ids.includes(id)) : [...col.favourites, ...fam.ids.filter((id) => !col.favourites.includes(id))];
      },
      undefined,
      false,
    );
  }

  function newCollection() {
    let name = "New collection";
    for (let n = 2; collNames.includes(name); n++) name = `New collection ${n}`;
    commit((l) => l.collections.push({ name, fontIds: [], favourites: [], sampleText: null }), undefined, false);
    setColl(name);
    setSel([]);
    setRenaming(name);
    setRenameVal(name);
  }

  function commitRename() {
    const old = renaming;
    setRenaming(null);
    if (!old) return;
    const val = renameVal.trim();
    if (!val || val === old || collNames.includes(val)) return;
    commit((l) => {
      const c = l.collections.find((x) => x.name === old);
      if (c) {
        c.name = val;
      }
    }, undefined, false);
    if (coll === old) setColl(val);
    if (target === old) setTarget(val);
  }

  function deleteCollection(name: string) {
    setCtx(null);
    commit((l) => {
      l.collections = l.collections.filter((c) => c.name !== name);
    }, `Deleted the collection ${name}. Its fonts stay in your library.`);
    if (coll === name) setColl(ALL);
  }

  async function setActive(ids: string[], on: boolean, msg?: string) {
    const res = await run(() => api.setActive(ids, on));
    if (res) setLib(res);
    if (msg) say(msg);
  }

  async function onlyThis(name: string) {
    setCtx(null);
    const c = libRef.current.collections.find((x) => x.name === name);
    if (!c) return;
    const others = libRef.current.fonts.filter((f) => !c.fontIds.includes(f.id) && f.active).map((f) => f.id);
    await setActive(c.fontIds, true);
    await setActive(others, false, `Only ${name} is active now`);
  }

  async function deleteFromLibrary(fams: Family[]) {
    fams = fams.filter((f) => !f.rep.external);
    const ids = fams.flatMap((f) => f.ids);
    if (!ids.length) return;
    const snapshot = cloneLibrary(libRef.current);
    const res = await run(() => api.deleteFonts(ids));
    if (!res) return;
    setLib(res.library);
    setHistory((h) => [...h.slice(-29), { kind: "delete", snapshot, removed: res.removed }]);
    setSel([]);
    say(`Deleted ${plural(fams.length)} from the library`, true);
  }

  // ---------- tags ----------
  function addTag() {
    const val = tagVal.trim();
    setTagging(false);
    setTagVal("");
    if (!val || !selectedIds.length) return;
    commit((l) => {
      editTags(l, selectedIds, (f) => {
        if (!f.customTags.includes(val)) f.customTags.push(val);
      });
    }, `Added the tag ${val}${selected.length > 1 ? ` to ${plural(selected.length)}` : ""}`, false);
  }

  function removeTag(label: string) {
    setTagCtx(null);
    const fams = selected.filter((f) => tagList(f).some((t) => t.label === label && !t.fixed));
    const ids = fams.flatMap((f) => f.ids);
    commit((l) => {
      editTags(l, ids, (f) => {
        f.customTags = f.customTags.filter((t) => t !== label);
        if (f.aiTags.includes(label) && !f.hiddenAiTags.includes(label)) f.hiddenAiTags.push(label);
      });
    }, `Removed the tag ${label}${fams.length > 1 ? ` from ${plural(fams.length)}` : ""}`);
  }

  // ---------- Adobe Fonts added recently ----------
  function recentAdobe(range: "today" | "week"): Family[] {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    if (range === "week") start.setDate(start.getDate() - 6);
    const since = start.getTime() / 1000;
    return families.filter((f) => f.origin === ADOBE && f.addedAt >= since).sort((a, b) => b.addedAt - a.addedAt);
  }

  function addRecentAdobe() {
    const fams = recentAdobe(adobeRange);
    setModal(null);
    if (!fams.length || !inColl) return;
    addToCollection(coll, fams.flatMap((f) => f.ids));
  }

  async function changeAdobeDir(dir: string | null) {
    const res = await run(() => api.setAdobeDir(dir));
    if (res) {
      setSettings(res);
      say(res.adobeDirExists ? "Adobe Fonts folder updated" : "That folder doesn't exist. Adobe Fonts won't be listed until it does.");
    }
  }

  async function rescan() {
    say("Looking for system and Adobe fonts…");
    const res = await run(() => api.rescanExternal());
    if (res) {
      setExternal(res);
      say(`Found ${res.length} system and Adobe font files`);
    }
  }

  // ---------- sample text ----------
  function onText(v: string) {
    if (inColl) {
      const next = cloneLibrary(libRef.current);
      const c = next.collections.find((x) => x.name === coll);
      if (c) c.sampleText = v;
      setLib(next);
      persist(next, true);
    } else setAllText(v);
  }

  // ---------- dragging fonts onto collections ----------
  function onCardPointerDown(fam: Family, e: React.PointerEvent) {
    if (e.button !== 0 || renaming) return;
    const keys = sel.includes(fam.key) ? sel : [fam.key];
    dragRef.current = { x: e.clientX, y: e.clientY, keys, active: false };
  }

  useEffect(() => {
    function move(e: PointerEvent) {
      const d = dragRef.current;
      if (!d) return;
      if (!d.active && Math.hypot(e.clientX - d.x, e.clientY - d.y) > 6) d.active = true;
      if (!d.active) return;
      setGhost({ x: e.clientX + 14, y: e.clientY + 14, label: d.keys.length === 1 ? famByKey.get(d.keys[0])?.family ?? "1 font" : plural(d.keys.length) });
      const el = document.elementFromPoint(e.clientX, e.clientY)?.closest("[data-coll]") as HTMLElement | null;
      setDropOn(el?.dataset.coll ?? null);
    }
    function up(e: PointerEvent) {
      const d = dragRef.current;
      dragRef.current = null;
      if (!d?.active) return;
      suppressClick.current = true;
      window.setTimeout(() => (suppressClick.current = false), 50);
      setGhost(null);
      setDropOn(null);
      const el = document.elementFromPoint(e.clientX, e.clientY)?.closest("[data-coll]") as HTMLElement | null;
      const name = el?.dataset.coll;
      if (name && name !== ALL) {
        const ids = d.keys.flatMap((k) => famByKey.get(k)?.ids ?? []);
        if (!sel.includes(d.keys[0])) setSel(d.keys);
        addToCollection(name, ids);
      }
    }
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  });

  // ---------- keyboard ----------
  useEffect(() => {
    function key(e: KeyboardEvent) {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        e.preventDefault();
        undo();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a" && !modal) {
        e.preventDefault();
        setSel(shown.map((f) => f.key));
      } else if (e.key === "Escape" && detail && !modal) {
        closeDetail();
      } else if (e.key === "Escape") {
        clearSel();
        setModal((m) => (m === "import" || m === "settings" || m === "report" || m === "adobe" || m === "design" ? null : m));
      } else if ((e.key === "Delete" || e.key === "Backspace") && !modal && inColl && selected.length) {
        e.preventDefault();
        removeFromCollection(selected);
      }
    }
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  });

  // Clicking anywhere outside a tag menu closes it.
  useEffect(() => {
    if (!tagCtx) return;
    function close(e: MouseEvent) {
      if ((e.target as HTMLElement)?.closest?.('[role="menu"]')) return;
      setTagCtx(null);
    }
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [tagCtx]);

  function onMainClick(e: React.MouseEvent) {
    if ((e.target as HTMLElement).closest("article, [data-font-cell], button, input, select, label, [role=menu], [role=status]")) return;
    clearSel();
  }

  // ---------- render pieces ----------
  const previewStyle = (f: FontRecord, scale = 1): React.CSSProperties => ({
    fontFamily: `"${fontFaceName(f.id)}", "Segoe UI", sans-serif`,
    fontSize: scale === 1 ? "var(--pv-size)" : `calc(var(--pv-size) * ${scale.toFixed(3)})`,
    textAlign: align,
    textTransform: caps ? "uppercase" : "none",
  });

  function famSub(f: Family) {
    return `${f.origin} · ${f.fonts.length === 1 ? "1 style" : `${f.fonts.length} styles`}`;
  }

  function ActiveSwitch({ fam }: { fam: Family }) {
    if (fam.rep.external) return <span className="icon44" title={fam.origin === ADOBE ? "Turned on and off in Adobe Creative Cloud" : "Installed in Windows: always available"} />;
    return (
      <button
        type="button"
        className="icon44"
        aria-label={`${fam.active ? "Deactivate" : "Activate"} ${fam.family}`}
        aria-pressed={fam.active}
        title={fam.active ? "Active: available in Illustrator and other apps" : "Inactive"}
        onClick={() => setActive(fam.ids, !fam.active)}
      >
        <span className={`switch${fam.active ? " on" : ""}`}>
          <i />
        </span>
      </button>
    );
  }

  const sourcesCount = families.length;
  const hint = inColl ? "Ctrl/Shift-click to select, drag onto a collection, Del removes" : "Ctrl/Shift-click to select several, drag onto a collection";

  if (!loaded) return <div className="app" />;

  return (
    <div className="app">
      <style>{faces}</style>
      <div ref={measureRef} aria-hidden="true" style={{ position: "absolute", left: -100000, top: 0, visibility: "hidden", whiteSpace: "nowrap" }}>
        {shown.map((fam) => (
          <span key={fam.key} data-k={fam.key} style={{ ...previewStyle(previewFont(fam, filters)), fontSize: 100, textAlign: "left", display: "inline-block", lineHeight: 1.15 }}>
            {previewText}
          </span>
        ))}
      </div>

      <header className="header">
        <div className="brand">
          <Logo />
          <span>Buni Font Manager</span>
        </div>
        <input className="sample" type="text" aria-label="Sample text" placeholder="Type your sample text" value={text} onChange={(e) => onText(e.target.value)} />
        <div className="sliders">
          <Slider id="size" label={(v) => (sameWidth ? "Width" : `Size ${v}`)} min={20} max={240} value={size} onLive={setPvSize} onCommit={setSize} />
          <Slider id="pad" label={(v) => `Padding ×${v.toFixed(1)}`} min={0.2} max={1.6} step={0.1} value={pad} onCommit={setPad} />
        </div>
        <button type="button" className="caps" aria-pressed={caps} aria-label="Show previews in capitals" title="All caps" onClick={() => setCaps(!caps)}>
          {caps ? "AA" : "Aa"}
        </button>
        <button type="button" className="caps" aria-pressed={sameWidth} aria-label="Same width: scale every preview to the same width" title="Same width" onClick={() => setSameWidth(!sameWidth)}>
          <IconWidth />
        </button>
        <div className="seg">
          <button type="button" aria-label="Align previews left" title="Align left" aria-pressed={align === "left"} onClick={() => setAlign("left")}>
            <IconAlignLeft />
          </button>
          <button type="button" aria-label="Centre previews" title="Centre" aria-pressed={align === "center"} onClick={() => setAlign("center")}>
            <IconAlignCenter />
          </button>
        </div>
        {focus && (
          <div className="colors">
            {(["text", "bg"] as const).map((k) => (
              <button
                key={k}
                type="button"
                data-swatch
                className="color-btn"
                aria-expanded={picker === k}
                aria-label={k === "text" ? "Text colour" : "Background colour"}
                title={k === "text" ? "Text colour" : "Background colour"}
                onClick={() => setPicker(picker === k ? null : k)}
              >
                <span className="swatch" style={{ background: colors.on ? colors[k] : k === "text" ? "var(--ink)" : "var(--bg)" }} />
                <small>{k === "text" ? "Aa" : "Bg"}</small>
              </button>
            ))}
            <button type="button" className="icon-round" aria-label="Swap text and background colours" title="Swap" onClick={() => {
              const c = colors.on ? colors : themeColors();
              setColors({ on: true, text: c.bg, bg: c.text });
            }}>
              <IconSwap size={16} />
            </button>
            {colors.on && (
              <button type="button" className="icon-round" aria-label="Back to the default colours" title="Default colours" onClick={() => { setColors({ ...colors, on: false }); setPicker(null); }}>
                <IconReset size={16} />
              </button>
            )}
            {picker && (
              <ColorPicker
                label={picker === "text" ? "Text colour" : "Background colour"}
                value={(colors.on ? colors : themeColors())[picker]}
                recent={recentColors}
                onChange={(hex) => {
                  const base = colors.on ? colors : themeColors();
                  setColors({ ...base, on: true, [picker]: hex });
                }}
                onPick={(hex) => setRecentColors([hex, ...recentColors.filter((c) => c !== hex)].slice(0, 10))}
                onReset={() => { setColors({ ...colors, on: false }); setPicker(null); }}
                onClose={() => setPicker(null)}
              />
            )}
          </div>
        )}
        <div className="seg" hidden={focus}>
          <button type="button" aria-label="Grid view" aria-pressed={view === "grid"} onClick={() => setView("grid")}>
            <IconGrid />
          </button>
          <button type="button" aria-label="List view" aria-pressed={view === "list"} onClick={() => setView("list")}>
            <IconList />
          </button>
        </div>
        <button type="button" className="btn" onClick={pickFiles}>
          <IconUpload size={16} /> Import fonts
        </button>
        <div style={{ display: "flex", gap: 8 }}>
          <button type="button" className="round" aria-label="Focus mode: compare fonts only" title="Focus mode" aria-pressed={focus} onClick={() => setFocus(!focus)}>
            <IconFocus />
          </button>
          <button type="button" className="round" aria-label="Toggle dark mode" title="Dark mode" onClick={() => setDark(!dark)}>
            {dark ? <IconSun /> : <IconMoon />}
          </button>
          <button type="button" className="round" aria-label="Settings" title="Settings" onClick={() => setModal("settings")}>
            <IconGear />
          </button>
        </div>
      </header>

      <div className="body">
        {!focus && (
          <nav className="side" aria-label="Library">
            <div className="search">
              <input type="search" aria-label="Search fonts by name" placeholder="Search fonts" value={search} onChange={(e) => setSearch(e.target.value)} />
              <IconSearch size={16} />
            </div>

            <section className="colls">
              <div className="colls-head">
                <h2>Collections</h2>
                <button type="button" className="icon-btn" aria-label="New collection" title="New collection" onClick={newCollection}>
                  <IconPlus size={16} />
                </button>
              </div>
              {[ALL, ...collNames].map((name) => {
                const count = name === ALL ? sourcesCount : new Set((lib.collections.find((c) => c.name === name)?.fontIds ?? []).map(familyOf)).size;
                const picked = coll === name;
                return (
                  <div key={name} style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    {renaming === name ? (
                      <input
                        className="rename"
                        aria-label="Collection name"
                        autoFocus
                        value={renameVal}
                        onChange={(e) => setRenameVal(e.target.value)}
                        onBlur={commitRename}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") commitRename();
                          if (e.key === "Escape") setRenaming(null);
                        }}
                      />
                    ) : (
                      <div className={`coll${dropOn === name && name !== ALL ? " drop" : picked ? " picked" : ""}`} data-coll={name}>
                        <button
                          type="button"
                          onClick={() => {
                            setColl(name);
                            setSel([]);
                            setCtx(null);
                          }}
                          onContextMenu={(e) => {
                            if (name === ALL) return;
                            e.preventDefault();
                            setCtx(name);
                          }}
                        >
                          <span className="name">{name}</span>
                          <span className="count">{count}</span>
                        </button>
                        {name !== ALL && (
                          <button type="button" className="icon-btn" aria-label={`Options for ${name}`} aria-expanded={ctx === name} onClick={() => setCtx(ctx === name ? null : name)}>
                            <IconDots />
                          </button>
                        )}
                      </div>
                    )}
                    {ctx === name && (
                      <div className="menu" role="menu" aria-label={`${name} options`}>
                        <button type="button" role="menuitem" onClick={() => { setCtx(null); setRenaming(name); setRenameVal(name); }}>Rename</button>
                        <button type="button" role="menuitem" onClick={() => { setCtx(null); const c = lib.collections.find((x) => x.name === name); if (c) setActive(c.fontIds, true, `Activated the fonts in ${name}`); }}>Activate all fonts</button>
                        <button type="button" role="menuitem" onClick={() => { setCtx(null); const c = lib.collections.find((x) => x.name === name); if (c) setActive(c.fontIds, false, `Deactivated the fonts in ${name}`); }}>Deactivate all fonts</button>
                        <button type="button" role="menuitem" onClick={() => onlyThis(name)}>Deactivate everything else</button>
                        <hr />
                        <button type="button" role="menuitem" onClick={() => deleteCollection(name)}>
                          <IconTrash size={15} /> Delete collection
                        </button>
                      </div>
                    )}
                  </div>
                );
              })}
            </section>

            {anyFilter && (
              <button type="button" className="clear-filters" onClick={() => setFilters({})}>
                <IconX size={14} /> Clear filters
              </button>
            )}
            {groups.map((g) => (
              <section key={g.key} style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <h2 style={{ padding: "0 8px" }}>{g.label}</h2>
                <div className="chips">
                  {g.chips.map((c) => {
                    const k = `${g.key}:${c}`;
                    return (
                      <button key={k} type="button" className="chip-btn" aria-pressed={!!filters[k]} onClick={() => setFilters({ ...filters, [k]: !filters[k] })}>
                        {c}
                      </button>
                    );
                  })}
                </div>
              </section>
            ))}

            <button type="button" className="credit" onClick={() => openUrl("https://andersonbunidesign.com")}>
              <span>Vibe coded by Anderson Santos</span>
              <span>Version {version}</span>
            </button>
          </nav>
        )}

        <main
          ref={mainRef}
          className={`main${focus && colors.on ? " custom" : ""}`}
          style={focus && colors.on ? ({ "--focus-bg": colors.bg, "--focus-ink": colors.text } as React.CSSProperties) : undefined}
          onClick={onMainClick}
        >
          {!focus && !detailFam && (
            <div className="titlebar">
              <div>
                <p className="kicker">Collection</p>
                <h1>{coll}</h1>
                <div className="meta">
                  <span>
                    {plural(shown.length)}
                    {inColl ? " · favourites first" : ""} · {hint}
                  </span>
                  <label htmlFor="sort" style={{ fontWeight: 600, fontSize: 12 }}>Sort</label>
                  <select id="sort" value={sort} onChange={(e) => setSort(e.target.value as "name" | "recent")}>
                    <option value="name">Name</option>
                    <option value="recent">Recently added</option>
                  </select>
                </div>
              </div>
              {inColl && (
                <div style={{ display: "flex", gap: 8, alignSelf: "flex-end", marginLeft: "auto" }}>
                  <button type="button" className="round" aria-label="Add recent Adobe Fonts to this collection" title="Add recent Adobe Fonts" onClick={() => setModal("adobe")}>
                    <IconClock />
                  </button>
                  <div style={{ position: "relative" }}>
                    <button
                      type="button"
                      className="round"
                      aria-label="Generate Artwork"
                      aria-expanded={artMenu}
                      title="Generate Artwork"
                      disabled={artBusy || !collection?.fontIds.length}
                      onClick={() => setArtMenu(!artMenu)}
                    >
                      <IconArtwork />
                    </button>
                    {artMenu && (
                      <div className="menu art-menu" role="menu" aria-label="Generate Artwork" onMouseLeave={() => setArtMenu(false)}>
                        <button type="button" role="menuitem" onClick={() => generateArtwork("ai")}>Illustrator (.ai)</button>
                        <button type="button" role="menuitem" onClick={() => generateArtwork("pdf")}>PDF</button>
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}

          {focus && !inColl && !detailFam && collNames.length > 0 && (
            <div className="target">
              <label htmlFor="target">Add to collection</label>
              <select id="target" value={target} onChange={(e) => setTarget(e.target.value)}>
                {collNames.map((n) => (
                  <option key={n} value={n}>{n}</option>
                ))}
              </select>
            </div>
          )}

          {flash && (
            <div className="flash" role="status">
              <span>{flash.msg}</span>
              {flash.undo && (
                <button type="button" onClick={undo}>Undo (Ctrl+Z)</button>
              )}
            </div>
          )}

          {detailFam ? (
            <FontPage key={detailFam.key} fam={detailFam} initialId={detail!.id} text={text} caps={caps} align={align} pad={pad} onBack={closeDetail} say={say} />
          ) : families.length === 0 ? (
            <p className="empty">Your library is empty. Drop zip, OTF or TTF files anywhere in this window, or click Import fonts.</p>
          ) : shown.length === 0 ? (
            <p className="empty">
              {inColl && !collection!.fontIds.length ? "This collection is empty. Drag fonts onto it, or import with this collection's name." : "No fonts match this search or these filters."}
              {anyFilter && (
                <>
                  {" "}
                  <button type="button" className="link" onClick={() => setFilters({})}>Clear filters</button>
                </>
              )}
            </p>
          ) : view === "list" && !focus ? (
            <div className="list">
              {shown.map((fam) => {
                const isSel = sel.includes(fam.key);
                return (
                  <div key={fam.key} data-font-cell="true" className={`row${isSel ? " sel" : ""}`} style={{ paddingLeft: padL }} onPointerDown={(e) => onCardPointerDown(fam, e)}>
                    <div className="info">
                      <div className="fname">{fam.family}</div>
                      <div className="fsub">{famSub(fam)}</div>
                    </div>
                    <button type="button" className="preview" aria-label={`Select ${fam.family}`} style={{ padding: "6px 0", flex: "1 1 auto", minWidth: 0 }} onClick={(e) => pick(fam, e)} onDoubleClick={() => openDetail(fam)} title="Double-click to see all characters">
                      <span className="pv" style={{ ...previewStyle(previewFont(fam, filters), scaleOf(fam)), lineHeight: 1.2 }}>{previewText}</span>
                    </button>
                    {inColl && (
                      <button type="button" className="icon44" aria-label={`Favourite ${fam.family}`} aria-pressed={isFav(fam)} onClick={() => toggleFav(fam)}>
                        <Star on={isFav(fam)} />
                      </button>
                    )}
                    <ActiveSwitch fam={fam} />
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="grid" style={{ gridTemplateColumns: `repeat(auto-fill, minmax(min(${cellW}px, 100%), 1fr))`, gap }}>
              {shown.map((fam) => {
                const isSel = sel.includes(fam.key);
                const inTarget = !!target && fam.ids.some((id) => lib.collections.find((c) => c.name === target)?.fontIds.includes(id));
                return (
                  <article key={fam.key} className={`card${isSel ? " sel" : ""}${focus ? " focus" : ""}`} onPointerDown={(e) => onCardPointerDown(fam, e)}>
                    <button
                      type="button"
                      className="preview"
                      aria-label={`Select ${fam.family}`}
                      style={{ paddingTop: padV, paddingBottom: focus ? padV : padB, paddingLeft: padL, paddingRight: focus ? 52 : padH, minHeight: cellH }}
                      onClick={(e) => pick(fam, e)}
                      onDoubleClick={() => openDetail(fam)}
                      title="Double-click to see all characters"
                    >
                      <span className="pv" style={previewStyle(previewFont(fam, filters), scaleOf(fam))}>{previewText}</span>
                    </button>
                    {focus ? (
                      inColl ? (
                        <button type="button" className="icon44 corner" aria-label={`Favourite ${fam.family}`} aria-pressed={isFav(fam)} onClick={() => toggleFav(fam)}>
                          <Star on={isFav(fam)} />
                        </button>
                      ) : (
                        target && (
                          <button
                            type="button"
                            className="icon44 corner"
                            aria-label={`Add ${fam.family} to ${target}`}
                            aria-pressed={inTarget}
                            style={{ color: inTarget ? "var(--amber-ink)" : "var(--arrow-off)" }}
                            onClick={() => {
                              if (inTarget) {
                                commit((l) => {
                                  const c = l.collections.find((x) => x.name === target)!;
                                  c.fontIds = c.fontIds.filter((id) => !fam.ids.includes(id));
                                  c.favourites = c.favourites.filter((id) => !fam.ids.includes(id));
                                }, `Removed ${fam.family} from ${target}`);
                              } else addToCollection(target, fam.ids);
                            }}
                          >
                            {inTarget ? <IconCheckCircle size={20} /> : <IconArrowIn size={20} />}
                          </button>
                        )
                      )
                    ) : (
                      <div className="card-foot" style={{ paddingLeft: padL }}>
                        <div className="info">
                          <div className="fname">{fam.family}</div>
                          <div className="fsub">{famSub(fam)}</div>
                        </div>
                        {inColl && (
                          <button type="button" className="icon44" aria-label={`Favourite ${fam.family}`} aria-pressed={isFav(fam)} onClick={() => toggleFav(fam)}>
                            <Star on={isFav(fam)} />
                          </button>
                        )}
                        <ActiveSwitch fam={fam} />
                      </div>
                    )}
                  </article>
                );
              })}
            </div>
          )}

          {!focus && !detailFam && families.length > 0 && <div className="dropzone">Drop zip, OTF or TTF files anywhere in the window to import them, or an Illustrator file to find and turn on its fonts</div>}
        </main>

        {!focus && (
          <aside className="panel" aria-label="Font details">
            {!primaryFam || !selected.length ? (
              <p style={{ margin: 0, paddingTop: 8, fontSize: 13, color: "var(--muted)" }}>No font selected. Click a preview to see its weights and tags.</p>
            ) : (
              <>
                <div>
                  <p className="kicker">{primaryFam.origin}</p>
                  <h2>{primaryFam.family}</h2>
                  {selected.length > 1 && <p style={{ margin: "6px 0 0", fontSize: 12, fontWeight: 600, color: "var(--muted)" }}>{selected.length} fonts selected</p>}
                </div>

                <div className="weights">
                  <h3>Weights</h3>
                  {primaryFam.fonts.map((f) => (
                    <div className="weight" key={f.id}>
                      <small>
                        {f.style} · {weightName(f.weight)} {f.weight}
                      </small>
                      <span style={{ fontFamily: `"${fontFaceName(f.id)}"`, textTransform: caps ? "uppercase" : "none" }}>{previewText}</span>
                    </div>
                  ))}
                </div>

                <h3 style={{ marginBottom: -10 }}>Tags</h3>
                <TagArea
                  selected={selected}
                  tagCtx={tagCtx}
                  setTagCtx={setTagCtx}
                  tagTimer={tagTimer}
                  removeTag={removeTag}
                  tagging={tagging}
                  setTagging={setTagging}
                  tagVal={tagVal}
                  setTagVal={setTagVal}
                  addTag={addTag}
                />

                <dl className="facts">
                  <dt>Languages</dt>
                  <dd>{primaryFam.rep.langs.join(", ") || "Latin basic only"}</dd>
                  <dt>Licence</dt>
                  <dd>
                    {primaryFam.rep.external ? (
                      primaryFam.origin === ADOBE ? "Adobe Fonts licence (your Creative Cloud plan)" : "Licensed with Windows"
                    ) : primaryFam.licence ? (
                      <button type="button" className="link" onClick={() => openPath(primaryFam.licence!)}>
                        {primaryFam.licence.split(/[\\/]/).pop()}
                      </button>
                    ) : (
                      "No licence file was found with this font"
                    )}
                  </dd>
                  {primaryFam.fonts.some((f) => f.noEmbedding) && (
                    <>
                      <dt>Note</dt>
                      <dd>This font's licence doesn't allow embedding it in PDFs.</dd>
                    </>
                  )}
                </dl>

                <div className="actions">
                  <button type="button" className="btn block" title={primaryFam.rep.path} onClick={() => revealItemInDir(primaryFam.rep.path)}>
                    <IconFolder size={16} /> Show in folder
                  </button>
                  {primaryFam.rep.external ? (
                    <p className="muted" style={{ fontSize: 12 }}>
                      {primaryFam.origin === ADOBE ? "Synced by Adobe Fonts. Turn it on or off in the Creative Cloud app." : "Installed in Windows. It's always available, so it can't be deactivated or deleted here."}
                    </p>
                  ) : (
                    <button type="button" className="btn block" onClick={() => setActive(selectedIds, !primaryFam.active)}>
                      {primaryFam.active ? (selected.length > 1 ? "Deactivate fonts" : "Deactivate font") : selected.length > 1 ? "Activate fonts" : "Activate font"}
                    </button>
                  )}
                  {inColl && (
                    <button type="button" className="btn block" onClick={() => removeFromCollection(selected)}>
                      <IconMinusCircle /> Remove from collection (Del)
                    </button>
                  )}
                  {selected.some((f) => !f.rep.external) && (
                    <button type="button" className="btn block" onClick={() => deleteFromLibrary(selected)}>
                      <IconTrash /> Delete from library
                    </button>
                  )}
                </div>
              </>
            )}
          </aside>
        )}
      </div>

      {modal === "import" && (
        <div className="overlay">
          <div className="dialog" role="dialog" aria-label="Import fonts">
            <div>
              <h2>Import fonts</h2>
              <p className="muted" style={{ marginTop: 6 }}>{describePending(pending)} Licence files found inside zips are kept with their fonts.</p>
            </div>
            <div className="field">
              <label htmlFor="impname">Collection</label>
              <input id="impname" list="collnames" value={importName} onChange={(e) => setImportName(e.target.value)} placeholder="Leave empty to import without a collection" />
              <datalist id="collnames">
                {collNames.map((n) => (
                  <option key={n} value={n} />
                ))}
              </datalist>
            </div>
            <div className="field">
              <label htmlFor="imporigin">Origin</label>
              <select id="imporigin" value={importOrigin} onChange={(e) => setImportOrigin(e.target.value)}>
                {Array.from(new Set([...lib.origins, ...DEFAULT_ORIGINS])).map((o) => (
                  <option key={o} value={o}>{o}</option>
                ))}
                <option value="__new">Other…</option>
              </select>
              {importOrigin === "__new" && <input aria-label="New origin" placeholder="Type the origin, e.g. MyFonts" value={customOrigin} onChange={(e) => setCustomOrigin(e.target.value)} autoFocus />}
            </div>
            <div className="dialog-actions">
              <button type="button" className="btn" onClick={() => setModal(null)} disabled={busy}>Cancel</button>
              <button type="button" className="btn primary" onClick={runImport} disabled={busy}>{busy ? "Importing…" : "Import"}</button>
            </div>
          </div>
        </div>
      )}

      {modal === "report" && report && (
        <div className="overlay">
          <div className="dialog" role="dialog" aria-label="Import report">
            <div>
              <h2>Import finished</h2>
              <p className="muted" style={{ marginTop: 6 }}>
                {report.dropped}
                <br />
                {report.collection ? `Added to the collection ${report.collection}` : "Imported without a collection"} · origin {report.origin}
              </p>
            </div>
            <div className="stats">
              <div><b>{report.report.added.length}</b><span>fonts added</span></div>
              <div><b>{report.report.skipped.length}</b><span>skipped, already in library</span></div>
              <div><b>{report.report.errors.length}</b><span>{report.report.errors.length === 1 ? "file with errors" : "files with errors"}</span></div>
            </div>
            {report.report.errors.length > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <h3>Errors</h3>
                {report.report.errors.map((er, i) => (
                  <div className="errline" key={i}>
                    <span style={{ color: "var(--red)", fontWeight: 700 }}>!</span>
                    <span><b>{er.file}</b>: {er.message}</span>
                  </div>
                ))}
              </div>
            )}
            {report.report.skipped.length > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <h3>Skipped</h3>
                <p>{report.report.skipped.map((s) => s.name).join(", ")}{report.collection ? ": added to the collection without copying them again." : "."}</p>
              </div>
            )}
            {(report.report.conflicts?.length ?? 0) > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <h3>Already on this computer</h3>
                <p>
                  {report.report.conflicts!.map((c) => `${c.family} (${c.otherOrigin})`).join(", ")}. Two fonts with the same name can get mixed up in Illustrator.
                </p>
                <div>
                  <button
                    type="button"
                    className="btn"
                    onClick={async () => {
                      const ids = report.report.conflicts!.flatMap((c) => c.ids);
                      const fams = families.filter((f) => f.ids.some((id) => ids.includes(id)));
                      setModal(null);
                      await deleteFromLibrary(fams);
                    }}
                  >
                    <IconTrash size={15} /> Remove imported copies
                  </button>
                </div>
              </div>
            )}
            <p className="muted">
              {report.report.licencesKept === 1 ? "1 licence file" : `${report.report.licencesKept} licence files`} kept and linked to their fonts.
              {report.report.duoTagged > 0 ? ` ${plural(report.report.duoTagged)} tagged Duo (two families in the same zip).` : ""}
            </p>
            <div className="dialog-actions">
              <button type="button" className="btn primary" onClick={() => setModal(null)}>Done</button>
            </div>
          </div>
        </div>
      )}

      {modal === "settings" && (
        <div className="overlay">
          <div className="dialog" role="dialog" aria-label="Settings">
            <h2>Settings</h2>
            <section style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <h3>Library folder</h3>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <code style={{ flex: "1 1 auto" }}>{libraryDir}</code>
                <button type="button" className="btn" onClick={() => openPath(libraryDir).catch(() => say("The folder will appear after your first import"))}>Open</button>
              </div>
            </section>
            <section style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <h3>Adobe Fonts folder</h3>
              <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                <code style={{ flex: "1 1 240px", minWidth: 0, overflowWrap: "anywhere" }}>{settings?.adobeDir || "Not found"}</code>
                <button
                  type="button"
                  className="btn"
                  onClick={async () => {
                    const dir = await openDialog({ directory: true, defaultPath: settings?.adobeDir || undefined });
                    if (typeof dir === "string") changeAdobeDir(dir);
                  }}
                >
                  Change
                </button>
                {settings && !settings.adobeDirIsDefault && (
                  <button type="button" className="btn" onClick={() => changeAdobeDir(null)}>Reset</button>
                )}
              </div>
              <p className="muted" style={{ fontSize: 12 }}>
                {settings && !settings.adobeDirExists ? "This folder doesn't exist yet. It appears once Creative Cloud syncs a font. " : ""}
                {external ? `${families.filter((f) => f.rep.external).length} system and Adobe font families listed.` : "Looking for system and Adobe fonts…"}{" "}
                <button type="button" className="link" onClick={rescan}>Look again</button>
              </p>
            </section>
            <section style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 16 }}>
              <div>
                <p style={{ fontWeight: 600, fontSize: 14 }}>Start with Windows</p>
                <p className="muted" style={{ marginTop: 4, fontSize: 12 }}>Runs quietly in the system tray and re-activates your fonts after a restart.</p>
              </div>
              <button
                type="button"
                className="icon44"
                aria-label="Start with Windows"
                aria-pressed={startup}
                onClick={async () => {
                  try {
                    if (startup) await disableAutostart();
                    else await enableAutostart();
                    setStartup(!startup);
                  } catch (e) {
                    say(errorText(e));
                  }
                }}
              >
                <span className={`switch${startup ? " on" : ""}`}>
                  <i />
                </span>
              </button>
            </section>
            <p className="muted">Coming in the next updates: AI tagging and Generate Artwork.</p>
            <div className="dialog-actions">
              <button type="button" className="btn primary" onClick={() => setModal(null)}>Done</button>
            </div>
          </div>
        </div>
      )}

      {modal === "design" && design && (
        <div className="overlay">
          <div className="dialog wide" role="dialog" aria-label="Fonts in the Illustrator files">
            <div>
              <h2>{design.files.length === 1 ? `Fonts in ${design.files[0].name}` : `Fonts in ${design.files.length} files`}</h2>
              <p className="muted" style={{ marginTop: 6 }}>
                {design.activated > 0 ? `${plural(design.activated)} from your library turned on. ` : ""}
                Fonts marked missing aren't on this computer.
              </p>
            </div>
            <div className="design-list">
              {design.files.map((file) => (
                <section key={file.file}>
                  {design.files.length > 1 && <h3>{file.name}</h3>}
                  {file.error && <p className="errline">{file.error}</p>}
                  {!file.error && file.fonts.length === 0 && <p className="muted">No fonts were found in this file. It may use only outlined text.</p>}
                  {file.fonts.map((u) => {
                    const m = matchUsed(u);
                    const label = m ? `${m.family} ${m.style}` : u.family ? `${u.family} ${u.style ?? ""}`.trim() : u.postscript;
                    const status = !m ? "Missing" : m.external ? (m.origin === ADOBE ? "Adobe Fonts" : "Installed in Windows") : m.active ? "Active" : "Inactive";
                    return (
                      <div className="design-row" key={u.postscript}>
                        <span className={`status ${!m ? "missing" : m.active || m.external ? "ok" : "off"}`}>{status}</span>
                        <span className="design-name" title={u.postscript}>{label}</span>
                        {m && !m.external && !m.active && (
                          <button type="button" className="btn" onClick={() => setActive([m.id], true)}>Activate</button>
                        )}
                        <button
                          type="button"
                          className="icon-round"
                          aria-label={`Copy the name ${label}`}
                          title="Copy the name"
                          onClick={() => navigator.clipboard.writeText(label).then(() => say(`Copied: ${label}`)).catch(() => say("Couldn't copy"))}
                        >
                          <IconCopy size={15} />
                        </button>
                      </div>
                    );
                  })}
                </section>
              ))}
            </div>
            <div className="dialog-actions">
              {(() => {
                const ids = new Set<string>();
                for (const f of design.files) for (const u of f.fonts) {
                  const m = matchUsed(u);
                  if (m) famByKey.get(`${m.origin}|${m.family}`)?.ids.forEach((id) => ids.add(id));
                }
                if (!ids.size) return null;
                let name = design.files.length === 1 ? design.files[0].name : "Illustrator fonts";
                for (let n = 2; collNames.includes(name); n++) name = `${design.files.length === 1 ? design.files[0].name : "Illustrator fonts"} ${n}`;
                return (
                  <button
                    type="button"
                    className="btn"
                    onClick={() => {
                      commit((l) => l.collections.push({ name, fontIds: [...ids], favourites: [], sampleText: null }), `Created the collection ${name}`);
                      setModal(null);
                      setColl(name);
                      setSel([]);
                    }}
                  >
                    <IconPlus size={15} /> Create collection “{name}”
                  </button>
                );
              })()}
              <button type="button" className="btn primary" onClick={() => setModal(null)}>Done</button>
            </div>
          </div>
        </div>
      )}

      {modal === "adobe" && (
        <div className="overlay">
          <div className="dialog" role="dialog" aria-label="Add recent Adobe Fonts">
            <div>
              <h2>Add recent Adobe Fonts</h2>
              <p className="muted" style={{ marginTop: 6 }}>Fonts you turned on in Creative Cloud, added to {coll}.</p>
            </div>
            <div className="seg" style={{ alignSelf: "flex-start" }}>
              <button type="button" aria-pressed={adobeRange === "today"} onClick={() => setAdobeRange("today")} style={{ width: "auto", padding: "0 14px" }}>Today</button>
              <button type="button" aria-pressed={adobeRange === "week"} onClick={() => setAdobeRange("week")} style={{ width: "auto", padding: "0 14px" }}>This week</button>
            </div>
            {(() => {
              const fams = recentAdobe(adobeRange);
              if (!external) return <p className="muted">Still looking for Adobe Fonts…</p>;
              if (!fams.length) return <p className="muted">No Adobe Fonts were turned on {adobeRange === "today" ? "today" : "this week"}.</p>;
              return <p>{fams.map((f) => f.family).join(", ")}</p>;
            })()}
            <div className="dialog-actions">
              <button type="button" className="btn" onClick={() => setModal(null)}>Cancel</button>
              <button type="button" className="btn primary" disabled={!recentAdobe(adobeRange).length} onClick={addRecentAdobe}>
                Add {plural(recentAdobe(adobeRange).length).replace("font", "family").replace("familys", "families")}
              </button>
            </div>
          </div>
        </div>
      )}

      {fileDrag && (
        <div className="drop-overlay">
          <div>Drop fonts to import them, or Illustrator files to see their fonts</div>
        </div>
      )}
      {ghost && (
        <div className="ghost" style={{ left: ghost.x, top: ghost.y }}>
          {ghost.label}
        </div>
      )}
    </div>
  );
}

/** The theme's own text and background colours, as a starting point for custom ones. */
function themeColors() {
  const cs = getComputedStyle(document.documentElement);
  const get = (v: string, d: string) => normHex(cs.getPropertyValue(v)) ?? d;
  return { on: true, text: get("--ink", "#343434"), bg: get("--bg", "#EDEBE8") };
}

function describePending(paths: string[]): string {
  const ext = (p: string) => p.split(".").pop()?.toLowerCase() ?? "";
  const zips = paths.filter((p) => ext(p) === "zip").length;
  const fonts = paths.filter((p) => ["otf", "ttf"].includes(ext(p))).length;
  const other = paths.length - zips - fonts;
  const parts = [];
  if (zips) parts.push(`${zips} zip${zips === 1 ? "" : "s"}`);
  if (fonts) parts.push(`${fonts} font file${fonts === 1 ? "" : "s"}`);
  if (other) parts.push(`${other} other item${other === 1 ? "" : "s"} (folders are searched for fonts)`);
  return `${paths.length} item${paths.length === 1 ? "" : "s"} dropped: ${parts.join(", ")}.`;
}

interface TagAreaProps {
  selected: Family[];
  tagCtx: string | null;
  setTagCtx: (v: string | null) => void;
  tagTimer: React.MutableRefObject<number | undefined>;
  removeTag: (label: string) => void;
  tagging: boolean;
  setTagging: (v: boolean) => void;
  tagVal: string;
  setTagVal: (v: string) => void;
  addTag: () => void;
}

function TagArea({ selected, tagCtx, setTagCtx, tagTimer, removeTag, tagging, setTagging, tagVal, setTagVal, addTag }: TagAreaProps) {
  // Combine tags across the selection, with a count when several fonts are selected.
  const agg = new Map<string, { label: string; fixed: boolean; source: string; count: number; removable: number }>();
  for (const fam of selected) {
    for (const t of tagList(fam)) {
      const a = agg.get(t.label) ?? { label: t.label, fixed: true, source: t.source, count: 0, removable: 0 };
      a.count++;
      if (!t.fixed) {
        a.fixed = false;
        a.removable++;
        a.source = t.source;
      }
      agg.set(t.label, a);
    }
  }
  const n = selected.length;
  return (
    <div className="tags">
      {Array.from(agg.values()).map((t) => (
        <span
          key={t.label}
          className={`tag${t.fixed ? " fixed" : ""}`}
          title={t.source}
          onContextMenu={(e) => {
            e.preventDefault();
            window.clearTimeout(tagTimer.current);
            setTagCtx(tagCtx === t.label ? null : t.label);
          }}
          onMouseEnter={() => window.clearTimeout(tagTimer.current)}
          onMouseLeave={() => {
            if (tagCtx !== t.label) return;
            window.clearTimeout(tagTimer.current);
            tagTimer.current = window.setTimeout(() => setTagCtx(null), 1000);
          }}
        >
          {n > 1 ? `${t.label} ${t.count}/${n}` : t.label}
          {tagCtx === t.label && (
            <span className="menu" role="menu">
              {t.fixed ? (
                <span style={{ padding: "8px 10px", fontSize: 12, color: "var(--muted)", whiteSpace: "nowrap", fontWeight: 400 }}>Read from the font file, can't be removed</span>
              ) : (
                <button type="button" role="menuitem" onClick={() => removeTag(t.label)}>
                  <IconTrash size={14} /> {n > 1 && t.removable > 1 ? `Remove tag from ${t.removable} fonts` : "Remove tag"}
                </button>
              )}
            </span>
          )}
        </span>
      ))}
      {tagging ? (
        <input
          className="tag-input"
          aria-label="New tag"
          placeholder="New tag, Enter to add"
          autoFocus
          value={tagVal}
          onChange={(e) => setTagVal(e.target.value)}
          onBlur={addTag}
          onKeyDown={(e) => {
            if (e.key === "Enter") addTag();
            if (e.key === "Escape") {
              setTagVal("");
              setTagging(false);
            }
          }}
        />
      ) : (
        <button type="button" className="tag-add" onClick={() => setTagging(true)}>
          + Tag
        </button>
      )}
    </div>
  );
}
