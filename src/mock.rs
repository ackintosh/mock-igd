//! Mock registration and management.

use crate::action::Action;
use crate::matcher::{Matcher, SoapRequest};
use crate::responder::{ResponseBody, Responder};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A registered mock that matches requests and generates responses.
pub(crate) struct Mock {
    /// The action matcher.
    action: Action,
    /// The responder to use when matched.
    responder: Responder,
    /// Priority for matching (higher = checked first).
    priority: u32,
    /// Maximum number of times this mock can be matched (None = unlimited).
    max_times: Option<u32>,
    /// Number of times this mock has been matched.
    match_count: AtomicU32,
}

impl Mock {
    /// Create a new mock with the given action and responder.
    pub fn new(action: impl Into<Action>, responder: impl Into<Responder>) -> Self {
        Mock {
            action: action.into(),
            responder: responder.into(),
            priority: 0,
            max_times: None,
            match_count: AtomicU32::new(0),
        }
    }

    /// Set the priority of this mock (higher = checked first).
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Limit the number of times this mock can be matched.
    pub fn times(mut self, n: u32) -> Self {
        self.max_times = Some(n);
        self
    }

    /// Check if this mock matches the given request.
    pub fn matches(&self, request: &SoapRequest) -> bool {
        // Check if we've exceeded max_times
        if let Some(max) = self.max_times {
            if self.match_count.load(Ordering::SeqCst) >= max {
                return false;
            }
        }
        self.action.matches(request)
    }

    /// Generate a response for the given request and increment match count.
    pub fn respond(&self, request: &SoapRequest) -> ResponseBody {
        self.match_count.fetch_add(1, Ordering::SeqCst);
        self.responder.respond(request)
    }

    /// Get the priority of this mock.
    pub fn priority(&self) -> u32 {
        self.priority
    }
}

impl std::fmt::Debug for Mock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mock")
            .field("action", &self.action)
            .field("responder", &self.responder)
            .field("priority", &self.priority)
            .field("max_times", &self.max_times)
            .field("match_count", &self.match_count.load(Ordering::SeqCst))
            .finish()
    }
}

/// Registry of mocks for matching requests.
#[derive(Default)]
pub(crate) struct MockRegistry {
    mocks: RwLock<Vec<Arc<Mock>>>,
}

impl MockRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        MockRegistry {
            mocks: RwLock::new(Vec::new()),
        }
    }

    /// Register a new mock.
    pub async fn register(&self, mock: Mock) {
        let mut mocks = self.mocks.write().await;
        mocks.push(Arc::new(mock));
        // Sort by priority (highest first)
        mocks.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Find a mock that matches the given request and generate a response.
    pub async fn find_response(&self, request: &SoapRequest) -> Option<ResponseBody> {
        let mocks = self.mocks.read().await;
        for mock in mocks.iter() {
            if mock.matches(request) {
                return Some(mock.respond(request));
            }
        }
        None
    }

    /// Clear all registered mocks.
    pub async fn clear(&self) {
        let mut mocks = self.mocks.write().await;
        mocks.clear();
    }
}
