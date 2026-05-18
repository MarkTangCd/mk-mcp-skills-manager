import { useEffect, useMemo, useState } from 'react';

import ErrorMessage from './ErrorMessage';
import PaginationControls from './PaginationControls';
import { ApiError, api } from '../lib/api';
import { paginateItems } from '../lib/pagination';
import type { AgentKind, ResourceRecord, ResourceType } from '../types/domain';

interface ResourceListPageProps {
  title: string;
  subtitle: string;
  resourceType: ResourceType;
}

const ALL_AGENTS = 'all';
const ALL_PROJECTS = 'all';
const RESOURCE_PAGE_SIZE = 50;

export default function ResourceListPage({
  title,
  subtitle,
  resourceType,
}: ResourceListPageProps) {
  const [resources, setResources] = useState<ResourceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [search, setSearch] = useState('');
  const [agentFilter, setAgentFilter] = useState<AgentKind | typeof ALL_AGENTS>(ALL_AGENTS);
  const [projectFilter, setProjectFilter] = useState(ALL_PROJECTS);
  const [currentPage, setCurrentPage] = useState(1);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    api.resources
      .list(resourceType)
      .then((list) => {
        if (cancelled) return;
        setResources(list);
        setError(null);
      })
      .catch((err: ApiError) => {
        if (cancelled) return;
        setError(err);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [resourceType]);

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

  useEffect(() => {
    setCurrentPage(1);
  }, [search, agentFilter, projectFilter, resourceType]);

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

  const page = paginateItems(filtered, currentPage, RESOURCE_PAGE_SIZE);

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">{title}</h1>
        <p className="page__subtitle">{subtitle}</p>
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
            onChange={(e) => setAgentFilter(e.target.value as AgentKind | typeof ALL_AGENTS)}
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
      </div>

      {error && <ErrorMessage error={error} />}

      {loading ? (
        <div className="page__placeholder">Loading…</div>
      ) : filtered.length === 0 ? (
        <div className="page__placeholder">No indexed resources. Rescan a project first.</div>
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
              </tr>
            </thead>
            <tbody>
              {page.items.map((resource) => (
                <tr key={resource.id}>
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
                </tr>
              ))}
            </tbody>
          </table>
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
