//! Poly Pizza API client using ureq.
//!
//! Based on the API used by <https://github.com/Chikanz/pizzabox>.
//! Explore endpoint:  GET https://poly.pizza/api/search/explore?take=N&skip=O
//! Model detail:      GET https://poly.pizza/api/m/{id}
//!
//! Network calls run on a background thread so the render loop never blocks.
//! Results are polled each frame via `try_recv`.

use std::sync::mpsc::{channel, Receiver, Sender};

const API_BASE: &str = "https://poly.pizza/api";

// ─── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ModelSummary {
    pub id: String,
    pub name: String,
    pub author: String,
    pub license: String,
    pub thumbnail_url: String,
    pub source_url: String,
}

#[derive(Debug)]
pub struct ModelDownload {
    pub id: String,
    pub name: String,
    pub author: String,
    pub license: String,
    pub source_url: String,
    pub file_ext: String,
    pub bytes: Vec<u8>,
}

pub type ExploreResult = Result<Vec<ModelSummary>, String>;
pub type DownloadResult = Result<ModelDownload, String>;

// ─── Client ──────────────────────────────────────────────────────────────────

pub struct PolyPizzaClient {
    explore_tx: Sender<ExploreResult>,
    explore_rx: Receiver<ExploreResult>,
    download_tx: Sender<DownloadResult>,
    download_rx: Receiver<DownloadResult>,
    explore_pending: bool,
    download_pending: bool,
}

impl PolyPizzaClient {
    pub fn new() -> Self {
        let (etx, erx) = channel();
        let (dtx, drx) = channel();
        Self {
            explore_tx: etx,
            explore_rx: erx,
            download_tx: dtx,
            download_rx: drx,
            explore_pending: false,
            download_pending: false,
        }
    }

    /// Start an async explore-page fetch. Results arrive via `poll_explore`.
    pub fn fetch_explore_page(&mut self, offset: usize, limit: usize) {
        if self.explore_pending {
            return;
        }
        self.explore_pending = true;
        let tx = self.explore_tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(fetch_explore(offset, limit));
        });
    }

    /// Non-blocking poll — returns `Some(result)` when the fetch completes.
    pub fn poll_explore(&mut self) -> Option<ExploreResult> {
        match self.explore_rx.try_recv() {
            Ok(r) => {
                self.explore_pending = false;
                Some(r)
            }
            Err(_) => None,
        }
    }

    /// Start an async download of the model identified by `summary`.
    pub fn download_model(&mut self, summary: &ModelSummary) {
        if self.download_pending {
            return;
        }
        self.download_pending = true;
        let tx = self.download_tx.clone();
        let id = summary.id.clone();
        let nm = summary.name.clone();
        let au = summary.author.clone();
        let li = summary.license.clone();
        let su = summary.source_url.clone();
        std::thread::spawn(move || {
            let _ = tx.send(do_download(&id, &nm, &au, &li, &su));
        });
    }

    /// Non-blocking poll — returns `Some(result)` when the download completes.
    pub fn poll_download(&mut self) -> Option<DownloadResult> {
        match self.download_rx.try_recv() {
            Ok(r) => {
                self.download_pending = false;
                Some(r)
            }
            Err(_) => None,
        }
    }

    pub fn is_explore_pending(&self) -> bool {
        self.explore_pending
    }
    pub fn is_download_pending(&self) -> bool {
        self.download_pending
    }
}

impl Default for PolyPizzaClient {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Network (ureq) ───────────────────────────────────────────────────────────

fn get_string(url: &str) -> Result<String, String> {
    ureq::get(url)
        .call()
        .map_err(|e| format!("GET {}: {}", url, e))?
        .into_string()
        .map_err(|e| format!("read body {}: {}", url, e))
}

fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("GET {}: {}", url, e))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| format!("read body {}: {}", url, e))?;
    Ok(buf)
}

// ─── API calls ────────────────────────────────────────────────────────────────

fn fetch_explore(offset: usize, limit: usize) -> ExploreResult {
    let url = format!("{}/search/explore?take={}&skip={}", API_BASE, limit, offset);
    log::info!("[PolyPizza] GET {}", url);
    let body = get_string(&url)?;
    log::debug!(
        "[PolyPizza] explore response (first 1200 chars): {:.1200}",
        body
    );
    parse_explore(&body)
        .ok_or_else(|| format!("[PolyPizza] could not parse explore JSON: {:.300}", body))
}

fn do_download(id: &str, name: &str, author: &str, license: &str, src: &str) -> DownloadResult {
    // Poly Pizza model detail endpoints to try (undocumented API)
    // /api/search/explore gives metadata only; detail page needed for download URL
    let candidates = vec![
        format!("{}/search/{}", API_BASE, id), // most likely REST pattern
        format!("{}/search/model/{}", API_BASE, id), // alternative
        format!("{}/search?id={}", API_BASE, id), // query param style
        format!("https://poly.pizza/api/{}", id), // minimal path
    ];

    let mut body = String::new();
    for url in &candidates {
        log::info!("[PolyPizza] GET model info: {}", url);
        match get_string(url) {
            Ok(b) => {
                log::debug!("[PolyPizza] response: {:.1200}", b);
                // Accept if it contains asset/download info
                if b.contains("glb")
                    || b.contains("fbx")
                    || b.contains("Assets")
                    || b.contains("assets")
                {
                    body = b;
                    break;
                }
                if body.is_empty() {
                    body = b;
                } // keep as fallback
            }
            Err(e) => log::debug!("[PolyPizza] {} -> {}", url, e),
        }
    }
    if body.is_empty() {
        return Err(format!("[PolyPizza] no model info for '{}'", id));
    }

    let (dl_url, ext) = extract_download_url(&body)
        .ok_or_else(|| format!("[PolyPizza] no download URL for '{}': {:.300}", id, body))?;

    log::info!(
        "[PolyPizza] Downloading {} ({} bytes expected) from {}",
        id,
        0,
        dl_url
    );
    let bytes = get_bytes(&dl_url)?;
    log::info!(
        "[PolyPizza] Downloaded {} bytes for '{}'",
        bytes.len(),
        name
    );

    Ok(ModelDownload {
        id: id.to_string(),
        name: name.to_string(),
        author: author.to_string(),
        license: license.to_string(),
        source_url: src.to_string(),
        file_ext: ext,
        bytes,
    })
}

// ─── JSON parsing (no serde dependency) ──────────────────────────────────────
//
// Poly Pizza JSON structure (from pizzabox reference):
// { "results": [
//     { "ID": "7S5Snphkam", "Title": "Cactus",
//       "Creator": { "DisplayName": "SoyMaria" },
//       "Licence": "CC-BY",
//       "Thumbnail": "https://...",
//       "Assets": [ { "Type": "Source", "URL": "https://...glb" } ]
//     }
//   ]
// }

fn parse_explore(body: &str) -> Option<Vec<ModelSummary>> {
    // Try "results", then "items", then "unity" (poly.pizza also returns unity assets)
    let arr = find_json_array(body, "results")
        .or_else(|| find_json_array(body, "items"))
        .or_else(|| find_json_array(body, "unity"))?;

    let objs = split_json_objects(arr);
    log::debug!("[PolyPizza] {} raw objects to parse", objs.len());

    let out: Vec<ModelSummary> = objs
        .iter()
        .filter_map(|o| {
            let s = parse_summary(o);
            if s.is_none() {
                log::debug!("[PolyPizza] parse_summary failed for obj: {:.100}", o);
            }
            s
        })
        .collect();
    log::info!("[PolyPizza] parsed {} summaries", out.len());
    Some(out)
}

fn parse_summary(obj: &str) -> Option<ModelSummary> {
    // Actual Poly Pizza API (observed response):
    // "publicID":"fnFCCFiHbQt" — string slug (the real ID)
    // "title":"Space probe"   — lowercase
    // "creator":{"username":"Poly by Google"} — lowercase
    // "licence":"CC-BY 3.0"   — lowercase
    // "previewUrl":"https://..."
    // "url":"/m/fnFCCFiHbQt"
    // "publicID" is the string slug; "id" is a useless integer; "url"="/m/{slug}" fallback
    let id = json_str(obj, "publicID")
        .or_else(|| json_str(obj, "ID"))
        .or_else(|| {
            json_str(obj, "url").and_then(|u| {
                if u.len() > 3 && &u[..3] == "/m/" {
                    Some(u[3..].to_string())
                } else {
                    None
                }
            })
        })?;
    let name = json_str(obj, "title")
        .or_else(|| json_str(obj, "Title"))
        .or_else(|| json_str(obj, "alt"))
        .unwrap_or_default();
    let author = json_nested_str(obj, "creator", "username")
        .or_else(|| json_nested_str(obj, "Creator", "DisplayName"))
        .or_else(|| json_nested_str(obj, "Creator", "Username"))
        .unwrap_or_default();
    let license = json_str(obj, "licence")
        .or_else(|| json_str(obj, "Licence"))
        .or_else(|| json_str(obj, "license"))
        .unwrap_or_else(|| "CC-BY".to_string());
    let thumbnail_url = json_str(obj, "previewUrl")
        .or_else(|| json_str(obj, "Thumbnail"))
        .unwrap_or_default();
    let source_url = format!("https://poly.pizza/m/{}", id);
    Some(ModelSummary {
        id,
        name,
        author,
        license,
        thumbnail_url,
        source_url,
    })
}

fn extract_download_url(body: &str) -> Option<(String, String)> {
    // Try Assets array — prefer "Source" type entry
    if let Some(assets) = find_json_array(body, "Assets") {
        let objs = split_json_objects(assets);
        // First pass: explicit Source/GLB type
        for obj in &objs {
            let t = json_str(obj, "Type").unwrap_or_default().to_lowercase();
            if t == "source" || t == "glb" || t == "fbx" {
                if let Some(url) = json_str(obj, "URL").or_else(|| json_str(obj, "url")) {
                    return Some((url.clone(), url_ext(&url)));
                }
            }
        }
        // Second pass: any asset with a 3D file extension
        for obj in &objs {
            if let Some(url) = json_str(obj, "URL").or_else(|| json_str(obj, "url")) {
                if url.contains(".glb") || url.contains(".fbx") || url.contains(".obj") {
                    return Some((url.clone(), url_ext(&url)));
                }
            }
        }
    }
    // Flat Download/download field
    if let Some(url) = json_str(body, "Download").or_else(|| json_str(body, "download")) {
        return Some((url.clone(), url_ext(&url)));
    }

    // Last resort: scan for any .glb/.fbx URL in the response body
    // The CDN URL pattern is: https://static.poly.pizza/.../*.glb
    for needle in &[".glb", ".fbx", ".obj"] {
        if let Some(pos) = body.find(needle) {
            // Walk back to find the opening quote of the URL
            let before = &body[..pos];
            if let Some(q) = before.rfind('"') {
                let url = &before[q + 1..];
                let url = format!("{}{}", url, &needle[..needle.len() - 1]);
                if url.starts_with("http") {
                    return Some((url, url_ext(needle)));
                }
            }
        }
    }
    None
}

fn url_ext(url: &str) -> String {
    for ext in &["glb", "fbx", "obj"] {
        if url.contains(&format!(".{}", ext)) {
            return ext.to_string();
        }
    }
    "glb".to_string()
}

// ─── Tiny zero-dependency JSON helpers ───────────────────────────────────────

fn json_str(s: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start = s.find(needle.as_str())? + needle.len();
    let rest = s[start..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let inner = &rest[1..];
    let end = find_closing_quote(inner)?;
    Some(unescape(&inner[..end]))
}

fn json_nested_str(s: &str, outer: &str, inner_key: &str) -> Option<String> {
    let needle = format!("\"{}\":", outer);
    let start = s.find(needle.as_str())? + needle.len();
    let rest = s[start..].trim_start();
    if !rest.starts_with('{') {
        return None;
    }
    let end = find_matching_brace(rest, '{', '}')?;
    json_str(&rest[..=end], inner_key)
}

fn find_json_array<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\":", key);
    let start = s.find(needle.as_str())? + needle.len();
    let rest = s[start..].trim_start();
    if !rest.starts_with('[') {
        return None;
    }
    let end = find_matching_brace(rest, '[', ']')?;
    Some(&rest[1..end])
}

fn split_json_objects(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(si) = start {
                        result.push(&s[si..=i]);
                        start = None;
                    }
                }
            }
            _ => {}
        }
    }
    result
}

fn find_matching_brace(s: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        if ch == open {
            depth += 1;
        }
        if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn find_closing_quote(s: &str) -> Option<usize> {
    let mut escaped = false;
    for (i, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Some(i);
        }
    }
    None
}

fn unescape(s: &str) -> String {
    s.replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .replace("\\/", "/")
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
}

// ─── Trait impl needed by ureq's read_to_end ─────────────────────────────────
use std::io::Read;
