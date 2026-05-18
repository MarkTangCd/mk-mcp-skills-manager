import { useCallback, useEffect, useMemo, useState } from 'react';

import DiffPreview from '../components/DiffPreview';
import ErrorMessage from '../components/ErrorMessage';
import McpForm from '../components/McpForm';
import PaginationControls from '../components/PaginationControls';
import { ApiError, api } from '../lib/api';
import { useMcpChangeFlow } from '../hooks/useMcpChangeFlow';
import { paginateItems } from '../lib/pagination';
import type { ResourceRecord } from '../types/domain';

const ALL_AGENTS = 'all';
const ALL_PROJECTS = 'all';
const RESOURCE_PAGE_SIZE = 50;

type AgentKindOrAll = 'claude-code' | 'codex' | 'opencode' | 'pi' | typeof ALL_AGENTS;

export default function McpServersPage() {
  const [resources, setResources] = useState<ResourceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [search, setSearch] = useState('');
  const [agentFilter, setAgentFilter] = useState<AgentKindOrAll>(ALL_AGENTS);
  const [projectFilter, setProjectFilter] = useState(ALL_PROJECTS);
  const [currentPage, setCurrentPage] = useState(1);

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

  const projects = useMemo(() => {
    const map = new Map<string, string>();
    for (const resource of resources) {
      for (const binding of resource.bindings) {
        if (binding.projectId) {
          map.set(binding.projectId, binding.projectName ?? binding.projectId);
        }
      }
    }
    return [...map.entries()].sort((a, b) => a[1].localeCompare(b[1]));
  }, [resources]);

  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    return resources.filter((resource) => {
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
  }, [resources, search, agentFilter, projectFilter]);

  useEffect(() => {
    setCurrentPage(1);
  }, [search, agentFilter, projectFilter]);

  const page = paginateItems(filtered, currentPage, RESOURCE_PAGE_SIZE);

  const flow = useMcpChangeFlow(load);

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
        <button
          type="button"
          className="diff-preview__btn diff-preview__btn--primary"
          onClick={flow.handleAdd}
        >
          Add MCP
        </button>
      </div>

      {error && <ErrorMessage error={error} />}

      {flow.actionError && <ErrorMessage error={flow.actionError} />}

      {loading ? (
        <div className="page__placeholder">Loading…</div>
      ) : filtered.length === 0 ? (
        <div className="page__placeholder">No indexed MCP servers. Rescan a project first.</div>
      ) : (
        <div className="matrix-table__scroll">
          <PaginationControls page={page} onPageChange={setCurrentPage} />
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
              {page.items.map((resource) => (
                <McpRow
                  key={resource.id}
                  resource={resource}
                  onEdit={flow.handleEdit}
                  onToggle={flow.handleToggle}
                  onDelete={flow.handleDelete}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}

      {flow.mode === 'form' && (
        <div className="modal-overlay">
          <div className="modal-content">
            <h2 className="dashboard__section-title">
              {flow.editData ? 'Edit MCP Server' : 'Add MCP Server'}
            </h2>
            <McpForm
              mode={flow.editData ? 'edit' : 'create'}
              initialData={flow.editData?.data}
              initialAgentKind={flow.editData?.agentKind}
              initialScopeType={flow.editData?.scopeType}
              initialProjectId={flow.editData?.projectId}
              onSubmit={flow.handleCreatePlan}
              onCancel={flow.handleCancel}
            />
          </div>
        </div>
      )}

      {flow.mode === 'preview' && flow.plan && (
        <div className="modal-overlay">
          <div className="modal-content">
            {flow.actionError && <ErrorMessage error={flow.actionError} />}
            <DiffPreview
              plan={flow.plan}
              onConfirm={
                flow.plan.status === 'previewed'
                  ? flow.handleConfirmPlan
                  : flow.plan.status === 'confirmed'
                    ? flow.handleApplyPlan
                    : undefined
              }
              onCancel={flow.handleCancel}
            />
          </div>
        </div>
      )}
    </div>
  );
}

function McpRow({
  resource,
  onEdit,
  onToggle,
  onDelete,
}: {
  resource: ResourceRecord;
  onEdit: (resource: ResourceRecord) => void;
  onToggle: (resource: ResourceRecord, enable: boolean) => void;
  onDelete: (resource: ResourceRecord) => void;
}) {
  const payload = resource.payload as { enabled?: boolean } | undefined;
  const isEnabled = payload?.enabled ?? false;

  return (
    <tr>
      <td>
        <div>{resource.name}</div>
        {resource.slug && <div className="resource-list__meta">{resource.slug}</div>}
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
          <button type="button" className="projects__remove" onClick={() => onEdit(resource)}>
            Edit
          </button>
          <button
            type="button"
            className="projects__remove"
            onClick={() => onToggle(resource, !isEnabled)}
          >
            {isEnabled ? 'Disable' : 'Enable'}
          </button>
          <button
            type="button"
            className="projects__remove"
            style={{ color: 'var(--color-danger)', borderColor: 'var(--color-danger)' }}
            onClick={() => onDelete(resource)}
          >
            Delete
          </button>
        </div>
      </td>
    </tr>
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
