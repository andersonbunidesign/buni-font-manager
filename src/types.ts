// Mirrors the Rust structures in crates/bfm-core/src/model.rs.

export interface FontRecord {
  id: string;
  family: string;
  style: string;
  fullName: string;
  postscriptName: string;
  weight: number;
  width: number;
  italic: boolean;
  monospace: boolean;
  variable: boolean;
  axes: string[];
  classification: string;
  spacing: string;
  langs: string[];
  path: string;
  origin: string;
  addedAt: number;
  licence: string | null;
  duo: boolean;
  noEmbedding: boolean;
  active: boolean;
  aiTags: string[];
  hiddenAiTags: string[];
  customTags: string[];
  /** System or Adobe Fonts font: listed, never moved, deleted or deactivated. */
  external?: boolean;
  /** Position inside a .ttc collection file. */
  index?: number;
}

export interface ExternalMeta {
  customTags: string[];
  hiddenAiTags: string[];
  aiTags: string[];
}

export interface Collection {
  name: string;
  fontIds: string[];
  favourites: string[];
  sampleText: string | null;
}

export interface Library {
  version: number;
  fonts: FontRecord[];
  collections: Collection[];
  origins: string[];
  detection?: number;
  externalMeta?: Record<string, ExternalMeta>;
}

export interface NameConflict {
  family: string;
  otherOrigin: string;
  ids: string[];
}

export interface Settings {
  adobeDir: string;
  adobeDirIsDefault: boolean;
  adobeDirExists: boolean;
}

export interface ImportReport {
  added: string[];
  skipped: { name: string; existingId: string }[];
  errors: { file: string; message: string }[];
  licencesKept: number;
  duoTagged: number;
  fontIds: string[];
  conflicts?: NameConflict[];
}

export interface AppState {
  library: Library;
  libraryDir: string;
  version: string;
}
