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
}

export interface ImportReport {
  added: string[];
  skipped: { name: string; existingId: string }[];
  errors: { file: string; message: string }[];
  licencesKept: number;
  duoTagged: number;
  fontIds: string[];
}

export interface AppState {
  library: Library;
  libraryDir: string;
  version: string;
}
