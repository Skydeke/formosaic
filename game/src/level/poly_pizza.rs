//! Poly Pizza API client using ureq.
//!
//! Uses the **official** Poly Pizza v1.1 REST API:
//!   `https://api.poly.pizza/v1.1`
//!
//! Authentication: `x-auth-token: <key>` header.
//! Authentication: `x-auth-token: <key>` header.
//! The key is baked in at **compile time** via `build.rs` reading the
//! `POLY_PIZZA_API_KEY` env var — works on both desktop and Android.
//! Never commit the key; set it as a build/CI secret.
//!
//! Key endpoints used:
//!   Search:       GET /search/{keyword}
//!   Model detail: GET /model/{id}    → includes `Download` (CDN GLB URL)
//!
//! Network calls run on a background thread so the render loop never blocks.
//! Results are polled each frame via `try_recv`.

use std::sync::mpsc::{channel, Receiver, Sender};

const API_BASE: &str = "https://api.poly.pizza/v1.1";

/// Read the Poly Pizza API key from the environment.
/// Returns an error string if the variable is unset or empty.
/// Return the Poly Pizza API key baked in at compile time.
///
/// Set `POLY_PIZZA_API_KEY` **before `cargo build`** (CI secret for releases).
/// `build.rs` forwards it via `cargo:rustc-env` so it works on desktop and
/// Android without any runtime environment dependency.  Never commit the key.
fn api_key() -> Result<&'static str, String> {
    const KEY: &str = env!("POLY_PIZZA_API_KEY");
    if KEY.is_empty() {
        Err("No Poly Pizza API key baked in. Set POLY_PIZZA_API_KEY before              `cargo build`. Get a key at https://poly.pizza/settings/api".to_string())
    } else {
        Ok(KEY)
    }
}

// ─── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ModelSummary {
    pub id: String,
    pub name: String,
    pub author: String,
    pub license: String,
    pub thumbnail_url: String,
    pub source_url: String,
    /// Direct CDN download URL (from the v1.1 search response `Download` field).
    /// If non-empty, we can skip the /model/{id} detail fetch.
    pub download_url: String,
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
        let tx  = self.download_tx.clone();
        let id  = summary.id.clone();
        let nm  = summary.name.clone();
        let au  = summary.author.clone();
        let li  = summary.license.clone();
        let su  = summary.source_url.clone();
        let dl  = summary.download_url.clone();   // pre-known CDN URL if available
        std::thread::spawn(move || {
            let _ = tx.send(do_download(&id, &nm, &au, &li, &su, &dl));
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

fn authed_get_string(url: &str, key: &str) -> Result<String, String> {
    ureq::get(url)
        .set("x-auth-token", key)
        .call()
        .map_err(|e| format!("GET {}: {}", url, e))?
        .into_string()
        .map_err(|e| format!("read body {}: {}", url, e))
}

fn authed_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    // CDN downloads (static.poly.pizza) don't need auth
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

/// Fetch a random page of free (CC0) animated models.
///
/// Strategy: fetch page 0 first to discover the `total` count, then pick a
/// uniformly random valid page and fetch it.  This avoids 401s from
/// out-of-range page numbers while still giving variety.
/// Falls back to CC0 non-animated if the animated set is empty.
fn fetch_explore(_offset: usize, limit: usize) -> ExploreResult {
    let key  = api_key()?;
    let lim  = limit.min(32);

    fetch_random_page(API_BASE, &key, lim, true)
        .or_else(|_| fetch_random_page(API_BASE, &key, lim, false))
}

fn fetch_random_page(base: &str, key: &str, lim: usize, animated: bool) -> ExploreResult {
    use rand::Rng;

    let anim_param = if animated { "&Animated=1" } else { "" };

    // ── Step 1: page 0 to learn total ────────────────────────────────────
    let url0 = format!("{}/search?License=1{}&Limit={}&Page=0", base, anim_param, lim);
    log::info!("[PolyPizza] GET {} (discover total)", url0);
    let body0 = authed_get_string(&url0, key)?;
    log::debug!("[PolyPizza] page-0 response (first 500): {:.500}", body0);

    let total = json_number(&body0, "total").unwrap_or(lim as f64) as usize;
    let max_page = if total > lim { (total / lim).saturating_sub(1) } else { 0 };

    log::info!("[PolyPizza] total={} max_page={}", total, max_page);

    // ── Step 2: pick a random valid page ─────────────────────────────────
    let page = if max_page > 0 { rand::rng().random_range(0..=max_page) } else { 0 };

    let body = if page == 0 {
        body0  // reuse what we already fetched
    } else {
        let url = format!("{}/search?License=1{}&Limit={}&Page={}", base, anim_param, lim, page);
        log::info!("[PolyPizza] GET {} (random page)", url);
        authed_get_string(&url, key)?
    };

    let summaries = parse_explore_v1(&body)
        .ok_or_else(|| format!("[PolyPizza] could not parse JSON: {:.300}", body))?;

    if summaries.is_empty() {
        Err(format!("[PolyPizza] page {} returned 0 models", page))
    } else {
        log::info!("[PolyPizza] parsed {} summaries from page {}", summaries.len(), page);
        Ok(summaries)
    }
}

/// Download a model: use `known_dl_url` if non-empty (it comes from the search
/// response `Download` field), otherwise fetch the model detail endpoint to get it.
fn do_download(id: &str, name: &str, author: &str, license: &str, src: &str, known_dl_url: &str) -> DownloadResult {
    // Fast path: search response already gave us the CDN URL
    let (dl_url, ext) = if !known_dl_url.is_empty() && (known_dl_url.contains(".glb") || known_dl_url.contains(".fbx")) {
        log::info!("[PolyPizza] Using inline download URL for '{}': {}", id, known_dl_url);
        (known_dl_url.to_string(), url_ext(known_dl_url))
    } else {
        // Slow path: fetch /model/{id} to get the Download field
        let key = api_key()?;
        let detail_url = format!("{}/model/{}", API_BASE, id);
        log::info!("[PolyPizza] GET model detail: {}", detail_url);
        let body = authed_get_string(&detail_url, &key)
            .map_err(|e| format!("[PolyPizza] model detail fetch failed for '{}': {}", id, e))?;
        log::debug!("[PolyPizza] model detail: {:.1200}", body);
        extract_download_url_v1(&body)
            .ok_or_else(|| format!("[PolyPizza] no Download URL in model detail for '{}': {:.300}", id, body))?
    };

    log::info!("[PolyPizza] Downloading '{}' from {}", name, dl_url);
    let bytes = authed_get_bytes(&dl_url)?;
    log::info!("[PolyPizza] Downloaded {} bytes for '{}'", bytes.len(), name);

    Ok(ModelDownload {
        id:         id.to_string(),
        name:       name.to_string(),
        author:     author.to_string(),
        license:    license.to_string(),
        source_url: src.to_string(),
        file_ext:   ext,
        bytes,
    })
}

// ─── JSON parsing (no serde dependency) ──────────────────────────────────────
//
// Poly Pizza v1.1 API JSON structure (from the official OpenAPI spec):
// {
//   "total": 77,
//   "results": [
//     {
//       "ID": "BwwnUrWGmV",
//       "Title": "Police Car",
//       "Attribution": "...",
//       "Thumbnail": "https://static.poly.pizza/....webp",
//       "Download":   "https://static.poly.pizza/....glb",
//       "Creator": { "Username": "Quaternius", "DPURL": "..." },
//       "Licence": "CC0 1.0",
//       "Animated": false
//     }
//   ]
// }
//
// /model/{id} response has identical field names.

fn parse_explore_v1(body: &str) -> Option<Vec<ModelSummary>> {
    let arr = find_json_array(body, "results")?;
    let objs = split_json_objects(arr);
    log::debug!("[PolyPizza] {} raw objects to parse", objs.len());

    let out: Vec<ModelSummary> = objs
        .iter()
        .filter_map(|o| parse_summary_v1(o))
        .collect();
    log::info!("[PolyPizza] parsed {} summaries", out.len());
    Some(out)
}

fn parse_summary_v1(obj: &str) -> Option<ModelSummary> {
    // v1.1 field names are PascalCase: ID, Title, Creator.Username, Licence
    let id = json_str(obj, "ID")?;
    let name = json_str(obj, "Title").unwrap_or_default();
    let author = json_nested_str(obj, "Creator", "Username").unwrap_or_default();
    let license = json_str(obj, "Licence").unwrap_or_else(|| "CC0".to_string());
    let thumbnail_url = json_str(obj, "Thumbnail").unwrap_or_default();
    // The v1.1 search results include a direct Download URL — store it in
    // source_url so we can skip the /model/{id} round-trip when possible.
    let download_url = json_str(obj, "Download").unwrap_or_default();
    let source_url = format!("https://poly.pizza/m/{}", id);
    Some(ModelSummary {
        id,
        name,
        author,
        license,
        thumbnail_url,
        source_url,
        download_url,
    })
}

/// Extract the CDN download URL from a v1.1 /model/{id} response.
/// The `Download` field is a direct HTTPS GLB URL.
fn extract_download_url_v1(body: &str) -> Option<(String, String)> {
    // Primary: flat "Download" field (v1.1 spec)
    if let Some(url) = json_str(body, "Download") {
        if !url.is_empty() {
            return Some((url.clone(), url_ext(&url)));
        }
    }
    // Fallback: scan for any CDN 3D URL in the body
    for needle in &[".glb", ".fbx", ".obj"] {
        if let Some(pos) = body.find(needle) {
            let before = &body[..pos];
            if let Some(q) = before.rfind('"') {
                let url = format!("{}{}", &before[q + 1..], &needle[..needle.len() - 1]);
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

/// Extract a JSON number value (integer or float) by key.
fn json_number(s: &str, key: &str) -> Option<f64> {
    let needle = format!("\"{}\":", key);
    let start  = s.find(needle.as_str())? + needle.len();
    let rest   = s[start..].trim_start();
    let end    = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

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
