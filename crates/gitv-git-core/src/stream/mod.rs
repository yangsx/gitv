use crate::error::GitError;
use crate::models::*;
use crate::repository::Repository;

pub struct CommitStream {
    repo: Box<dyn Repository>,
    filter: CommitFilter,
    buffer: Vec<CommitInfo>,
    exhausted: bool,
    walk_initialized: bool,
    head_consumed: usize,
    extra_tips: Vec<Oid>,
}

impl CommitStream {
    pub fn new(repo: Box<dyn Repository>, filter: CommitFilter) -> Self {
        Self {
            repo,
            filter,
            buffer: Vec::new(),
            exhausted: false,
            walk_initialized: false,
            head_consumed: 0,
            extra_tips: Vec::new(),
        }
    }

    pub fn with_extra_tips(mut self, extra_tips: Vec<Oid>) -> Self {
        self.extra_tips = extra_tips;
        self
    }

    pub fn has_more(&self) -> bool {
        !self.exhausted || self.head_consumed < self.buffer.len()
    }

    pub fn next_batch(&mut self, count: usize) -> Result<Option<Vec<CommitInfo>>, GitError> {
        if !self.walk_initialized {
            self.buffer = self.repo.commits(None, &self.extra_tips)?;
            self.walk_initialized = true;
            if self.buffer.is_empty() {
                self.exhausted = true;
                return Ok(None);
            }
            self.apply_filters();
        }

        let remaining = self.buffer.len().saturating_sub(self.head_consumed);
        if remaining == 0 {
            self.exhausted = true;
            return Ok(None);
        }

        let take = count.min(remaining);
        let start = self.head_consumed;
        let end = start + take;
        let batch: Vec<CommitInfo> = self.buffer[start..end].to_vec();
        self.head_consumed = end;

        if self.head_consumed >= self.buffer.len() {
            self.exhausted = true;
        }

        Ok(Some(batch))
    }

    pub fn cancel(&mut self) {
        self.exhausted = true;
    }

    fn apply_filters(&mut self) {
        if self.filter.hide_merges {
            self.buffer.retain(|c| c.parent_oids.len() <= 1);
        }
        if let Some(ref author) = self.filter.author {
            let pattern = author.to_lowercase();
            self.buffer.retain(|c| {
                c.author.name.to_lowercase().contains(&pattern)
                    || c.author.email.to_lowercase().contains(&pattern)
            });
        }
        if let Some(ref range) = self.filter.date_range {
            self.buffer.retain(|c| {
                if let Some(from) = range.from
                    && c.author_time < from
                {
                    return false;
                }
                if let Some(to) = range.to
                    && c.author_time > to
                {
                    return false;
                }
                true
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gix_repo::GixRepository;
    use std::path::Path;
    use std::process::Command;

    struct TempRepo {
        dir: tempfile::TempDir,
    }

    impl TempRepo {
        fn new() -> Self {
            let dir = tempfile::TempDir::new().expect("temp dir");
            let path = dir.path();
            run_git(path, &["init"]);
            run_git(path, &["config", "user.name", "Test"]);
            run_git(path, &["config", "user.email", "test@test.com"]);
            Self { dir }
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }

        fn commit_file(&self, name: &str, content: &str, msg: &str) {
            let file_path = self.dir.path().join(name);
            std::fs::write(&file_path, content).expect("write file");
            run_git(self.path(), &["add", name]);
            run_git(self.path(), &["commit", "-m", msg]);
        }

        fn make_merge(&self, branch: &str, msg: &str) {
            let default_branch = self.default_branch_name();
            run_git(self.path(), &["checkout", "-b", branch]);
            let file_path = self.dir.path().join(format!("{branch}.txt"));
            std::fs::write(&file_path, branch).expect("write file");
            run_git(self.path(), &["add", &format!("{branch}.txt")]);
            run_git(self.path(), &["commit", "-m", &format!("{branch} commit")]);
            run_git(self.path(), &["checkout", &default_branch]);
            run_git(self.path(), &["merge", branch, "-m", msg]);
        }

        fn default_branch_name(&self) -> String {
            let output = Command::new("git")
                .args(["symbolic-ref", "--short", "HEAD"])
                .current_dir(self.path())
                .output()
                .expect("git symbolic-ref");
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_DATE", "2025-01-01T00:00:00+0000")
            .env("GIT_COMMITTER_DATE", "2025-01-01T00:00:00+0000")
            .status()
            .expect("git command");
        assert!(status.success(), "git {:?} failed", args);
    }

    fn open_repo(path: &Path) -> Box<dyn Repository> {
        let repo = GixRepository::open(path).expect("open repo");
        Box::new(repo)
    }

    #[test]
    fn batch_yields_correct_count() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "1", "commit 1");
        temp.commit_file("b.txt", "2", "commit 2");
        temp.commit_file("c.txt", "3", "commit 3");
        let repo = open_repo(temp.path());
        let mut stream = CommitStream::new(repo, CommitFilter::default());
        let batch = stream.next_batch(2).expect("batch").expect("some");
        assert_eq!(batch.len(), 2);
        let batch2 = stream.next_batch(2).expect("batch").expect("some");
        assert_eq!(batch2.len(), 1);
    }

    #[test]
    fn has_more_is_false_when_exhausted() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "1", "commit 1");
        let repo = open_repo(temp.path());
        let mut stream = CommitStream::new(repo, CommitFilter::default());
        assert!(stream.has_more());
        let batch = stream.next_batch(10).expect("batch").expect("some");
        assert_eq!(batch.len(), 1);
        assert!(!stream.has_more());
        let result = stream.next_batch(10).expect("batch");
        assert!(result.is_none());
    }

    #[test]
    fn cancel_stops_iteration() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "1", "commit 1");
        temp.commit_file("b.txt", "2", "commit 2");
        let repo = open_repo(temp.path());
        let mut stream = CommitStream::new(repo, CommitFilter::default());
        stream.cancel();
        assert!(!stream.has_more());
    }

    #[test]
    fn hide_merges_filter() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "1", "initial");
        temp.make_merge("feature", "merge feature");
        let repo = open_repo(temp.path());
        let filter = CommitFilter {
            hide_merges: true,
            ..CommitFilter::default()
        };
        let mut stream = CommitStream::new(repo, filter);
        let batch = stream.next_batch(10).expect("batch").expect("some");
        assert!(batch.iter().all(|c| c.parent_oids.len() <= 1));
    }

    #[test]
    fn empty_repo_yields_none() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        run_git(dir.path(), &["init"]);
        run_git(dir.path(), &["config", "user.name", "Test"]);
        run_git(dir.path(), &["config", "user.email", "test@test.com"]);
        let repo = GixRepository::open(dir.path()).expect("open");
        let mut stream = CommitStream::new(Box::new(repo), CommitFilter::default());
        let result = stream.next_batch(10).expect("should not error");
        assert!(result.is_none(), "empty repo should yield None");
    }
}
