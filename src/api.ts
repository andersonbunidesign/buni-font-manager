import { invoke } from "@tauri-apps/api/core";
import type { AppState, FontRecord, ImportReport, Library } from "./types";

export const api = {
  getState: () => invoke<AppState>("get_state"),
  importPaths: (paths: string[], collection: string | null, origin: string) =>
    invoke<{ report: ImportReport; library: Library }>("import_paths", { paths, collection, origin }),
  saveLibrary: (library: Library) => invoke<void>("save_library", { library }),
  setActive: (ids: string[], active: boolean) => invoke<Library>("set_active", { ids, active }),
  deleteFonts: (ids: string[]) => invoke<{ removed: FontRecord[]; library: Library }>("delete_fonts", { ids }),
  restoreFonts: (removed: FontRecord[], snapshot: Library) => invoke<Library>("restore_fonts", { removed, snapshot }),
};
