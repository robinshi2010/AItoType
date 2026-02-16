//! 用户易错词记忆与替换模块

use aho_corasick::{AhoCorasickBuilder, MatchKind};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const CORRECTIONS_FILENAME: &str = "corrections.json";
pub const CORRECTIONS_VERSION: u32 = 1;
pub const MAX_ENTRIES: usize = 500;
pub const MAX_VARIANTS_PER_ENTRY: usize = 50;
pub const MAX_VARIANT_CHARS: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrectionHit {
    pub variant: String,
    pub correct: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionEntry {
    pub correct: String,
    pub variants: Vec<String>,
    pub updated_at: String,
    #[serde(default)]
    pub hit_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionStore {
    #[serde(default = "default_store_version")]
    pub version: u32,
    #[serde(default)]
    pub corrections: Vec<CorrectionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCorrectionsResult {
    pub text: String,
    pub hits: Vec<CorrectionHit>,
}

impl Default for CorrectionStore {
    fn default() -> Self {
        Self {
            version: CORRECTIONS_VERSION,
            corrections: Vec::new(),
        }
    }
}

fn default_store_version() -> u32 {
    CORRECTIONS_VERSION
}

fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

fn normalize_variant_input(input: &str) -> Result<String, String> {
    let normalized = input.trim().to_lowercase();
    if normalized.is_empty() {
        return Err("错误词不能为空".to_string());
    }
    if normalized.chars().count() > MAX_VARIANT_CHARS {
        return Err(format!(
            "错误词长度不能超过 {} 个字符",
            MAX_VARIANT_CHARS
        ));
    }
    Ok(normalized)
}

fn sanitize_store(store: CorrectionStore) -> CorrectionStore {
    let mut used_variants: HashSet<String> = HashSet::new();
    let mut sanitized = Vec::new();

    for entry in store.corrections.into_iter().take(MAX_ENTRIES) {
        let correct = entry.correct.trim().to_string();
        if correct.is_empty() {
            continue;
        }

        let mut local_seen: HashSet<String> = HashSet::new();
        let mut variants = Vec::new();
        for variant in entry.variants {
            let normalized = match normalize_variant_input(&variant) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if local_seen.contains(&normalized) || used_variants.contains(&normalized) {
                continue;
            }
            local_seen.insert(normalized.clone());
            used_variants.insert(normalized.clone());
            variants.push(normalized);
            if variants.len() >= MAX_VARIANTS_PER_ENTRY {
                break;
            }
        }

        if variants.is_empty() {
            continue;
        }

        sanitized.push(CorrectionEntry {
            correct,
            variants,
            updated_at: if entry.updated_at.trim().is_empty() {
                now_rfc3339()
            } else {
                entry.updated_at
            },
            hit_count: entry.hit_count,
        });
    }

    CorrectionStore {
        version: CORRECTIONS_VERSION,
        corrections: sanitized,
    }
}

fn backup_invalid_store(path: &Path, original_content: &str) {
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let backup_name = format!("corrections.bak.{}.json", timestamp);
    let backup_path = path.with_file_name(backup_name);

    if std::fs::rename(path, &backup_path).is_ok() {
        return;
    }

    let _ = std::fs::write(&backup_path, original_content);
}

pub fn corrections_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取配置目录失败: {:?}", e))?;
    Ok(dir.join(CORRECTIONS_FILENAME))
}

pub fn load_corrections(app: &tauri::AppHandle) -> CorrectionStore {
    let path = match corrections_path(app) {
        Ok(path) => path,
        Err(_) => return CorrectionStore::default(),
    };

    if !path.exists() {
        return CorrectionStore::default();
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return CorrectionStore::default(),
    };

    match serde_json::from_str::<CorrectionStore>(&content) {
        Ok(store) => sanitize_store(store),
        Err(err) => {
            eprintln!("load_corrections: parse failed {:?}: {:?}", path, err);
            backup_invalid_store(&path, &content);
            CorrectionStore::default()
        }
    }
}

pub fn save_corrections(app: &tauri::AppHandle, store: &CorrectionStore) -> Result<(), String> {
    let path = corrections_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {:?}", e))?;
    }

    let sanitized = sanitize_store(store.clone());
    let json =
        serde_json::to_string_pretty(&sanitized).map_err(|e| format!("序列化失败: {:?}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("保存失败: {:?}", e))
}

pub fn add_correction(store: &mut CorrectionStore, wrong: &str, correct: &str) -> Result<(), String> {
    let variant = normalize_variant_input(wrong)?;
    let correct_trimmed = correct.trim().to_string();
    if correct_trimmed.is_empty() {
        return Err("正确词不能为空".to_string());
    }

    *store = sanitize_store(store.clone());
    store.version = CORRECTIONS_VERSION;

    for entry in &mut store.corrections {
        entry.variants.retain(|v| v != &variant);
    }
    store.corrections.retain(|entry| !entry.variants.is_empty());

    if let Some(existing) = store
        .corrections
        .iter_mut()
        .find(|entry| entry.correct.eq_ignore_ascii_case(&correct_trimmed))
    {
        if !existing.variants.contains(&variant) {
            if existing.variants.len() >= MAX_VARIANTS_PER_ENTRY {
                return Err(format!(
                    "每个正确词最多绑定 {} 个错误变体",
                    MAX_VARIANTS_PER_ENTRY
                ));
            }
            existing.variants.push(variant);
        }
        existing.correct = correct_trimmed;
        existing.updated_at = now_rfc3339();
        return Ok(());
    }

    if store.corrections.len() >= MAX_ENTRIES {
        return Err(format!("易错词条目最多 {} 条", MAX_ENTRIES));
    }

    store.corrections.push(CorrectionEntry {
        correct: correct_trimmed,
        variants: vec![variant],
        updated_at: now_rfc3339(),
        hit_count: 0,
    });

    Ok(())
}

pub fn remove_correction(store: &mut CorrectionStore, correct: &str) -> bool {
    let correct_trimmed = correct.trim();
    if correct_trimmed.is_empty() {
        return false;
    }

    let before = store.corrections.len();
    store
        .corrections
        .retain(|entry| !entry.correct.eq_ignore_ascii_case(correct_trimmed));
    before != store.corrections.len()
}

pub fn remove_correction_variant(store: &mut CorrectionStore, correct: &str, variant: &str) -> bool {
    let variant = match normalize_variant_input(variant) {
        Ok(variant) => variant,
        Err(_) => return false,
    };
    let correct_trimmed = correct.trim();
    if correct_trimmed.is_empty() {
        return false;
    }

    let mut changed = false;
    for entry in &mut store.corrections {
        if !entry.correct.eq_ignore_ascii_case(correct_trimmed) {
            continue;
        }
        let before = entry.variants.len();
        entry.variants.retain(|item| item != &variant);
        if before != entry.variants.len() {
            changed = true;
            entry.updated_at = now_rfc3339();
        }
        break;
    }

    if changed {
        store.corrections.retain(|entry| !entry.variants.is_empty());
    }

    changed
}

fn lowercase_with_span_map(text: &str) -> (String, Vec<(usize, usize)>) {
    let mut lowered = String::with_capacity(text.len());
    let mut spans: Vec<(usize, usize)> = Vec::with_capacity(text.len());

    for (start, ch) in text.char_indices() {
        let end = start + ch.len_utf8();
        let segment: String = ch.to_lowercase().collect();
        for _ in 0..segment.len() {
            spans.push((start, end));
        }
        lowered.push_str(&segment);
    }

    (lowered, spans)
}

fn lowered_range_to_original(
    spans: &[(usize, usize)],
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    if start >= end || end > spans.len() {
        return None;
    }
    let original_start = spans[start].0;
    let original_end = spans[end - 1].1;
    Some((original_start, original_end))
}

fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn needs_left_boundary(variant: &str) -> bool {
    variant
        .trim()
        .chars()
        .next()
        .map(is_token_char)
        .unwrap_or(false)
}

fn needs_right_boundary(variant: &str) -> bool {
    variant
        .trim()
        .chars()
        .next_back()
        .map(is_token_char)
        .unwrap_or(false)
}

fn passes_boundaries(
    text: &str,
    start: usize,
    end: usize,
    check_left: bool,
    check_right: bool,
) -> bool {
    if check_left && start > 0 {
        let prev = text[..start].chars().next_back();
        if prev.map(is_token_char).unwrap_or(false) {
            return false;
        }
    }

    if check_right && end < text.len() {
        let next = text[end..].chars().next();
        if next.map(is_token_char).unwrap_or(false) {
            return false;
        }
    }

    true
}

pub fn apply_corrections(text: &str, store: &CorrectionStore) -> ApplyCorrectionsResult {
    if text.is_empty() || store.corrections.is_empty() {
        return ApplyCorrectionsResult {
            text: text.to_string(),
            hits: Vec::new(),
        };
    }

    #[derive(Clone)]
    struct PatternMeta {
        variant: String,
        correct: String,
        check_left: bool,
        check_right: bool,
    }

    let mut patterns = Vec::new();
    let mut metadata = Vec::new();

    for entry in &store.corrections {
        if entry.correct.trim().is_empty() {
            continue;
        }
        for variant in &entry.variants {
            if variant.is_empty() {
                continue;
            }
            patterns.push(variant.clone());
            metadata.push(PatternMeta {
                variant: variant.clone(),
                correct: entry.correct.clone(),
                check_left: needs_left_boundary(variant),
                check_right: needs_right_boundary(variant),
            });
        }
    }

    if patterns.is_empty() {
        return ApplyCorrectionsResult {
            text: text.to_string(),
            hits: Vec::new(),
        };
    }

    let ac = match AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .build(&patterns)
    {
        Ok(ac) => ac,
        Err(err) => {
            eprintln!("apply_corrections: build matcher failed: {:?}", err);
            return ApplyCorrectionsResult {
                text: text.to_string(),
                hits: Vec::new(),
            };
        }
    };

    let (lowered, spans) = lowercase_with_span_map(text);
    let mut hits = Vec::new();

    for mat in ac.find_iter(&lowered) {
        let pattern_idx = mat.pattern().as_usize();
        let meta = match metadata.get(pattern_idx) {
            Some(meta) => meta,
            None => continue,
        };

        let (start, end) = match lowered_range_to_original(&spans, mat.start(), mat.end()) {
            Some(range) => range,
            None => continue,
        };

        if !passes_boundaries(text, start, end, meta.check_left, meta.check_right) {
            continue;
        }

        hits.push(CorrectionHit {
            variant: meta.variant.clone(),
            correct: meta.correct.clone(),
            start,
            end,
        });
    }

    if hits.is_empty() {
        return ApplyCorrectionsResult {
            text: text.to_string(),
            hits,
        };
    }

    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for hit in &hits {
        if hit.start < cursor || hit.end > text.len() {
            continue;
        }
        output.push_str(&text[cursor..hit.start]);
        output.push_str(&hit.correct);
        cursor = hit.end;
    }
    output.push_str(&text[cursor..]);

    ApplyCorrectionsResult { text: output, hits }
}

pub fn increment_hit_counts(store: &mut CorrectionStore, hits: &[CorrectionHit]) -> bool {
    if hits.is_empty() {
        return false;
    }

    let mut changed = false;
    for hit in hits {
        if let Some(entry) = store
            .corrections
            .iter_mut()
            .find(|entry| entry.correct.eq_ignore_ascii_case(&hit.correct))
        {
            entry.hit_count = entry.hit_count.saturating_add(1);
            entry.updated_at = now_rfc3339();
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_case_insensitive() {
        let mut store = CorrectionStore::default();
        add_correction(&mut store, "gemeni", "Gemini").unwrap();

        let result = apply_corrections("GEMENI and gemeni", &store);
        assert_eq!(result.text, "Gemini and Gemini");
        assert_eq!(result.hits.len(), 2);
    }

    #[test]
    fn longest_pattern_wins() {
        let mut store = CorrectionStore::default();
        add_correction(&mut store, "mac book", "MacBook").unwrap();
        add_correction(&mut store, "mac book pro", "MacBook Pro").unwrap();

        let result = apply_corrections("I use mac book pro daily.", &store);
        assert_eq!(result.text, "I use MacBook Pro daily.");
    }

    #[test]
    fn mixed_cjk_and_ascii_replacement() {
        let mut store = CorrectionStore::default();
        add_correction(&mut store, "gemeni", "Gemini").unwrap();
        add_correction(&mut store, "k8s", "Kubernetes").unwrap();

        let result = apply_corrections("我用gemeni部署k8s", &store);
        assert_eq!(result.text, "我用Gemini部署Kubernetes");
    }

    #[test]
    fn boundary_prevents_substring_replacement() {
        let mut store = CorrectionStore::default();
        add_correction(&mut store, "cat", "CAT").unwrap();

        let result = apply_corrections("catalog cat scat cat.", &store);
        assert_eq!(result.text, "catalog CAT scat CAT.");
    }

    #[test]
    fn unicode_safe_with_emoji() {
        let mut store = CorrectionStore::default();
        add_correction(&mut store, "哈哈", "呵呵").unwrap();

        let result = apply_corrections("emoji🙂哈哈", &store);
        assert_eq!(result.text, "emoji🙂呵呵");
    }

    #[test]
    fn variant_migrates_between_correct_entries() {
        let mut store = CorrectionStore::default();
        add_correction(&mut store, "gimini", "Gemini").unwrap();
        add_correction(&mut store, "gimini", "Gemma").unwrap();

        assert_eq!(store.corrections.len(), 1);
        assert_eq!(store.corrections[0].correct, "Gemma");
        assert_eq!(store.corrections[0].variants, vec!["gimini".to_string()]);
    }
}
