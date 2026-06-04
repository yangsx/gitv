use serde::{Deserialize, Serialize};

use crate::models::{DateRange, Oid};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub use_regex: bool,
    pub sha_prefix: Option<String>,
    pub author: Option<String>,
    pub date_range: Option<DateRange>,
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    Message,
    Sha,
    Author,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Highlight {
    pub start: usize,
    pub length: usize,
}
