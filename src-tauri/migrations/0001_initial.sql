-- Initial schema for AgentHub Local.
-- Tables track the indexed view of agent configurations and history.
-- The filesystem remains the source of truth for agent configs.

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    installed INTEGER NOT NULL DEFAULT 0,
    version TEXT,
    health_status TEXT NOT NULL DEFAULT 'unknown',
    last_detected_at TEXT
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS config_scopes (
    id TEXT PRIMARY KEY,
    agent_kind TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    project_id TEXT,
    config_path TEXT NOT NULL,
    writable INTEGER NOT NULL DEFAULT 1,
    UNIQUE (agent_kind, scope_type, project_id, config_path),
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS resources (
    id TEXT PRIMARY KEY,
    resource_type TEXT NOT NULL,
    name TEXT NOT NULL,
    slug TEXT,
    agent_kind TEXT,
    source_path TEXT,
    json_payload TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_resources_type ON resources (resource_type);
CREATE INDEX IF NOT EXISTS idx_resources_agent ON resources (agent_kind);

CREATE TABLE IF NOT EXISTS resource_bindings (
    id TEXT PRIMARY KEY,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    agent_kind TEXT NOT NULL,
    project_id TEXT,
    scope_type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (resource_type, resource_id, agent_kind, project_id, scope_type),
    FOREIGN KEY (resource_id) REFERENCES resources (id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS scan_snapshots (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    agent_kind TEXT,
    summary_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_scan_snapshots_project ON scan_snapshots (project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS doctor_issues (
    id TEXT PRIMARY KEY,
    severity TEXT NOT NULL,
    category TEXT NOT NULL,
    message TEXT NOT NULL,
    target_ref_json TEXT,
    fixable INTEGER NOT NULL DEFAULT 0,
    project_id TEXT,
    agent_kind TEXT,
    resolved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_doctor_issues_open ON doctor_issues (resolved, severity);

CREATE TABLE IF NOT EXISTS change_sets (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    operations_json TEXT NOT NULL,
    patches_json TEXT NOT NULL,
    diff_summary_json TEXT NOT NULL,
    backup_id TEXT,
    project_id TEXT,
    agent_kind TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_change_sets_status ON change_sets (status, created_at DESC);

CREATE TABLE IF NOT EXISTS backups (
    id TEXT PRIMARY KEY,
    change_set_id TEXT NOT NULL,
    manifest_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (change_set_id) REFERENCES change_sets (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS prompt_templates (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    variables_json TEXT NOT NULL DEFAULT '[]',
    tags_json TEXT NOT NULL DEFAULT '[]',
    favorite INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
