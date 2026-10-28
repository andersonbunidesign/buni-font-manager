use serde::{Deserialize, Serialize};

/// One font file in the library (one weight / style of a family).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FontRecord {
    /// Short content hash, stable across sessions.
    pub id: String,
    /// Family name as shown to the user (typographic family when present).
    pub family: String,
    /// Style name inside the family, e.g. "Light Italic".
    pub style: String,
    pub full_name: String,
    pub postscript_name: String,
    /// usWeightClass, 100–900.
    pub weight: u16,
    /// usWidthClass, 1–9 (5 = normal).
    pub width: u16,
    pub italic: bool,
    pub monospace: bool,
    pub variable: bool,
    /// Variation axis tags for variable fonts, e.g. ["wght", "opsz"].
    #[serde(default)]
    pub axes: Vec<String>,
    /// Serif, Sans serif, Script, Decorative, Handwritten, Blackletter, Symbols.
    pub classification: String,
    /// Normal, Monospace, Condensed, Expanded.
    pub spacing: String,
    /// Language groups the font can set, e.g. "Swedish (å ä ö)".
    #[serde(default)]
    pub langs: Vec<String>,
    /// Absolute path to the font file.
    pub path: String,
    /// Where the font came from: Envato, Google Fonts, Client…
    pub origin: String,
    /// Seconds since the Unix epoch when it entered the library.
    pub added_at: u64,
    /// Absolute path to a licence file found next to the font, if any.
    #[serde(default)]
    pub licence: Option<String>,
    /// Two different families arrived in the same zip.
    #[serde(default)]
    pub duo: bool,
    /// OS/2 fsType says the font may not be embedded in documents.
    #[serde(default)]
    pub no_embedding: bool,
    /// Whether the font is currently active (available to other apps).
    #[serde(default = "yes")]
    pub active: bool,
    /// Tags suggested by AI tagging.
    #[serde(default)]
    pub ai_tags: Vec<String>,
    /// AI tags the user removed; kept so they are not suggested again.
    #[serde(default)]
    pub hidden_ai_tags: Vec<String>,
    /// Tags the user typed.
    #[serde(default)]
    pub custom_tags: Vec<String>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub name: String,
    /// Font ids in the collection.
    #[serde(default)]
    pub font_ids: Vec<String>,
    /// Font ids marked as favourite inside this collection only.
    #[serde(default)]
    pub favourites: Vec<String>,
    /// The sample text this collection remembers.
    #[serde(default)]
    pub sample_text: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    #[serde(default = "version_one")]
    pub version: u32,
    #[serde(default)]
    pub fonts: Vec<FontRecord>,
    #[serde(default)]
    pub collections: Vec<Collection>,
    /// Origins the user has used, most recent first.
    #[serde(default)]
    pub origins: Vec<String>,
    /// Version of the style detection that filled in classification/spacing.
    #[serde(default)]
    pub detection: u32,
}

/// Bump when the style detection improves, so existing libraries are re-read once.
pub const DETECTION_VERSION: u32 = 2;

fn version_one() -> u32 {
    1
}

impl Library {
    pub fn font(&self, id: &str) -> Option<&FontRecord> {
        self.fonts.iter().find(|f| f.id == id)
    }

    pub fn collection_mut(&mut self, name: &str) -> Option<&mut Collection> {
        self.collections.iter_mut().find(|c| c.name == name)
    }

    /// Adds ids to a collection (creating it if needed) without duplicates.
    pub fn add_to_collection(&mut self, name: &str, ids: &[String]) -> usize {
        if self.collection_mut(name).is_none() {
            self.collections.push(Collection { name: name.to_string(), ..Default::default() });
        }
        let c = self.collection_mut(name).expect("just created");
        let mut added = 0;
        for id in ids {
            if !c.font_ids.contains(id) {
                c.font_ids.push(id.clone());
                added += 1;
            }
        }
        added
    }

    /// Moves `origin` to the front of the remembered origins list.
    pub fn remember_origin(&mut self, origin: &str) {
        self.origins.retain(|o| o != origin);
        self.origins.insert(0, origin.to_string());
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Ids of fonts copied into the library by this import.
    pub added: Vec<String>,
    /// Fonts that were already in the library (not copied again).
    pub skipped: Vec<SkippedFont>,
    pub errors: Vec<ImportError>,
    /// Licence files kept and linked to fonts.
    pub licences_kept: usize,
    /// Number of fonts tagged Duo.
    pub duo_tagged: usize,
    /// Every font id touched (added or skipped), in order — what the collection receives.
    pub font_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFont {
    pub name: String,
    pub existing_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportError {
    pub file: String,
    pub message: String,
}
