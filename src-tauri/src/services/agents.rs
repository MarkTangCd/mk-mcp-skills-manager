use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::adapters::{AdapterRegistry, ScanContext};
use crate::domain::{Agent, AgentKind, HealthStatus};

const CACHE_TTL: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct AgentService {
    registry: Arc<AdapterRegistry>,
    cache: Arc<Mutex<Option<CachedAgents>>>,
}

struct CachedAgents {
    fetched_at: Instant,
    agents: Vec<Agent>,
}

impl AgentService {
    pub fn new(registry: Arc<AdapterRegistry>) -> Self {
        Self {
            registry,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn list(&self) -> Vec<Agent> {
        if let Some(cached) = self.cache.lock().as_ref() {
            if cached.fetched_at.elapsed() < CACHE_TTL {
                return cached.agents.clone();
            }
        }

        let agents = self.detect_all();
        *self.cache.lock() = Some(CachedAgents {
            fetched_at: Instant::now(),
            agents: agents.clone(),
        });
        agents
    }

    /// Force-refresh the cached agent list (e.g. after install / uninstall).
    pub fn invalidate_cache(&self) {
        *self.cache.lock() = None;
    }

    fn detect_all(&self) -> Vec<Agent> {
        let adapters = self.registry.all();
        let ctx = ScanContext::empty();

        let mut agents: Vec<Agent> = std::thread::scope(|scope| {
            let handles: Vec<_> = adapters
                .iter()
                .map(|adapter| {
                    let adapter = adapter.clone();
                    let ctx = &ctx;
                    scope.spawn(move || detect_one(adapter.as_ref(), ctx))
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("agent detection thread panicked"))
                .collect()
        });

        agents.sort_by_key(|agent| agent.kind.as_str().to_string());
        agents
    }
}

fn detect_one(adapter: &dyn crate::adapters::AgentAdapter, ctx: &ScanContext) -> Agent {
    let kind = adapter.kind();
    match adapter.detect_installation(ctx) {
        Ok(detection) => Agent {
            id: kind.as_str().to_string(),
            kind,
            display_name: display_name(kind).into(),
            installed: detection.installed,
            version: detection.version,
            health_status: if detection.installed {
                HealthStatus::Ok
            } else {
                HealthStatus::Unknown
            },
        },
        Err(_) => Agent {
            id: kind.as_str().to_string(),
            kind,
            display_name: display_name(kind).into(),
            installed: false,
            version: None,
            health_status: HealthStatus::Warning,
        },
    }
}

fn display_name(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::ClaudeCode => "Claude Code",
        AgentKind::Codex => "Codex",
        AgentKind::Opencode => "opencode",
        AgentKind::Pi => "Pi",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::MockAdapter;

    #[test]
    fn detection_errors_do_not_abort_agent_list() {
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(MockAdapter::new(AgentKind::Codex)));
        registry.register(Arc::new(MockAdapter::uninstalled(AgentKind::Pi)));
        let service = AgentService::new(Arc::new(registry));
        let agents = service.list();
        assert_eq!(agents.len(), 2);
        assert!(agents.iter().any(|agent| agent.kind == AgentKind::Codex));
        assert!(agents
            .iter()
            .any(|agent| agent.kind == AgentKind::Pi && !agent.installed));
    }

    #[test]
    fn second_call_within_ttl_uses_cache() {
        let mut registry = AdapterRegistry::new();
        registry.register(Arc::new(MockAdapter::new(AgentKind::Codex)));
        let service = AgentService::new(Arc::new(registry));

        let first = service.list();
        let second = service.list();
        assert_eq!(first.len(), second.len());

        service.invalidate_cache();
        let third = service.list();
        assert_eq!(first.len(), third.len());
    }
}
