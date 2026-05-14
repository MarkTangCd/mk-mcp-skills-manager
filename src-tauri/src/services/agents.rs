use std::sync::Arc;

use crate::adapters::{AdapterRegistry, ScanContext};
use crate::domain::{Agent, AgentKind, HealthStatus};

#[derive(Clone)]
pub struct AgentService {
    registry: Arc<AdapterRegistry>,
}

impl AgentService {
    pub fn new(registry: Arc<AdapterRegistry>) -> Self {
        Self { registry }
    }

    pub fn list(&self) -> Vec<Agent> {
        let ctx = ScanContext::empty();
        let mut agents = self
            .registry
            .all()
            .into_iter()
            .map(|adapter| {
                let kind = adapter.kind();
                match adapter.detect_installation(&ctx) {
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
            })
            .collect::<Vec<_>>();
        agents.sort_by_key(|agent| agent.kind.as_str().to_string());
        agents
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
}
