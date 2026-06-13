use serde::{Deserialize, Serialize};

use crate::models::{DateRange, Oid};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub use_regex: bool,
    #[serde(default)]
    pub search_patch: bool,
    pub sha_prefix: Option<String>,
    pub author: Option<String>,
    pub date_range: Option<DateRange>,
    pub file_path: Option<String>,
    pub combine_mode: CombineMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombineMode {
    #[default]
    And,
    Or,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub commit_oid: Oid,
    pub match_type: MatchType,
    pub highlights: Vec<Highlight>,
    #[serde(default)]
    pub patch_matches: Vec<PatchMatchLocation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchMatchLocation {
    pub file_path: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub matched_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    Message,
    Sha,
    Author,
    Patch,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Highlight {
    pub start: usize,
    pub length: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub patch_search_id: Option<u64>,
    pub patch_search_total: Option<u64>,
}
