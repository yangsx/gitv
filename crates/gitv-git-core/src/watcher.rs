use notify_debouncer_full::notify;
use notify_debouncer_full::{Debouncer, FileIdMap, new_debouncer};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WatchError {
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchEvent {
    RefsChanged,
    HeadChanged,
    IndexChanged,
}

#[derive(Debug)]
pub struct RepositoryWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, FileIdMap>,
}

impl RepositoryWatcher {
    pub fn start(repo_git_dir: &Path, tx: mpsc::Sender<WatchEvent>) -> Result<Self, WatchError> {
        use notify::RecursiveMode;
        use notify::Watcher;

        let repo_git_dir = repo_git_dir.to_path_buf();

        let head_path = repo_git_dir.join("HEAD");
        let refs_path = repo_git_dir.join("refs");
        let index_path = repo_git_dir.join("index");

        let (notify_tx, notify_rx) = mpsc::channel::<notify_debouncer_full::DebounceEventResult>();

        let mut debouncer = new_debouncer(Duration::from_millis(500), None, notify_tx)?;

        if head_path.exists() {
            debouncer
                .watcher()
                .watch(&head_path, RecursiveMode::NonRecursive)?;
        }

        if refs_path.exists() {
            debouncer
                .watcher()
                .watch(&refs_path, RecursiveMode::Recursive)?;
        }

        if index_path.exists() {
            debouncer
                .watcher()
                .watch(&index_path, RecursiveMode::NonRecursive)?;
        }

        std::thread::spawn(move || {
            for result in notify_rx {
                match result {
                    Ok(events) => {
                        for event in events {
                            let event_path = event.paths.first().cloned();
                            let kind = &event.kind;

                            let is_modify = matches!(kind, notify::EventKind::Modify(_));
                            let is_create = matches!(kind, notify::EventKind::Create(_));

                            let watch_event = match event_path {
                                Some(p) if p == head_path => {
                                    if is_modify || is_create {
                                        Some(WatchEvent::HeadChanged)
                                    } else {
                                        None
                                    }
                                }
                                Some(p) if p.starts_with(&refs_path) => {
                                    if is_modify || is_create {
                                        Some(WatchEvent::RefsChanged)
                                    } else {
                                        None
                                    }
                                }
                                Some(p) if p == index_path => {
                                    if is_modify {
                                        Some(WatchEvent::IndexChanged)
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };

                            if let Some(we) = watch_event
                                && tx.send(we).is_err()
                            {
                                return;
                            }
                        }
                    }
                    Err(errors) => {
                        for e in &errors {
                            tracing::warn!("watcher error: {e}");
                        }
                    }
                }
            }
        });

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}
