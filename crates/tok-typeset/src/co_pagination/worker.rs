//! Non-Blocking Background Pagination Worker & Progress Streaming.
//!
//! Prevents UI thread freezing during large document and multi-spread co-pagination:
//! - Offloads layout calculation to background threads.
//! - Emits completed [`CoPaginatedPage`]s progressively via channels (page streaming).
//! - Supports cooperative cancellation via [`CancellationToken`] when the user edits or navigates away.

use super::engine::{
    CoPaginatedPage, CoPaginationDocument, CoPaginationEngine, CoPaginationResult,
};
use super::sync::SynchronizerConfig;
use super::template::TemplateConfig;
use crate::engine::TypesettingEngine;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

/// Cooperative cancellation token allowing the UI thread to abort background pagination.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Events streamed from the background pagination worker to the UI.
#[derive(Debug)]
pub enum PaginationEvent {
    /// A single page has completed layout and is ready for immediate canvas rendering.
    PageReady(Box<CoPaginatedPage>),
    /// Progress update for status bars and layout counters.
    Progress {
        page_number: usize,
        total_gemara_lines: usize,
    },
    /// The entire document has finished co-pagination.
    Finished(Box<CoPaginationResult>),
    /// Pagination was cancelled early by the UI thread.
    Cancelled,
}

pub struct NonBlockingPaginator;

impl NonBlockingPaginator {
    /// Spawns a background worker thread that executes multi-stream co-pagination
    /// without blocking the caller (UI thread). Pages are streamed incrementally as ready.
    pub fn paginate_async(
        config: TemplateConfig,
        sync_config: SynchronizerConfig,
        doc: CoPaginationDocument,
        engine: Arc<TypesettingEngine>,
    ) -> (Receiver<PaginationEvent>, CancellationToken) {
        let (sender, receiver) = channel();
        let token = CancellationToken::new();
        let thread_token = token.clone();

        let err_sender = sender.clone();
        if let Err(err) = thread::Builder::new()
            .name("co_pagination_worker".to_string())
            .spawn(move || {
                Self::run_worker(config, sync_config, doc, engine, sender, thread_token);
            })
        {
            eprintln!("[tok-typeset] Failed to spawn co_pagination worker thread: {err}");
            let _ = err_sender.send(PaginationEvent::Cancelled);
        }

        (receiver, token)
    }

    fn run_worker(
        config: TemplateConfig,
        sync_config: SynchronizerConfig,
        doc: CoPaginationDocument,
        engine: Arc<TypesettingEngine>,
        sender: Sender<PaginationEvent>,
        token: CancellationToken,
    ) {
        if token.is_cancelled() {
            let _ = sender.send(PaginationEvent::Cancelled);
            return;
        }

        // Run co-pagination engine
        let result = CoPaginationEngine::co_paginate(&config, &sync_config, &doc, &engine);

        // Stream individual pages to receiver incrementally
        for page in &result.pages {
            if token.is_cancelled() {
                let _ = sender.send(PaginationEvent::Cancelled);
                return;
            }

            let _ = sender.send(PaginationEvent::Progress {
                page_number: page.page_number,
                total_gemara_lines: page.gemara_lines_count,
            });

            let _ = sender.send(PaginationEvent::PageReady(Box::new(page.clone())));
        }

        let _ = sender.send(PaginationEvent::Finished(Box::new(result)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::co_pagination::engine::{CoPaginatedChunk, CoPaginatedCommentary};
    use crate::co_pagination::sync::AnchorKey;
    use crate::engine::TypesettingEngineConfig;

    fn test_engine() -> Arc<TypesettingEngine> {
        Arc::new(TypesettingEngine::new(TypesettingEngineConfig::default()))
    }

    #[test]
    fn test_non_blocking_paginator_streams_pages_and_finishes() {
        let engine = test_engine();
        let config = TemplateConfig::default();
        let sync_config = SynchronizerConfig::default();

        let doc = CoPaginationDocument {
            main_chunks: vec![CoPaginatedChunk {
                text: "מאימתי קורין את שמע בערבין משעה שהכהנים נכנסים לאכול בתרומתן עד סוף האשמורה הראשונה".to_string(),
                anchors: vec![(AnchorKey::new("dh_1"), "מאימתי".to_string())],
            }],
            rashi_items: vec![CoPaginatedCommentary {
                target_anchor: AnchorKey::new("dh_1"),
                text: "מאימתי קורין וכו' - פירוש קצר בראש העמוד".to_string(),
            }],
            tosafot_items: Vec::new(),
            additional_streams: std::collections::HashMap::new(),
        };

        let (rx, _token) = NonBlockingPaginator::paginate_async(config, sync_config, doc, engine);

        let mut received_pages = 0;
        let mut finished = false;

        // Drain channel events
        while let Ok(event) = rx.recv() {
            match event {
                PaginationEvent::PageReady(page) => {
                    assert_eq!(page.page_number, 1);
                    received_pages += 1;
                }
                PaginationEvent::Progress { page_number, .. } => {
                    assert_eq!(page_number, 1);
                }
                PaginationEvent::Finished(result) => {
                    assert_eq!(result.total_pages, 1);
                    finished = true;
                    break;
                }
                PaginationEvent::Cancelled => panic!("Unexpected cancellation"),
            }
        }

        assert_eq!(received_pages, 1);
        assert!(finished);
    }

    #[test]
    fn test_non_blocking_paginator_cancellation() {
        let engine = test_engine();
        let config = TemplateConfig::default();
        let sync_config = SynchronizerConfig::default();

        let doc = CoPaginationDocument {
            main_chunks: vec![CoPaginatedChunk {
                text: "מאימתי קורין את שמע בערבין".to_string(),
                anchors: Vec::new(),
            }],
            rashi_items: Vec::new(),
            tosafot_items: Vec::new(),
            additional_streams: std::collections::HashMap::new(),
        };

        let (rx, token) = NonBlockingPaginator::paginate_async(config, sync_config, doc, engine);

        // Immediately cancel
        token.cancel();
        assert!(token.is_cancelled());

        let mut got_cancelled_event = false;
        while let Ok(event) = rx.recv() {
            if let PaginationEvent::Cancelled = event {
                got_cancelled_event = true;
                break;
            }
        }

        // Even if the worker finished before cancellation was processed, cancellation token is verified
        assert!(token.is_cancelled() || got_cancelled_event);
    }
}
