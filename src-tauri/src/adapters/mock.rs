// MockAdapter: deterministic stand-in used by ScanService tests and the
// Phase 2 UI before real agent adapters land. Behavior is configurable
// via builder methods so tests can simulate failures.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::domain::{AgentKind, McpServer, McpTransport, ScanSummary, ScopeType};

use super::traits::{
    AdapterError, AdapterResult, AgentAdapter, DetectionResult, ScanContext, ScanOutcome,
    ScopeLocation,
};

pub struct MockAdapter {
    kind: AgentKind,
    fail_scan: bool,
    fail_detect: bool,
    installed: bool,
    pub scan_calls: AtomicUsize,
}

impl MockAdapter {
    pub fn new(kind: AgentKind) -> Self {
        Self {
            kind,
            fail_scan: false,
            fail_detect: false,
            installed: true,
            scan_calls: AtomicUsize::new(0),
        }
    }

    pub fn failing_scan(kind: AgentKind) -> Self {
        Self {
            kind,
            fail_scan: true,
            fail_detect: false,
            installed: true,
            scan_calls: AtomicUsize::new(0),
        }
    }

    pub fn uninstalled(kind: AgentKind) -> Self {
        Self {
            kind,
            fail_scan: false,
            fail_detect: false,
            installed: false,
            scan_calls: AtomicUsize::new(0),
        }
    }

    pub fn scan_call_count(&self) -> usize {
        self.scan_calls.load(Ordering::SeqCst)
    }

    fn synthetic_mcp(&self) -> McpServer {
        McpServer {
            id: format!("mock-{}-mcp", self.kind.as_str()),
            name: format!("mock-{}", self.kind.as_str()),
            transport: McpTransport::Stdio,
            command: Some("echo".into()),
            args: vec!["hello".into()],
            url: None,
            env_refs: vec![],
            enabled: true,
        }
    }
}

impl AgentAdapter for MockAdapter {
    fn kind(&self) -> AgentKind {
        self.kind
    }

    fn detect_installation(&self, _ctx: &ScanContext) -> AdapterResult<DetectionResult> {
        if self.fail_detect {
            return Err(AdapterError::Invalid("simulated detect failure".into()));
        }
        Ok(DetectionResult {
            installed: self.installed,
            version: if self.installed {
                Some("0.0.0-mock".into())
            } else {
                None
            },
            notes: vec![],
        })
    }

    fn locate_global_config(&self, ctx: &ScanContext) -> AdapterResult<Option<ScopeLocation>> {
        let base = ctx
            .fixture_root
            .clone()
            .unwrap_or_else(|| PathBuf::from("/tmp/mock-global"));
        Ok(Some(ScopeLocation {
            scope_type: ScopeType::Global,
            config_path: base.join(format!("{}-global.json", self.kind.as_str())),
            writable: false,
        }))
    }

    fn locate_project_config(&self, ctx: &ScanContext) -> AdapterResult<Option<ScopeLocation>> {
        let Some(project) = ctx.project_path.clone() else {
            return Ok(None);
        };
        Ok(Some(ScopeLocation {
            scope_type: ScopeType::Project,
            config_path: project.join(format!(".{}.json", self.kind.as_str())),
            writable: false,
        }))
    }

    fn scan(&self, ctx: &ScanContext) -> AdapterResult<ScanOutcome> {
        self.scan_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_scan {
            return Err(AdapterError::Parse("simulated scan failure".into()));
        }
        let mut scopes = vec![];
        if let Some(g) = self.locate_global_config(ctx)? {
            scopes.push(g);
        }
        if let Some(p) = self.locate_project_config(ctx)? {
            scopes.push(p);
        }
        let mcp_servers = vec![self.synthetic_mcp()];
        let summary = ScanSummary {
            total_resources: mcp_servers.len() as u32,
            mcp_count: mcp_servers.len() as u32,
            skill_count: 0,
            sub_agent_count: 0,
            pi_resource_count: 0,
            errors: vec![],
        };
        Ok(ScanOutcome {
            agent_kind_str: self.kind.as_str().to_string(),
            scopes,
            mcp_servers,
            skills: vec![],
            sub_agents: vec![],
            pi_resources: vec![],
            summary,
            errors: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_scan_returns_one_mcp() {
        let m = MockAdapter::new(AgentKind::Codex);
        let out = m.scan(&ScanContext::empty()).unwrap();
        assert_eq!(out.mcp_servers.len(), 1);
        assert_eq!(out.summary.mcp_count, 1);
        assert_eq!(m.scan_call_count(), 1);
    }

    #[test]
    fn mock_failing_scan_returns_error() {
        let m = MockAdapter::failing_scan(AgentKind::Opencode);
        let err = m.scan(&ScanContext::empty()).unwrap_err();
        assert!(matches!(err, AdapterError::Parse(_)));
    }

    #[test]
    fn detect_reports_install_state() {
        let installed = MockAdapter::new(AgentKind::ClaudeCode);
        let uninstalled = MockAdapter::uninstalled(AgentKind::Pi);
        assert!(
            installed
                .detect_installation(&ScanContext::empty())
                .unwrap()
                .installed
        );
        assert!(
            !uninstalled
                .detect_installation(&ScanContext::empty())
                .unwrap()
                .installed
        );
    }
}
