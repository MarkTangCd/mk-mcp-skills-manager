-- Add binding-level metadata used by Matrix source drilldowns.
-- Resource rows remain normalized definitions; binding rows describe
-- where and how each resource appears for a project/agent/scope.

ALTER TABLE resource_bindings ADD COLUMN config_path TEXT;
ALTER TABLE resource_bindings ADD COLUMN status TEXT NOT NULL DEFAULT 'active';

CREATE INDEX IF NOT EXISTS idx_resource_bindings_project
ON resource_bindings (project_id, agent_kind, resource_type, status);
