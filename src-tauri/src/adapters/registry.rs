// AdapterRegistry: holds a set of `AgentAdapter` trait objects keyed by
// `AgentKind`. ScanService iterates the registry to run scans across
// all known agents.

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::AgentKind;

use super::traits::AgentAdapter;

#[derive(Clone, Default)]
pub struct AdapterRegistry {
    adapters: HashMap<AgentKind, Arc<dyn AgentAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, adapter: Arc<dyn AgentAdapter>) {
        self.adapters.insert(adapter.kind(), adapter);
    }

    pub fn get(&self, kind: AgentKind) -> Option<Arc<dyn AgentAdapter>> {
        self.adapters.get(&kind).cloned()
    }

    pub fn all(&self) -> Vec<Arc<dyn AgentAdapter>> {
        self.adapters.values().cloned().collect()
    }

    pub fn kinds(&self) -> Vec<AgentKind> {
        self.adapters.keys().copied().collect()
    }

    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::MockAdapter;

    #[test]
    fn register_and_lookup_by_kind() {
        let mut reg = AdapterRegistry::new();
        reg.register(Arc::new(MockAdapter::new(AgentKind::Codex)));
        reg.register(Arc::new(MockAdapter::new(AgentKind::Opencode)));
        assert_eq!(reg.len(), 2);
        assert!(reg.get(AgentKind::Codex).is_some());
        assert!(reg.get(AgentKind::ClaudeCode).is_none());
    }

    #[test]
    fn re_register_replaces_previous() {
        let mut reg = AdapterRegistry::new();
        reg.register(Arc::new(MockAdapter::new(AgentKind::Codex)));
        reg.register(Arc::new(MockAdapter::new(AgentKind::Codex)));
        assert_eq!(reg.len(), 1);
    }
}
