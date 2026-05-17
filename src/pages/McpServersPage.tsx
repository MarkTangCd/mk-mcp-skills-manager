import { useCallback, useEffect, useState } from 'react';

import DiffPreview from '../components/DiffPreview';
import McpForm from '../components/McpForm';
import { ApiError, api } from '../lib/api';
import type { AgentKind, ChangeIntent, ChangePlan, McpServer, ResourceRecord } from '../types/domain';

const ALL_AGENTS = 'all';
const ALL_PROJECTS = 'all';

type AgentKindOrAll = 'claude-code' | 'codex' | 'opencode' | 'pi' | typeof ALL_AGENTS;

type Mode = 'list' | 'form' | 'preview';

export default function McpServersPage() {
  const [resources, setResources] = useState<ResourceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [search, setSearch] = useState('');
  const [agentFilter, setAgentFilter] = useState<AgentKindOrAll>(ALL_AGENTS);
  const [projectFilter, setProjectFilter] = useState(ALL_PROJECTS);

  const [mode, setMode] = useState<Mode>('list');
  const [editData, setEditData] = useState<
    | {
        data: McpServer;
        agentKind: AgentKind;
        scopeType: 'global' | 'project';
        projectId: string | null;
      }
    | undefined
  >(undefined);
  const [plan, setPlan] = useState<ChangePlan | null>(null);
  const [planProjectId, setPlanProjectId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<ApiError | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.resources.list('mcp');
      setResources(list);
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const projects = (() => {
    const map = new Map<string, string>();
    for (const resource of resources) {
      for (const binding of resource.bindings) {
        if (binding.projectId) {
          map.set(binding.projectId, binding.projectName ?? binding.projectId);
        }
      }
    }
    return [...map.entries()].sort((a, b) => a[1].localeCompare(b[1]));
  })();

  const filtered = resources.filter((resource) => {
    const query = search.trim().toLowerCase();
    const text = [
      resource.name,
      resource.slug,
      resource.sourcePath,
      ...resource.bindings.map((binding) => binding.configPath),
    ]
      .filter(Boolean)
      .join(' ')
      .toLowerCase();
    const matchesSearch = !query || text.includes(query);
    const matchesAgent =
      agentFilter === ALL_AGENTS ||
      resource.bindings.some((binding) => binding.agentKind === agentFilter);
    const matchesProject =
      projectFilter === ALL_PROJECTS ||
      resource.bindings.some((binding) => binding.projectId === projectFilter);
    return matchesSearch && matchesAgent && matchesProject;
  });

  async function handleCreatePlan(intent: ChangeIntent) {
    setActionError(null);
    try {
      const created = await api.changes.createChangePlan(intent);
      const previewed = await api.changes.transition(created.id, 'previewed');
      setPlan(previewed);
      setPlanProjectId(intent.projectId ?? null);
      setMode('preview');
    } catch (err) {
      setActionError(err as ApiError);
    }
  }

  async function handleConfirmPlan() {
    if (!plan) return;
    setActionError(null);
    try {
      const confirmed = await api.changes.transition(plan.id, 'confirmed');
      setPlan(confirmed);
    } catch (err) {
      setActionError(err as ApiError);
    }
  }

  async function handleApplyPlan() {
    if (!plan) return;
    setActionError(null);
    try {
      await api.changes.applyPlan(plan.id);
      setPlan(null);
      setMode('list');
      // Trigger rescan of affected project so the resource list stays in sync.
      if (planProjectId) {
        try {
          await api.projects.rescan(planProjectId);
        } catch {
          // Best-effort rescan; don't block the flow on failure.
        }
      }
      setPlanProjectId(null);
      await load();
    } catch (err) {
      setActionError(err as ApiError);
    }
  }

  function handleAdd() {
    setEditData(undefined);
    setMode('form');
    setActionError(null);
  }

  function handleEdit(resource: ResourceRecord) {
    const payload = resource.payload as McpServer | undefined;
    if (!payload) return;
    const binding = resource.bindings[0];
    setEditData({
      data: payload,
      agentKind: binding?.agentKind ?? 'claude-code',
      scopeType: binding?.scopeType ?? 'global',
      projectId: binding?.projectId ?? null,
    });
    setMode('form');
    setActionError(null);
  }

  async function handleToggle(resource: ResourceRecord, enable: boolean) {
    const payload = resource.payload as McpServer | undefined;
    if (!payload) return;
    const intent: ChangeIntent = {
      id: crypto.randomUUID(),
      changeType: enable ? 'enableMcp' : 'disableMcp',
      agentKind: resource.bindings[0]?.agentKind ?? 'claude-code',
      projectId: resource.bindings[0]?.projectId ?? null,
      scopeType: resource.bindings[0]?.scopeType ?? 'global',
      resourceId: resource.id,
      payload: { id: resource.id, enabled: enable },
      createdAt: new Date().toISOString(),
    };
    await handleCreatePlan(intent);
  }

  async function handleDelete(resource: ResourceRecord) {
    const intent: ChangeIntent = {
      id: crypto.randomUUID(),
      changeType: 'deleteMcp',
      agentKind: resource.bindings[0]?.agentKind ?? 'claude-code',
      projectId: resource.bindings[0]?.projectId ?? null,
      scopeType: resource.bindings[0]?.scopeType ?? 'global',
      resourceId: resource.id,
      payload: { id: resource.id },
      createdAt: new Date().toISOString(),
    };
    await handleCreatePlan(intent);
  }

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">MCP Servers</h1>
        <p className="page__subtitle">Manage configured MCP servers.</p>
      </header>

      <div className="resource-toolbar">
        <label className="projects__field">
          <span>Search</span>
          <input
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Name, slug, source path"
          />
        </label>
        <label className="projects__field">
          <span>Agent</span>
          <select
            value={agentFilter}
            onChange={(e) => setAgentFilter(e.target.value as AgentKindOrAll)}
          >
            <option value={ALL_AGENTS}>All agents</option>
            <option value="claude-code">Claude Code</option>
            <option value="codex">Codex</option>
            <option value="opencode">opencode</option>
            <option value="pi">Pi</option>
          </select>
        </label>
        <label className="projects__field">
          <span>Project</span>
          <select value={projectFilter} onChange={(e) => setProjectFilter(e.target.value)}>
            <option value={ALL_PROJECTS}>All projects</option>
            {projects.map(([id, name]) => (
              <option key={id} value={id}>
                {name}
              </option>
            ))}
          </select>
        </label>
        <button type="button" className="diff-preview__btn diff-preview__btn--primary" onClick={handleAdd}>
          Add MCP
        </button>
      </div>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      {actionError && (
        <div className="dashboard__error" role="alert">
          [{actionError.code}] {actionError.message}
        </div>
      )}

      {loading ? (
        <div className="page__placeholder">Loading…</div>
      ) : filtered.length === 0 ? (
        <div className="page__placeholder">No indexed MCP servers. Rescan a project first.</div>
      ) : (
        <div className="matrix-table__scroll">
          <table className="projects__table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Status</th>
                <th>Agents</th>
                <th>Projects</th>
                <th>Source</th>
                <th style={{ textAlign: 'right' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((resource) => {
                const payload = resource.payload as McpServer | undefined;
                const isEnabled = payload?.enabled ?? false;
                return (
                  <tr key={resource.id}>
                    <td>
                      <div>{resource.name}</div>
                      {resource.slug && (
                        <div className="resource-list__meta">{resource.slug}</div>
                      )}
                    </td>
                    <td>
                      <span className={`resource-status resource-status--${resource.status}`}>
                        {resource.status}
                      </span>
                    </td>
                    <td>{formatAgents(resource)}</td>
                    <td>{formatProjects(resource)}</td>
                    <td className="projects__path">
                      {resource.sourcePath ?? firstConfigPath(resource) ?? '—'}
                    </td>
                    <td style={{ textAlign: 'right' }}>
                      <div style={{ display: 'flex', gap: 'var(--space-2)', justifyContent: 'flex-end' }}>
                        <button
                          type="button"
                          className="projects__remove"
                          onClick={() => handleEdit(resource)}
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          className="projects__remove"
                          onClick={() => handleToggle(resource, !isEnabled)}
                        >
                          {isEnabled ? 'Disable' : 'Enable'}
                        </button>
                        <button
                          type="button"
                          className="projects__remove"
                          style={{ color: 'var(--color-danger)', borderColor: 'var(--color-danger)' }}
                          onClick={() => handleDelete(resource)}
                        >
                          Delete
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {mode === 'form' && (
        <div className="modal-overlay">
          <div className="modal-content">
            <h2 className="dashboard__section-title">
              {editData ? 'Edit MCP Server' : 'Add MCP Server'}
            </h2>
            <McpForm
              mode={editData ? 'edit' : 'create'}
              initialData={editData?.data}
              initialAgentKind={editData?.agentKind}
              initialScopeType={editData?.scopeType}
              initialProjectId={editData?.projectId}
              onSubmit={handleCreatePlan}
              onCancel={() => setMode('list')}
            />
          </div>
        </div>
      )}

      {mode === 'preview' && plan && (
        <div className="modal-overlay">
          <div className="modal-content">
            <DiffPreview
              plan={plan}
              onConfirm={
                plan.status === 'previewed'
                  ? handleConfirmPlan
                  : plan.status === 'confirmed'
                    ? handleApplyPlan
                    : undefined
              }
              onCancel={() => {
                setPlan(null);
                setMode('list');
              }}
            />
          </div>
        </div>
      )}
    </div>
  );
}

function formatAgents(resource: ResourceRecord) {
  const agents = [...new Set(resource.bindings.map((binding) => binding.agentKind))];
  return agents.length === 0 ? '—' : agents.join(', ');
}

function formatProjects(resource: ResourceRecord) {
  const projects = [
    ...new Set(
      resource.bindings.map((binding) =>
        binding.projectName ?? (binding.projectId ? binding.projectId : 'global'),
      ),
    ),
  ];
  return projects.length === 0 ? '—' : projects.join(', ');
}

function firstConfigPath(resource: ResourceRecord) {
  return resource.bindings.find((binding) => binding.configPath)?.configPath ?? null;
}
