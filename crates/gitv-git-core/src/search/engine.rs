use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use roaring::RoaringBitmap;
use std::collections::HashMap;
use std::ops::BitOrAssign;

use crate::error::SearchError;
use crate::models::{CommitInfo, DateRange, Oid};

use super::query::*;

pub struct SearchEngine {
    commits: Vec<CommitInfo>,
    message_index: CommitMessageIndex,
    author_index: HashMap<String, RoaringBitmap>,
}

struct CommitMessageIndex {
    postings: HashMap<String, RoaringBitmap>,
}

impl CommitMessageIndex {
    fn new() -> Self {
        Self {
            postings: HashMap::new(),
        }
    }

    fn index_commits(&mut self, commits: &[CommitInfo], start_idx: u32) {
        // Build per-commit postings in parallel, then merge
        let per_commit: Vec<HashMap<String, RoaringBitmap>> = commits
            .par_iter()
            .enumerate()
            .map(|(i, commit)| {
                let idx = start_idx + i as u32;
                let mut local: HashMap<String, RoaringBitmap> = HashMap::new();

                // Index message tokens
                let terms = tokenize(&commit.message);
                let unique_terms: std::collections::HashSet<&str> = terms.into_iter().collect();
                for term in unique_terms {
                    local.entry(term.to_lowercase()).or_default().insert(idx);
                }

                // Index summary tokens
                let summary_terms = tokenize(&commit.summary);
                let unique_summary: std::collections::HashSet<&str> =
                    summary_terms.into_iter().collect();
                for term in unique_summary {
                    local.entry(term.to_lowercase()).or_default().insert(idx);
                }

                local
            })
            .collect();

        // Merge parallel results into self.postings
        for local in per_commit {
            for (term, bitmap) in local {
                self.postings.entry(term).or_default().bitor_assign(&bitmap);
            }
        }
    }

    fn search_term(&self, term: &str) -> Option<&RoaringBitmap> {
        self.postings.get(&term.to_lowercase())
    }
}

fn tokenize(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect()
}

impl SearchEngine {
    #[must_use]
    pub fn new(commits: Vec<CommitInfo>) -> Self {
        let mut message_index = CommitMessageIndex::new();
        message_index.index_commits(&commits, 0);

        // Build author index in parallel
        let per_commit_author: Vec<HashMap<String, RoaringBitmap>> = commits
            .par_iter()
            .enumerate()
            .map(|(i, commit)| {
                let idx = i as u32;
                let mut local: HashMap<String, RoaringBitmap> = HashMap::new();
                let author_key = format!(
                    "{} {}",
                    commit.author.name.to_lowercase(),
                    commit.author.email.to_lowercase()
                );
                for word in author_key.split_whitespace() {
                    local.entry(word.to_string()).or_default().insert(idx);
                }
                local
            })
            .collect();

        let mut author_index: HashMap<String, RoaringBitmap> = HashMap::new();
        for local in per_commit_author {
            for (term, bitmap) in local {
                author_index.entry(term).or_default().bitor_assign(&bitmap);
            }
        }

        Self {
            commits,
            message_index,
            author_index,
        }
    }

    pub fn commit_oids(&self) -> Vec<Oid> {
        self.commits.iter().map(|c| c.oid).collect()
    }

    pub fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SearchError> {
        let mut result_bitmap: Option<RoaringBitmap> = None;
        let mut compiled_regex: Option<Regex> = None;

        let text = query.text.as_ref().filter(|t| !t.is_empty());
        let sha_prefix = query.sha_prefix.as_ref().filter(|p| !p.is_empty());

        match (text, sha_prefix) {
            (Some(text), Some(sha_prefix)) => {
                let text_results = if query.use_regex {
                    let re = Regex::new(text)?;
                    compiled_regex = Some(re.clone());
                    self.search_regex_with(&re)
                } else {
                    self.search_text(text)
                };
                let sha_results = self.search_sha(sha_prefix);
                result_bitmap = Some(text_results | sha_results);
            }
            (Some(text), None) => {
                let text_results = if query.use_regex {
                    let re = Regex::new(text)?;
                    compiled_regex = Some(re.clone());
                    self.search_regex_with(&re)
                } else {
                    self.search_text(text)
                };
                result_bitmap = Some(Self::combine(
                    result_bitmap,
                    text_results,
                    query.combine_mode,
                ));
            }
            (None, Some(sha_prefix)) => {
                let sha_results = self.search_sha(sha_prefix);
                result_bitmap = Some(Self::combine(
                    result_bitmap,
                    sha_results,
                    query.combine_mode,
                ));
            }
            (None, None) => {}
        }

        if let Some(ref author) = query.author
            && !author.is_empty()
        {
            let author_results = self.search_author(author);
            result_bitmap = Some(Self::combine(
                result_bitmap,
                author_results,
                query.combine_mode,
            ));
        }

        if let Some(ref range) = query.date_range {
            let date_results = self.search_date_range(range);
            result_bitmap = Some(Self::combine(
                result_bitmap,
                date_results,
                query.combine_mode,
            ));
        }

        if let Some(ref file_path) = query.file_path
            && !file_path.is_empty()
        {
            let file_results = self.search_message_contains(file_path);
            result_bitmap = Some(Self::combine(
                result_bitmap,
                file_results,
                query.combine_mode,
            ));
        }

        let bitmap = result_bitmap.unwrap_or_else(|| {
            let mut all = RoaringBitmap::new();
            for i in 0..self.commits.len() as u32 {
                all.insert(i);
            }
            all
        });

        let mut results = Vec::new();
        for idx in bitmap {
            if let Some(commit) = self.commits.get(idx as usize) {
                let match_type = self.determine_match_type(query, commit);
                let highlights = self.compute_highlights(query, commit, compiled_regex.as_ref());
                results.push(SearchResult {
                    commit_oid: commit.oid,
                    match_type,
                    highlights,
                    patch_matches: Vec::new(),
                });
            }
        }

        Ok(results)
    }

    /// Add new commits to the index. Caller must ensure no duplicates
    /// (commits already passed to `new()` or a previous `ensure_indexed` call).
    pub fn ensure_indexed(&mut self, new_commits: &[CommitInfo]) {
        let start_idx = self.commits.len() as u32;
        self.message_index.index_commits(new_commits, start_idx);

        for (i, commit) in new_commits.iter().enumerate() {
            let idx = start_idx + i as u32;
            let author_key = format!(
                "{} {}",
                commit.author.name.to_lowercase(),
                commit.author.email.to_lowercase()
            );
            for word in author_key.split_whitespace() {
                self.author_index
                    .entry(word.to_string())
                    .or_default()
                    .insert(idx);
            }
        }

        self.commits.extend(new_commits.iter().cloned());
    }

    #[must_use]
    pub fn commit_count(&self) -> usize {
        self.commits.len()
    }

    fn search_text(&self, text: &str) -> RoaringBitmap {
        let terms = tokenize(text);
        if terms.is_empty() {
            return RoaringBitmap::new();
        }

        let mut result: Option<RoaringBitmap> = None;
        for term in terms {
            let lower = term.to_lowercase();
            let matches = self.message_index.search_term(&lower);
            match (result, matches) {
                (None, Some(bm)) => result = Some(bm.clone()),
                (None, None) => return RoaringBitmap::new(),
                (Some(mut acc), Some(bm)) => {
                    acc &= bm;
                    result = Some(acc);
                }
                (Some(_), None) => return RoaringBitmap::new(),
            }
        }

        result.unwrap_or_default()
    }

    fn search_regex_with(&self, re: &Regex) -> RoaringBitmap {
        let mut result = RoaringBitmap::new();
        for (i, commit) in self.commits.iter().enumerate() {
            if re.is_match(&commit.message) || re.is_match(&commit.summary) {
                result.insert(i as u32);
            }
        }
        result
    }

    fn search_sha(&self, prefix: &str) -> RoaringBitmap {
        let mut result = RoaringBitmap::new();
        for (i, commit) in self.commits.iter().enumerate() {
            if commit.oid.starts_with_hex(prefix) {
                result.insert(i as u32);
            }
        }
        result
    }

    fn search_author(&self, author: &str) -> RoaringBitmap {
        let lower = author.to_lowercase();
        let terms: Vec<&str> = lower.split_whitespace().collect();
        if terms.is_empty() {
            return RoaringBitmap::new();
        }

        let mut result: Option<RoaringBitmap> = None;
        for term in terms {
            let matches = self.author_index.get(term);
            match (result, matches) {
                (None, Some(bm)) => result = Some(bm.clone()),
                (None, None) => return RoaringBitmap::new(),
                (Some(mut acc), Some(bm)) => {
                    acc &= bm;
                    result = Some(acc);
                }
                (Some(_), None) => return RoaringBitmap::new(),
            }
        }

        result.unwrap_or_default()
    }

    /// Searches commit message and summary text.
    /// NOTE: Despite the name `file_path` in the query, this currently searches
    /// commit messages, not actual changed file paths. True file-path search
    /// would require indexing `CommitDetails.changed_files` which is not
    /// available in the search engine's `CommitInfo` index.
    fn search_message_contains(&self, text: &str) -> RoaringBitmap {
        let lower = text.to_lowercase();
        let mut result = RoaringBitmap::new();
        for (i, commit) in self.commits.iter().enumerate() {
            if commit.message.to_lowercase().contains(&lower)
                || commit.summary.to_lowercase().contains(&lower)
            {
                result.insert(i as u32);
            }
        }
        result
    }

    fn search_date_range(&self, range: &DateRange) -> RoaringBitmap {
        let mut result = RoaringBitmap::new();
        for (i, commit) in self.commits.iter().enumerate() {
            if let Some(from) = range.from
                && commit.commit_time < from
            {
                continue;
            }
            if let Some(to) = range.to
                && commit.commit_time > to
            {
                continue;
            }
            result.insert(i as u32);
        }
        result
    }

    fn combine(
        existing: Option<RoaringBitmap>,
        new: RoaringBitmap,
        mode: CombineMode,
    ) -> RoaringBitmap {
        match (existing, mode) {
            (None, _) => new,
            (Some(acc), CombineMode::And) => acc & new,
            (Some(acc), CombineMode::Or) => acc | new,
        }
    }

    fn determine_match_type(&self, query: &SearchQuery, commit: &CommitInfo) -> MatchType {
        if let Some(ref prefix) = query.sha_prefix
            && !prefix.is_empty()
            && commit.oid.starts_with_hex(prefix)
        {
            return MatchType::Sha;
        }
        if let Some(ref author) = query.author
            && !author.is_empty()
        {
            let lower = author.to_lowercase();
            if commit.author.name.to_lowercase().contains(&lower)
                || commit.author.email.to_lowercase().contains(&lower)
            {
                return MatchType::Author;
            }
        }
        MatchType::Message
    }

    fn compute_highlights(
        &self,
        query: &SearchQuery,
        commit: &CommitInfo,
        compiled_regex: Option<&Regex>,
    ) -> Vec<Highlight> {
        let mut highlights = Vec::new();

        if let Some(ref text) = query.text
            && !text.is_empty()
        {
            if let Some(re) = compiled_regex {
                for mat in re.find_iter(&commit.message) {
                    highlights.push(Highlight {
                        start: mat.start(),
                        length: mat.len(),
                    });
                }
            } else {
                let lower_msg = commit.message.to_lowercase();
                let lower_text = text.to_lowercase();
                let mut start = 0;
                while let Some(pos) = lower_msg[start..].find(&lower_text) {
                    let abs_pos = start + pos;
                    highlights.push(Highlight {
                        start: abs_pos,
                        length: text.len(),
                    });
                    start = abs_pos + 1;
                }
            }
        }

        highlights
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use chrono::{TimeZone, Utc};

    fn make_oid(n: u8) -> Oid {
        let mut bytes = [0u8; 20];
        bytes[0] = n;
        Oid::from_bytes(bytes)
    }

    fn make_commit(oid: u8, parents: Vec<u8>, msg: &str, author_name: &str) -> CommitInfo {
        CommitInfo {
            oid: make_oid(oid),
            short_oid: format!("{oid:02x}00000"),
            message: msg.to_string(),
            summary: msg.lines().next().unwrap_or("").to_string(),
            author: Author {
                name: author_name.to_string(),
                email: format!("{author_name}@test.com"),
            },
            committer: Author {
                name: author_name.to_string(),
                email: format!("{author_name}@test.com"),
            },
            author_time: Utc.timestamp_opt(1000 + oid as i64, 0).single().unwrap(),
            commit_time: Utc.timestamp_opt(1000 + oid as i64, 0).single().unwrap(),
            parent_oids: parents.into_iter().map(make_oid).collect(),
            refs: Vec::new(),
        }
    }

    fn make_commits() -> Vec<CommitInfo> {
        vec![
            make_commit(1, vec![], "initial commit", "alice"),
            make_commit(2, vec![1], "add feature X", "bob"),
            make_commit(3, vec![2], "fix bug in feature X", "alice"),
            make_commit(4, vec![3], "update documentation", "charlie"),
            make_commit(5, vec![4], "refactor core module", "bob"),
        ]
    }

    #[test]
    fn empty_query_returns_all() {
        let engine = SearchEngine::new(make_commits());
        let results = engine.search(&SearchQuery::default()).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn text_search_finds_match() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("feature".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.match_type == MatchType::Message));
    }

    #[test]
    fn text_search_is_case_insensitive() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("FEATURE".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn sha_prefix_search() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            sha_prefix: Some("01".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit_oid, make_oid(1));
        assert_eq!(results[0].match_type, MatchType::Sha);
    }

    #[test]
    fn text_and_sha_prefix_or_combined() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("feature".to_string()),
            sha_prefix: Some("01".to_string()),
            combine_mode: CombineMode::And,
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        // "feature" matches commits 2 and 3; sha_prefix "01" matches commit 1.
        // text+sha are Or-combined regardless of combine_mode.
        let oids: Vec<_> = results.iter().map(|r| r.commit_oid).collect();
        assert_eq!(oids.len(), 3);
        assert!(oids.contains(&make_oid(1)));
        assert!(oids.contains(&make_oid(2)));
        assert!(oids.contains(&make_oid(3)));
    }

    #[test]
    fn text_and_sha_prefix_or_combined_with_author_filter() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("feature".to_string()),
            sha_prefix: Some("01".to_string()),
            author: Some("alice".to_string()),
            combine_mode: CombineMode::And,
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        // (text "feature" OR sha "01") AND author "alice"
        // text matches 2,3; sha matches 1 → union = {1,2,3}
        // author alice → {1,3} → intersection = {1,3}
        let oids: Vec<_> = results.iter().map(|r| r.commit_oid).collect();
        assert_eq!(oids.len(), 2);
        assert!(oids.contains(&make_oid(1)));
        assert!(oids.contains(&make_oid(3)));
    }

    #[test]
    fn author_search() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            author: Some("alice".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.match_type == MatchType::Author));
    }

    #[test]
    fn date_range_filter() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            date_range: Some(DateRange {
                from: Some(Utc.timestamp_opt(1002, 0).single().unwrap()),
                to: Some(Utc.timestamp_opt(1004, 0).single().unwrap()),
            }),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn combined_text_and_author_and_mode() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("feature".to_string()),
            author: Some("alice".to_string()),
            combine_mode: CombineMode::And,
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit_oid, make_oid(3));
    }

    #[test]
    fn combined_text_and_author_or_mode() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("documentation".to_string()),
            author: Some("alice".to_string()),
            combine_mode: CombineMode::Or,
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert!(results.len() >= 3);
    }

    #[test]
    fn no_results_for_non_matching() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("nonexistent".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn incremental_indexing() {
        let mut engine = SearchEngine::new(make_commits());
        assert_eq!(engine.commit_count(), 5);

        let new_commits = vec![make_commit(6, vec![5], "add new widget system", "dave")];
        engine.ensure_indexed(&new_commits);
        assert_eq!(engine.commit_count(), 6);

        let query = SearchQuery {
            text: Some("widget system".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit_oid, make_oid(6));
    }

    #[test]
    fn regex_search() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some(r"feature \w+".to_string()),
            use_regex: true,
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn invalid_regex_returns_error() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("[invalid".to_string()),
            use_regex: true,
            ..Default::default()
        };
        assert!(engine.search(&query).is_err());
    }

    #[test]
    fn highlights_in_text_search() {
        let engine = SearchEngine::new(make_commits());
        let query = SearchQuery {
            text: Some("feature".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert!(results.iter().any(|r| !r.highlights.is_empty()));
    }

    #[test]
    fn mixed_case_messages_are_case_insensitive() {
        let commits = vec![
            make_commit(10, vec![], "Implement FeatureRequest module", "alice"),
            make_commit(11, vec![10], "Fix EdgeCase in Parser", "bob"),
            make_commit(12, vec![11], "update README", "charlie"),
        ];
        let engine = SearchEngine::new(commits);

        let query = SearchQuery {
            text: Some("featurerequest".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit_oid, make_oid(10));

        let query = SearchQuery {
            text: Some("edgecase".to_string()),
            ..Default::default()
        };
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].commit_oid, make_oid(11));
    }
}
