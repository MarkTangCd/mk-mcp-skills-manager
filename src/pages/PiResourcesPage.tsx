import { useCallback, useEffect, useMemo, useState } from 'react';
import type React from 'react';

import DiffPreview from '../components/DiffPreview';
import { ApiError, api } from '../lib/api';
import type { ChangeIntent, ChangePlan, PiResource, PiResourceKind, ResourceRecord } from '../types/domain';

const RESOURCE_KINDS: PiResourceKind[] = [
  'setting',
  'skill',
  'prompt-template',
  'extension',
  'package',
  'theme',
];

const PATH_KEYS = ['skills', 'prompt_templates', 'extensions', 'packages', 'themes'];
const SECURITY_KEYS = ['allow_skill_commands', 'allow_extension_commands'];

export default function PiResourcesPage() {
  const [resources, setResources] = useState<ResourceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [selectedKind, setSelectedKind] = useState<PiResourceKind | 'all'>('all');
  const [search, setSearch] = useState('');
  const [pathKey, setPathKey] = useState(PATH_KEYS[0]);
  const [pathValue, setPathValue] = useState('');
  const [securityKey, setSecurityKey] = useState(SECURITY_KEYS[0]);
  const [securityEnabled, setSecurityEnabled] = useState(false);
  const [plan, setPlan] = useState<ChangePlan | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setResources(await api.resources.list('pi-resource'));
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

  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    return resources.filter((resource) => {
      const pi = asPiResource(resource.payload);
      const kind = pi?.resourceType ?? 'setting';
      const text = [
        resource.name,
        resource.slug,
        resource.sourcePath,
        pi?.source,
        pi?.path,
        ...resource.bindings.map((binding) => binding.configPath),
      ]
        .filter(Boolean)
        .join(' ')
        .toLowerCase();
      return (selectedKind === 'all' || kind === selectedKind) && (!query || text.includes(query));
    });
  }, [resources, search, selectedKind]);

  const counts = useMemo(() => {
    const map = new Map<PiResourceKind, number>();
    for (const resource of resources) {
      const kind = asPiResource(resource.payload)?.resourceType ?? 'setting';
      map.set(kind, (map.get(kind) ?? 0) + 1);
    }
    return map;
  }, [resources]);

  const hasProjectOverrides = resources.some((resource) => {
    const scopes = new Set(resource.bindings.map((binding) => binding.scopeType));
    return scopes.has('global') && scopes.has('project');
  });

  async function createPlan(changeType: string, payload: Record<string, unknown>) {
    const intent: ChangeIntent = {
      id: crypto.randomUUID(),
      changeType,
      agentKind: 'pi',
      projectId: null,
      scopeType: 'global',
      resourceId: null,
      payload,
      createdAt: new Date().toISOString(),
    };
    const draft = await api.changes.createChangePlan(intent);
    const previewed = await api.changes.transition(draft.id, 'previewed');
    setPlan(previewed);
  }

  async function handleApplyPlan() {
    if (!plan) return;
    try {
      if (plan.status === 'previewed') {
        setPlan(await api.changes.transition(plan.id, 'confirmed'));
        return;
      }
      if (plan.status === 'confirmed') {
        const applied = await api.changes.applyPlan(plan.id);
        setPlan(applied);
        await load();
      }
    } catch (err) {
      setError(err as ApiError);
    }
  }

  async function handlePathSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      await createPlan('updatePiResourcePath', { key: pathKey, path: pathValue });
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    }
  }

  async function handleSecuritySubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      await createPlan('updatePiSecuritySetting', {
        key: securityKey,
        enabled: securityEnabled,
      });
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    }
  }

  return (
    <div className="page page--wide">
      <header className="page__header">
        <h1 className="page__title">Pi Resources</h1>
        <p className="page__subtitle">
          Pi settings, skills, prompt templates, extensions, packages, and themes.
        </p>
      </header>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      {hasProjectOverrides && (
        <div className="pi-resources__warning" role="status">
          Project settings override global Pi settings for at least one indexed resource.
        </div>
      )}

      <section className="pi-resources__overview" aria-label="Pi overview">
        <button
          type="button"
          className={selectedKind === 'all' ? 'pi-resources__pill pi-resources__pill--active' : 'pi-resources__pill'}
          onClick={() => setSelectedKind('all')}
        >
          <span>All</span>
          <strong>{resources.length}</strong>
        </button>
        {RESOURCE_KINDS.map((kind) => (
          <button
            key={kind}
            type="button"
            className={selectedKind === kind ? 'pi-resources__pill pi-resources__pill--active' : 'pi-resources__pill'}
            onClick={() => setSelectedKind(kind)}
          >
            <span>{formatKind(kind)}</span>
            <strong>{counts.get(kind) ?? 0}</strong>
          </button>
        ))}
      </section>

      <div className="pi-resources__layout">
        <section className="pi-resources__main">
          <div className="resource-toolbar resource-toolbar--compact">
            <label className="projects__field">
              <span>Search</span>
              <input
                type="search"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Source, path, config"
              />
            </label>
          </div>

          {loading ? (
            <div className="page__placeholder">Loading Pi resources…</div>
          ) : filtered.length === 0 ? (
            <div className="page__placeholder">No indexed Pi resources. Rescan a project first.</div>
          ) : (
            <div className="matrix-table__scroll">
              <table className="projects__table">
                <thead>
                  <tr>
                    <th>Kind</th>
                    <th>Source</th>
                    <th>Status</th>
                    <th>Path</th>
                    <th>Risk</th>
                    <th>Scope</th>
                  </tr>
                </thead>
                <tbody>
                  {filtered.map((resource) => {
                    const pi = asPiResource(resource.payload);
                    const risk = riskFor(resource, pi);
                    return (
                      <tr key={resource.id}>
                        <td>{formatKind(pi?.resourceType ?? 'setting')}</td>
                        <td>
                          <div>{pi?.source ?? resource.name}</div>
                          <div className="resource-list__meta">{resource.id}</div>
                        </td>
                        <td>
                          <span className={`resource-status resource-status--${resource.status}`}>
                            {pi?.enabled === false ? 'disabled' : resource.status}
                          </span>
                        </td>
                        <td className="projects__path">{pi?.path ?? resource.sourcePath ?? '—'}</td>
                        <td className={risk.level === 'warning' ? 'pi-resources__risk' : 'resource-list__meta'}>
                          {risk.label}
                        </td>
                        <td>{formatScopes(resource)}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </section>

        <aside className="pi-resources__side">
          <section className="pi-resources__panel">
            <h2 className="dashboard__section-title">Pi Settings</h2>
            <form className="pi-resources__form" onSubmit={handlePathSubmit}>
              <label className="projects__field">
                <span>Resource path</span>
                <select value={pathKey} onChange={(event) => setPathKey(event.target.value)}>
                  {PATH_KEYS.map((key) => (
                    <option key={key} value={key}>
                      {key}
                    </option>
                  ))}
                </select>
              </label>
              <label className="projects__field">
                <span>Path</span>
                <input
                  value={pathValue}
                  onChange={(event) => setPathValue(event.target.value)}
                  placeholder="/Users/me/Library/Application Support/AgentHubLocal/library/skills"
                />
              </label>
              <button type="submit" disabled={!pathValue.trim()}>
                Preview path change
              </button>
            </form>
            <form className="pi-resources__form" onSubmit={handleSecuritySubmit}>
              <label className="projects__field">
                <span>Security</span>
                <select value={securityKey} onChange={(event) => setSecurityKey(event.target.value)}>
                  {SECURITY_KEYS.map((key) => (
                    <option key={key} value={key}>
                      {key}
                    </option>
                  ))}
                </select>
              </label>
              <label className="pi-resources__toggle">
                <input
                  type="checkbox"
                  checked={securityEnabled}
                  onChange={(event) => setSecurityEnabled(event.target.checked)}
                />
                <span>Enabled</span>
              </label>
              <button type="submit">Preview security change</button>
            </form>
          </section>

          {plan && (
            <DiffPreview
              plan={plan}
              onConfirm={plan.status === 'previewed' || plan.status === 'confirmed' ? handleApplyPlan : undefined}
              onCancel={() => setPlan(null)}
            />
          )}
        </aside>
      </div>
    </div>
  );
}

function asPiResource(payload: unknown): PiResource | null {
  if (!payload || typeof payload !== 'object') return null;
  const candidate = payload as Partial<PiResource>;
  return typeof candidate.resourceType === 'string' ? (candidate as PiResource) : null;
}

function formatKind(kind: PiResourceKind) {
  return kind.replace('-', ' ');
}

function formatScopes(resource: ResourceRecord) {
  const scopes = [...new Set(resource.bindings.map((binding) => binding.scopeType))];
  return scopes.length === 0 ? '—' : scopes.join(', ');
}

function riskFor(resource: ResourceRecord, pi: PiResource | null) {
  if (pi?.resourceType === 'extension') {
    return pi.trusted
      ? { level: 'info', label: 'Extension indexed only' }
      : { level: 'warning', label: 'Untrusted extension' };
  }
  if (resource.status === 'missing') {
    return { level: 'warning', label: 'Missing path' };
  }
  if (pi?.path === null && pi.resourceType !== 'prompt-template') {
    return { level: 'warning', label: 'No path recorded' };
  }
  return { level: 'info', label: '—' };
}
