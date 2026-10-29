//! Reads which fonts an Illustrator file uses, without opening Illustrator.
//!
//! Illustrator stores a list of the document's fonts in the file's XMP metadata
//! (`stFnt:fontName`, `stFnt:fontFamily`, `stFnt:fontFace`). When that list is
//! missing, the PDF part of the file is searched for `/BaseFont` entries, also
//! inside compressed streams.

use flate2::read::ZlibDecoder;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsedFont {
    /// PostScript name, e.g. "Fraunces-Bold".
    pub postscript: String,
    pub family: Option<String>,
    pub style: Option<String>,
}

pub fn is_design_file(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".ai") || n.ends_with(".eps")
}

fn xml_unescape(s: &str) -> String {
    s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'")
}

/// Value of `<tag>…</tag>` or `tag="…"` inside an XMP font entry.
fn xmp_value(entry: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    if let Some(a) = entry.find(&open) {
        let rest = &entry[a + open.len()..];
        let b = rest.find('<')?;
        let v = xml_unescape(rest[..b].trim());
        return (!v.is_empty()).then_some(v);
    }
    let attr = format!("{tag}=\"");
    let a = entry.find(&attr)?;
    let rest = &entry[a + attr.len()..];
    let b = rest.find('"')?;
    let v = xml_unescape(rest[..b].trim());
    (!v.is_empty()).then_some(v)
}

fn from_xmp(text: &str) -> Vec<UsedFont> {
    let mut out = Vec::new();
    let mut rest = text;
    // Each font is an <rdf:li …> (element or attribute form) that holds stFnt:fontName.
    while let Some(i) = rest.find("stFnt:fontName") {
        let start = rest[..i].rfind("<rdf:li").unwrap_or(i);
        let end = rest[i..].find("</rdf:li>").map(|e| i + e).or_else(|| rest[i..].find("/>").map(|e| i + e)).unwrap_or(rest.len());
        let entry = &rest[start..end];
        if let Some(ps) = xmp_value(entry, "stFnt:fontName") {
            out.push(UsedFont { postscript: ps, family: xmp_value(entry, "stFnt:fontFamily"), style: xmp_value(entry, "stFnt:fontFace") });
        }
        rest = &rest[end.max(i + 1)..];
    }
    out
}

/// Decodes `#20`-style escapes in PDF names.
fn pdf_name(raw: &[u8]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'#' && i + 2 < raw.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&raw[i + 1..i + 3]).unwrap_or("zz"), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(raw[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn base_fonts(data: &[u8], out: &mut Vec<String>) {
    let key = b"/BaseFont";
    let mut i = 0;
    while let Some(p) = find(&data[i..], key) {
        let mut j = i + p + key.len();
        while j < data.len() && (data[j] == b' ' || data[j] == b'\r' || data[j] == b'\n') {
            j += 1;
        }
        if j < data.len() && data[j] == b'/' {
            let s = j + 1;
            let mut e = s;
            while e < data.len() && !b" \r\n\t/[]<>()%{}".contains(&data[e]) {
                e += 1;
            }
            let mut name = pdf_name(&data[s..e]);
            // Subset prefix: ABCDEF+Name.
            if name.len() > 7 && name.as_bytes()[6] == b'+' && name[..6].bytes().all(|b| b.is_ascii_uppercase()) {
                name = name[7..].to_string();
            }
            if !name.is_empty() {
                out.push(name);
            }
            i = e;
        } else {
            i = j;
        }
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Inflates the compressed streams of a PDF (object streams hold the font dictionaries).
fn inflated_streams(data: &[u8], budget: &mut usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = find(&data[i..], b"stream") {
        let mut s = i + p + 6;
        // Skip "endstream" matches.
        if i + p >= 3 && &data[i + p - 3..i + p] == b"end" {
            i = s;
            continue;
        }
        if s < data.len() && data[s] == b'\r' {
            s += 1;
        }
        if s < data.len() && data[s] == b'\n' {
            s += 1;
        }
        let Some(e) = find(&data[s..], b"endstream") else { break };
        let raw = &data[s..s + e];
        // Only zlib streams (78 xx) are worth trying.
        if raw.len() > 2 && raw[0] == 0x78 && *budget > 0 {
            let mut buf = Vec::new();
            let limit = (*budget).min(32 << 20) as u64;
            if ZlibDecoder::new(raw).take(limit).read_to_end(&mut buf).is_ok() || !buf.is_empty() {
                *budget = budget.saturating_sub(buf.len());
                if find(&buf, b"/BaseFont").is_some() || find(&buf, b"stFnt:fontName").is_some() {
                    out.push(buf);
                }
            }
        }
        i = s + e + 9;
    }
    out
}

/// Fonts used by an Illustrator (or EPS) file, without duplicates.
pub fn used_fonts(data: &[u8]) -> Vec<UsedFont> {
    let text = String::from_utf8_lossy(data);
    let mut fonts = from_xmp(&text);
    if fonts.is_empty() {
        let mut names = Vec::new();
        base_fonts(data, &mut names);
        let mut budget = 256usize << 20;
        for s in inflated_streams(data, &mut budget) {
            let t = String::from_utf8_lossy(&s);
            fonts.extend(from_xmp(&t));
            base_fonts(&s, &mut names);
        }
        // Old PostScript-based files list them in a comment.
        for line in text.lines().filter(|l| l.starts_with("%%DocumentFonts:") || l.starts_with("%%+ ")) {
            for n in line.trim_start_matches("%%DocumentFonts:").trim_start_matches("%%+").split_whitespace() {
                if n != "(atend)" {
                    names.push(n.to_string());
                }
            }
        }
        fonts.extend(names.into_iter().map(|n| UsedFont { postscript: n, family: None, style: None }));
    }
    let mut seen = BTreeMap::new();
    for f in fonts {
        seen.entry(f.postscript.clone()).or_insert(f);
    }
    seen.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    #[test]
    fn reads_xmp_font_list() {
        let xmp = r#"<xmpTPg:Fonts><rdf:Bag>
<rdf:li rdf:parseType="Resource"><stFnt:fontName>Fraunces-Bold</stFnt:fontName><stFnt:fontFamily>Fraunces</stFnt:fontFamily><stFnt:fontFace>Bold</stFnt:fontFace></rdf:li>
<rdf:li stFnt:fontName="Inter-Regular" stFnt:fontFamily="Inter" stFnt:fontFace="Regular"/>
</rdf:Bag></xmpTPg:Fonts>"#;
        let f = used_fonts(format!("%PDF-1.6\n{xmp}\n").as_bytes());
        assert_eq!(f.len(), 2);
        assert_eq!(f[0], UsedFont { postscript: "Fraunces-Bold".into(), family: Some("Fraunces".into()), style: Some("Bold".into()) });
        assert_eq!(f[1].family.as_deref(), Some("Inter"));
    }

    #[test]
    fn falls_back_to_pdf_fonts_in_compressed_streams() {
        let mut enc = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(b"<< /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+Pinyon#20Script >>").unwrap();
        let z = enc.finish().unwrap();
        let mut file = b"%PDF-1.6\n1 0 obj << /BaseFont /SpaceMono-Bold >> endobj\n2 0 obj << /Length 10 /Filter /FlateDecode >>\nstream\n".to_vec();
        file.extend(z);
        file.extend(b"\nendstream\nendobj\n");
        let f: Vec<String> = used_fonts(&file).into_iter().map(|u| u.postscript).collect();
        assert_eq!(f, vec!["Pinyon Script".to_string(), "SpaceMono-Bold".to_string()]);
    }
}
