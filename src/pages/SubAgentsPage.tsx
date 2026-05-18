import { useEffect, useMemo, useState } from 'react';

import DiffPreview from '../components/DiffPreview';
import { ApiError, api } from '../lib/api';
import type {
  AgentKind,
  ChangeIntent,
  ChangePlan,
  LibraryEntry,
  LibraryEntryDetail,
  LibraryMetadata,
  Project,
  ResourceRecord,
  ScopeType,
} from '../types/domain';

// ------------------------------------------------------------------
// SubAgentsPage: Sub-agent Library management UI
// ------------------------------------------------------------------

type ModalMode = 'detail' | 'create' | null;

const AGENT_KIND_LABELS: Record<AgentKind, string> = {
  'claude-code': 'Claude Code',
  codex: 'Codex',
  opencode: 'opencode',
  pi: 'Pi',
};

const SUB_AGENT_AGENT_KINDS: AgentKind[] = ['claude-code', 'codex'];

export default function SubAgentsPage() {
  const [entries, setEntries] = useState<LibraryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [search, setSearch] = useState('');
  const [tagFilter, setTagFilter] = useState('');
  const [modalMode, setModalMode] = useState<ModalMode>(null);
  const [selectedSlug, setSelectedSlug] = useState<string | null>(null);

  const load = () => {
    setLoading(true);
    setError(null);
    api.subAgents
      .list(search || undefined, tagFilter ? [tagFilter] : undefined)
      .then((list) => {
        setEntries(list);
      })
      .catch((err: ApiError) => {
        setError(err);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, tagFilter]);

  const allTags = useMemo(() => {
    const set = new Set<string>();
    for (const entry of entries) {
      for (const tag of entry.metadata.tags) {
        set.add(tag);
      }
    }
    return [...set].sort();
  }, [entries]);

  const openDetail = (slug: string) => {
    setSelectedSlug(slug);
    setModalMode('detail');
  };

  const closeModal = () => {
    setModalMode(null);
    setSelectedSlug(null);
  };

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">Sub-agent Library</h1>
        <p className="page__subtitle">Create and manage reusable sub-agents.</p>
      </header>

      <div className="resource-toolbar">
        <label className="projects__field">
          <span>Search</span>
          <input
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Slug, title, role, tags"
          />
        </label>
        <label className="projects__field">
          <span>Tag</span>
          <select value={tagFilter} onChange={(e) => setTagFilter(e.target.value)}>
            <option value="">All tags</option>
            {allTags.map((tag) => (
              <option key={tag} value={tag}>
                {tag}
              </option>
            ))}
          </select>
        </label>
        <div
          className="projects__field"
          style={{ display: 'flex', gap: '8px', alignItems: 'flex-end' }}
        >
          <button onClick={() => setModalMode('create')}>Create Sub-agent</button>
        </div>
      </div>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      {loading ? (
        <div className="page__placeholder">Loading…</div>
      ) : entries.length === 0 ? (
        <div className="page__placeholder">
          No sub-agents found. Create a new sub-agent to get started.
        </div>
      ) : (
        <div className="dashboard__grid">
          {entries.map((entry) => (
            <SubAgentCard
              key={entry.slug}
              entry={entry}
              onClick={() => openDetail(entry.slug)}
            />
          ))}
        </div>
      )}

      {modalMode === 'detail' && selectedSlug && (
        <DetailModal
          slug={selectedSlug}
          onClose={closeModal}
          onDelete={() => {
            closeModal();
            load();
          }}
          onUpdated={() => {
            load();
          }}
        />
      )}
      {modalMode === 'create' && (
        <CreateModal
          onClose={closeModal}
          onCreated={() => {
            closeModal();
            load();
          }}
        />
      )}
    </div>
  );
}

// ------------------------------------------------------------------
// SubAgentCard
// ------------------------------------------------------------------

function SubAgentCard({
  entry,
  onClick,
}: {
  entry: LibraryEntry;
  onClick: () => void;
}) {
  const { metadata } = entry;
  const role = metadata.role ?? '';
  const roleSummary = role.length > 80 ? role.slice(0, 80) + '…' : role;

  return (
    <button
      className="dashboard__card"
      onClick={onClick}
      style={{ textAlign: 'left', cursor: 'pointer', width: '100%' }}
    >
      <div style={{ fontWeight: 600, marginBottom: '4px' }}>{metadata.title}</div>
      <div className="resource-list__meta" style={{ marginBottom: '8px' }}>
        {entry.slug}
      </div>
      {roleSummary && (
        <div
          style={{
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            marginBottom: '8px',
          }}
        >
          {roleSummary}
        </div>
      )}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px', marginBottom: '8px' }}>
        {metadata.agentKinds.map((kind) => (
          <span
            key={kind}
            style={{
              fontSize: '11px',
              padding: '2px 6px',
              borderRadius: '4px',
              background: 'var(--color-bg-elevated)',
              color: 'var(--color-text-muted)',
            }}
          >
            {AGENT_KIND_LABELS[kind]}
          </span>
        ))}
      </div>
      {metadata.tags.length > 0 && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
          {metadata.tags.map((tag) => (
            <span
              key={tag}
              style={{
                fontSize: '11px',
                padding: '2px 6px',
                borderRadius: '4px',
                border: '1px solid var(--color-border)',
                color: 'var(--color-text-muted)',
              }}
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </button>
  );
}

// ------------------------------------------------------------------
// DetailModal
// ------------------------------------------------------------------

function DetailModal({
  slug,
  onClose,
  onDelete,
  onUpdated,
}: {
  slug: string;
  onClose: () => void;
  onDelete: () => void;
  onUpdated: () => void;
}) {
  const [detail, setDetail] = useState<LibraryEntryDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [isEditing, setIsEditing] = useState(false);

  // Enable-to-agent flow state
  const [showEnableForm, setShowEnableForm] = useState(false);
  const [agentKind, setAgentKind] = useState<AgentKind>('claude-code');
  const [scopeType, setScopeType] = useState<ScopeType>('global');
  const [projectId, setProjectId] = useState<string | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [plan, setPlan] = useState<ChangePlan | null>(null);
  const [planActionError, setPlanActionError] = useState<ApiError | null>(null);
  const [planLoading, setPlanLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    api.subAgents
      .get(slug)
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch((err: ApiError) => {
        if (!cancelled) setError(err);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [slug]);

  useEffect(() => {
    if (showEnableForm && scopeType === 'project' && projects.length === 0) {
      api.projects.list().then(setProjects).catch(() => {});
    }
  }, [showEnableForm, scopeType, projects.length]);

  const handleDelete = () => {
    if (!confirm(`Delete sub-agent "${slug}"? This cannot be undone.`)) return;
    api.subAgents
      .delete(slug)
      .then(() => onDelete())
      .catch((err: ApiError) => setError(err));
  };

  const handleSaved = () => {
    setIsEditing(false);
    api.subAgents
      .get(slug)
      .then((d) => setDetail(d))
      .catch(() => {});
    onUpdated();
  };

  const buildIntent = (changeType: 'enableSubAgent' | 'disableSubAgent'): ChangeIntent => {
    const payload: Record<string, unknown> = {
      slug,
      role: detail?.metadata.role ?? '',
      description: detail?.metadata.description ?? null,
      tools: detail?.metadata.boundMcpIds ?? [],
      skills: detail?.metadata.boundSkillIds ?? [],
    };
    return {
      id: crypto.randomUUID(),
      changeType,
      agentKind,
      projectId,
      scopeType,
      resourceId: `library:sub-agent:${slug}`,
      payload,
      createdAt: new Date().toISOString(),
    };
  };

  const handleCreatePlan = async (changeType: 'enableSubAgent' | 'disableSubAgent') => {
    if (agentKind === 'pi' || agentKind === 'opencode') {
      setPlanActionError(new ApiError({
        code: 'unsupported',
        message: `${agentKind} sub-agent sync is not supported`,
        recoverable: true,
      }));
      return;
    }
    setPlanLoading(true);
    setPlanActionError(null);
    try {
      const created = await (changeType === 'enableSubAgent'
        ? api.subAgents.enable(buildIntent(changeType))
        : api.subAgents.disable(buildIntent(changeType)));
      const previewed = await api.changes.transition(created.id, 'previewed');
      setPlan(previewed);
    } catch (err) {
      setPlanActionError(err as ApiError);
    } finally {
      setPlanLoading(false);
    }
  };

  const handleConfirmPlan = async () => {
    if (!plan) return;
    setPlanLoading(true);
    setPlanActionError(null);
    try {
      const confirmed = await api.changes.transition(plan.id, 'confirmed');
      setPlan(confirmed);
    } catch (err) {
      setPlanActionError(err as ApiError);
    } finally {
      setPlanLoading(false);
    }
  };

  const handleApplyPlan = async () => {
    if (!plan) return;
    setPlanLoading(true);
    setPlanActionError(null);
    try {
      await api.changes.applyPlan(plan.id, projectId);
      setPlan(null);
      setShowEnableForm(false);
    } catch (err) {
      setPlanActionError(err as ApiError);
    } finally {
      setPlanLoading(false);
    }
  };

  const handlePlanAction = () => {
    if (!plan) return;
    if (plan.status === 'previewed') {
      handleConfirmPlan();
    } else if (plan.status === 'confirmed') {
      handleApplyPlan();
    }
  };

  const isUnsupportedAgent = agentKind === 'pi' || agentKind === 'opencode';

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginBottom: '16px',
          }}
        >
          <h2 style={{ margin: 0, fontSize: '16px' }}>
            {isEditing ? 'Edit Sub-agent' : 'Sub-agent Detail'}
          </h2>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--color-text-muted)',
              cursor: 'pointer',
            }}
          >
            Close
          </button>
        </div>

        {error && (
          <div
            className="dashboard__error"
            role="alert"
            style={{ marginBottom: '12px' }}
          >
            [{error.code}] {error.message}
          </div>
        )}

        {loading ? (
          <div className="page__placeholder">Loading…</div>
        ) : !detail ? (
          <div className="page__placeholder">Sub-agent not found.</div>
        ) : plan ? (
          <div>
            {planActionError && (
              <div className="dashboard__error" role="alert" style={{ marginBottom: '12px' }}>
                [{planActionError.code}] {planActionError.message}
              </div>
            )}
            <DiffPreview
              plan={plan}
              onConfirm={handlePlanAction}
              onCancel={() => { setPlan(null); setShowEnableForm(false); }}
            />
          </div>
        ) : showEnableForm ? (
          <div>
            {planActionError && (
              <div className="dashboard__error" role="alert" style={{ marginBottom: '12px' }}>
                [{planActionError.code}] {planActionError.message}
              </div>
            )}
            <div style={{ marginBottom: '12px' }}>
              <label style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'block', marginBottom: '4px' }}>
                Target Agent
              </label>
              <select
                value={agentKind}
                onChange={(e) => setAgentKind(e.target.value as AgentKind)}
                style={{ width: '100%' }}
              >
                <option value="claude-code">Claude Code</option>
                <option value="codex">Codex</option>
                <option value="pi">Pi</option>
                <option value="opencode">opencode</option>
              </select>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'block', marginBottom: '4px' }}>
                Scope
              </label>
              <select
                value={scopeType}
                onChange={(e) => {
                  setScopeType(e.target.value as ScopeType);
                  setProjectId(null);
                }}
                style={{ width: '100%' }}
              >
                <option value="global">Global</option>
                <option value="project">Project</option>
              </select>
            </div>
            {scopeType === 'project' && (
              <div style={{ marginBottom: '12px' }}>
                <label style={{ color: 'var(--color-text-muted)', fontSize: '12px', display: 'block', marginBottom: '4px' }}>
                  Project
                </label>
                <select
                  value={projectId ?? ''}
                  onChange={(e) => setProjectId(e.target.value || null)}
                  style={{ width: '100%' }}
                >
                  <option value="">Select a project…</option>
                  {projects.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>
            )}
            {isUnsupportedAgent && (
              <div
                style={{
                  marginBottom: '12px',
                  padding: '8px 12px',
                  background: 'var(--color-bg-elevated)',
                  border: '1px solid var(--color-border)',
                  borderRadius: '4px',
                  color: 'var(--color-danger)',
                  fontSize: '12px',
                }}
              >
                {agentKind} sub-agent sync is not supported.
              </div>
            )}
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
              <button type="button" onClick={() => setShowEnableForm(false)} disabled={planLoading}>
                Cancel
              </button>
              <button
                type="button"
                onClick={() => handleCreatePlan('enableSubAgent')}
                disabled={planLoading || isUnsupportedAgent || (scopeType === 'project' && !projectId)}
                style={{
                  background: 'var(--color-accent)',
                  borderColor: 'var(--color-accent)',
                  color: '#fff',
                }}
              >
                {planLoading ? 'Creating Plan…' : 'Enable'}
              </button>
            </div>
          </div>
        ) : isEditing ? (
          <SubAgentForm
            initial={detail.metadata}
            onCancel={() => setIsEditing(false)}
            onSaved={handleSaved}
            mode="update"
            slug={slug}
          />
        ) : (
          <div>
            <div style={{ marginBottom: '12px' }}>
              <label
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '12px',
                  display: 'block',
                  marginBottom: '4px',
                }}
              >
                Title
              </label>
              <div style={{ fontWeight: 600 }}>{detail.metadata.title}</div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '12px',
                  display: 'block',
                  marginBottom: '4px',
                }}
              >
                Slug
              </label>
              <div className="resource-list__meta">{detail.slug}</div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '12px',
                  display: 'block',
                  marginBottom: '4px',
                }}
              >
                Role
              </label>
              <div>{detail.metadata.role || '—'}</div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '12px',
                  display: 'block',
                  marginBottom: '4px',
                }}
              >
                Description
              </label>
              <div>{detail.metadata.description || '—'}</div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '12px',
                  display: 'block',
                  marginBottom: '4px',
                }}
              >
                Tags
              </label>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
                {detail.metadata.tags.length === 0 ? (
                  <span style={{ color: 'var(--color-text-muted)' }}>—</span>
                ) : (
                  detail.metadata.tags.map((tag) => (
                    <span
                      key={tag}
                      style={{
                        fontSize: '11px',
                        padding: '2px 6px',
                        borderRadius: '4px',
                        border: '1px solid var(--color-border)',
                        color: 'var(--color-text-muted)',
                      }}
                    >
                      {tag}
                    </span>
                  ))
                )}
              </div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '12px',
                  display: 'block',
                  marginBottom: '4px',
                }}
              >
                Agent Kinds
              </label>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
                {detail.metadata.agentKinds.length === 0 ? (
                  <span style={{ color: 'var(--color-text-muted)' }}>—</span>
                ) : (
                  detail.metadata.agentKinds.map((kind) => (
                    <span
                      key={kind}
                      style={{
                        fontSize: '11px',
                        padding: '2px 6px',
                        borderRadius: '4px',
                        background: 'var(--color-bg-elevated)',
                        color: 'var(--color-text-muted)',
                      }}
                    >
                      {AGENT_KIND_LABELS[kind]}
                    </span>
                  ))
                )}
              </div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '12px',
                  display: 'block',
                  marginBottom: '4px',
                }}
              >
                Bound MCPs
              </label>
              <div>
                {detail.metadata.boundMcpIds.length === 0
                  ? '—'
                  : detail.metadata.boundMcpIds.join(', ')}
              </div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label
                style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '12px',
                  display: 'block',
                  marginBottom: '4px',
                }}
              >
                Bound Skills
              </label>
              <div>
                {detail.metadata.boundSkillIds.length === 0
                  ? '—'
                  : detail.metadata.boundSkillIds.join(', ')}
              </div>
            </div>
            <div
              style={{
                display: 'flex',
                justifyContent: 'flex-end',
                gap: '8px',
                marginTop: '16px',
              }}
            >
              <button
                onClick={() => setIsEditing(true)}
                style={{
                  padding: '6px 12px',
                  background: 'var(--color-accent)',
                  border: '1px solid var(--color-accent)',
                  borderRadius: '4px',
                  color: '#fff',
                  cursor: 'pointer',
                }}
              >
                Edit
              </button>
              <button
                onClick={() => setShowEnableForm(true)}
                style={{
                  padding: '6px 12px',
                  background: 'var(--color-accent)',
                  border: '1px solid var(--color-accent)',
                  borderRadius: '4px',
                  color: '#fff',
                  cursor: 'pointer',
                }}
              >
                Enable to Agent
              </button>
              <button
                onClick={handleDelete}
                style={{
                  padding: '6px 12px',
                  background: 'transparent',
                  border: '1px solid var(--color-danger)',
                  borderRadius: '4px',
                  color: 'var(--color-danger)',
                  cursor: 'pointer',
                }}
              >
                Delete
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------
// CreateModal
// ------------------------------------------------------------------

function CreateModal({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: () => void;
}) {
  const [step, setStep] = useState<'template' | 'form'>('template');
  const [templates, setTemplates] = useState<LibraryEntry[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<LibraryEntry | null>(
    null
  );
  const [loadingTemplates, setLoadingTemplates] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);

  useEffect(() => {
    api.subAgents
      .templates()
      .then((t) => {
        setTemplates(t);
      })
      .catch((err: ApiError) => setError(err))
      .finally(() => setLoadingTemplates(false));
  }, []);

  const handleSelectTemplate = (template: LibraryEntry | null) => {
    setSelectedTemplate(template);
    setStep('form');
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginBottom: '16px',
          }}
        >
          <h2 style={{ margin: 0, fontSize: '16px' }}>
            {step === 'template' ? 'Choose Template' : 'Create Sub-agent'}
          </h2>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--color-text-muted)',
              cursor: 'pointer',
            }}
          >
            Close
          </button>
        </div>

        {error && (
          <div
            className="dashboard__error"
            role="alert"
            style={{ marginBottom: '12px' }}
          >
            [{error.code}] {error.message}
          </div>
        )}

        {step === 'template' ? (
          <div>
            {loadingTemplates ? (
              <div className="page__placeholder">Loading templates…</div>
            ) : (
              <div className="dashboard__grid">
                {templates.map((t) => (
                  <button
                    key={t.slug}
                    className="dashboard__card"
                    onClick={() => handleSelectTemplate(t)}
                    style={{
                      textAlign: 'left',
                      cursor: 'pointer',
                      width: '100%',
                    }}
                  >
                    <div style={{ fontWeight: 600, marginBottom: '4px' }}>
                      {t.metadata.title}
                    </div>
                    <div
                      className="resource-list__meta"
                      style={{ marginBottom: '8px' }}
                    >
                      {t.slug}
                    </div>
                    {t.metadata.role && (
                      <div
                        style={{
                          color: 'var(--color-text-muted)',
                          fontSize: '12px',
                          marginBottom: '8px',
                        }}
                      >
                        {t.metadata.role}
                      </div>
                    )}
                    <div
                      style={{
                        display: 'flex',
                        flexWrap: 'wrap',
                        gap: '4px',
                      }}
                    >
                      {t.metadata.tags.map((tag) => (
                        <span
                          key={tag}
                          style={{
                            fontSize: '11px',
                            padding: '2px 6px',
                            borderRadius: '4px',
                            border: '1px solid var(--color-border)',
                            color: 'var(--color-text-muted)',
                          }}
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  </button>
                ))}
                <button
                  className="dashboard__card"
                  onClick={() => handleSelectTemplate(null)}
                  style={{
                    textAlign: 'left',
                    cursor: 'pointer',
                    width: '100%',
                    borderStyle: 'dashed',
                  }}
                >
                  <div style={{ fontWeight: 600, marginBottom: '4px' }}>
                    Blank
                  </div>
                  <div
                    style={{
                      color: 'var(--color-text-muted)',
                      fontSize: '12px',
                    }}
                  >
                    Start from scratch with no pre-filled fields.
                  </div>
                </button>
              </div>
            )}
          </div>
        ) : (
          <SubAgentForm
            initial={
              selectedTemplate
                ? selectedTemplate.metadata
                : {
                    slug: '',
                    title: '',
                    role: '',
                    description: '',
                    tags: [],
                    agentKinds: [],
                    boundMcpIds: [],
                    boundSkillIds: [],
                    entryFile: null,
                    createdAt: '',
                    updatedAt: '',
                  }
            }
            onCancel={() => {
              setStep('template');
              setSelectedTemplate(null);
            }}
            onSaved={onCreated}
            mode="create"
          />
        )}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------
// SubAgentForm (shared for create and edit)
// ------------------------------------------------------------------

function SubAgentForm({
  initial,
  onCancel,
  onSaved,
  mode,
  slug,
}: {
  initial: LibraryMetadata;
  onCancel: () => void;
  onSaved: () => void;
  mode: 'create' | 'update';
  slug?: string;
}) {
  const [formSlug, setFormSlug] = useState(initial.slug);
  const [title, setTitle] = useState(initial.title);
  const [role, setRole] = useState(initial.role ?? '');
  const [description, setDescription] = useState(initial.description ?? '');
  const [tags, setTags] = useState(initial.tags.join(', '));
  const [agentKinds, setAgentKinds] = useState<AgentKind[]>(
    initial.agentKinds.filter((k) => k !== 'pi')
  );
  const [boundMcpIds, setBoundMcpIds] = useState<string[]>(
    initial.boundMcpIds
  );
  const [boundSkillIds, setBoundSkillIds] = useState<string[]>(
    initial.boundSkillIds
  );

  const [availableMcps, setAvailableMcps] = useState<ResourceRecord[]>([]);
  const [availableSkills, setAvailableSkills] = useState<LibraryEntry[]>([]);
  const [error, setError] = useState<ApiError | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    api.resources
      .list('mcp')
      .then((list) => setAvailableMcps(list))
      .catch(() => {});
    api.skills
      .list()
      .then((list) => setAvailableSkills(list))
      .catch(() => {});
  }, []);

  const slugValid = /^[a-z0-9]+(-[a-z0-9]+)*$/.test(formSlug);
  const canSubmit = mode === 'update' ? true : slugValid && formSlug.length > 0;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    setSaving(true);
    setError(null);

    const tagList = tags
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean);

    const payload: Partial<LibraryMetadata> = {
      title: title || formSlug,
      description: description || null,
      tags: tagList,
      role: role || null,
      agentKinds,
      boundMcpIds,
      boundSkillIds,
    };

    if (mode === 'create') {
      api.subAgents
        .create(formSlug, payload)
        .then(() => onSaved())
        .catch((err: ApiError) => {
          setError(err);
          setSaving(false);
        });
    } else if (slug) {
      api.subAgents
        .update(slug, payload)
        .then(() => onSaved())
        .catch((err: ApiError) => {
          setError(err);
          setSaving(false);
        });
    }
  };

  const toggleAgentKind = (kind: AgentKind) => {
    setAgentKinds((prev) =>
      prev.includes(kind) ? prev.filter((k) => k !== kind) : [...prev, kind]
    );
  };

  const toggleMcp = (id: string) => {
    setBoundMcpIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]
    );
  };

  const toggleSkill = (id: string) => {
    setBoundSkillIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]
    );
  };

  return (
    <form onSubmit={handleSubmit}>
      {mode === 'create' && (
        <label className="projects__field" style={{ marginBottom: '12px' }}>
          <span>Slug *</span>
          <input
            type="text"
            value={formSlug}
            onChange={(e) => setFormSlug(e.target.value)}
            placeholder="my-sub-agent"
            required
            pattern="^[a-z0-9]+(-[a-z0-9]+)*$"
            title="Lowercase kebab-case, e.g. my-sub-agent"
          />
          {!slugValid && formSlug.length > 0 && (
            <span style={{ color: 'var(--color-danger)', fontSize: '12px' }}>
              Must be lowercase kebab-case.
            </span>
          )}
        </label>
      )}
      <label className="projects__field" style={{ marginBottom: '12px' }}>
        <span>Title</span>
        <input
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="My Sub-agent"
        />
      </label>
      <label className="projects__field" style={{ marginBottom: '12px' }}>
        <span>Role</span>
        <input
          type="text"
          value={role}
          onChange={(e) => setRole(e.target.value)}
          placeholder="What does this sub-agent do?"
        />
      </label>
      <label className="projects__field" style={{ marginBottom: '12px' }}>
        <span>Description</span>
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Detailed description"
          rows={3}
          style={{
            padding: '8px 12px',
            background: 'var(--color-bg-elevated)',
            border: '1px solid var(--color-border)',
            borderRadius: '6px',
            color: 'var(--color-text)',
            fontSize: '13px',
            resize: 'vertical',
          }}
        />
      </label>
      <label className="projects__field" style={{ marginBottom: '12px' }}>
        <span>Tags (comma-separated)</span>
        <input
          type="text"
          value={tags}
          onChange={(e) => setTags(e.target.value)}
          placeholder="review, quality"
        />
      </label>

      <div className="projects__field" style={{ marginBottom: '12px' }}>
        <span style={{ display: 'block', marginBottom: '4px' }}>
          Agent Kinds
        </span>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
          {SUB_AGENT_AGENT_KINDS.map((kind) => (
            <label
              key={kind}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                cursor: 'pointer',
                fontSize: '13px',
              }}
            >
              <input
                type="checkbox"
                checked={agentKinds.includes(kind)}
                onChange={() => toggleAgentKind(kind)}
              />
              {AGENT_KIND_LABELS[kind]}
            </label>
          ))}
        </div>
      </div>

      <div className="projects__field" style={{ marginBottom: '12px' }}>
        <span style={{ display: 'block', marginBottom: '4px' }}>
          Bound MCPs
        </span>
        {availableMcps.length === 0 ? (
          <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
            No MCPs available.
          </span>
        ) : (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              maxHeight: '150px',
              overflow: 'auto',
              border: '1px solid var(--color-border)',
              borderRadius: '6px',
              padding: '8px',
            }}
          >
            {availableMcps.map((mcp) => (
              <label
                key={mcp.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  cursor: 'pointer',
                  fontSize: '13px',
                }}
              >
                <input
                  type="checkbox"
                  checked={boundMcpIds.includes(mcp.id)}
                  onChange={() => toggleMcp(mcp.id)}
                />
                {mcp.name}
              </label>
            ))}
          </div>
        )}
      </div>

      <div className="projects__field" style={{ marginBottom: '16px' }}>
        <span style={{ display: 'block', marginBottom: '4px' }}>
          Bound Skills
        </span>
        {availableSkills.length === 0 ? (
          <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
            No skills available.
          </span>
        ) : (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              maxHeight: '150px',
              overflow: 'auto',
              border: '1px solid var(--color-border)',
              borderRadius: '6px',
              padding: '8px',
            }}
          >
            {availableSkills.map((skill) => (
              <label
                key={skill.slug}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  cursor: 'pointer',
                  fontSize: '13px',
                }}
              >
                <input
                  type="checkbox"
                  checked={boundSkillIds.includes(skill.slug)}
                  onChange={() => toggleSkill(skill.slug)}
                />
                {skill.metadata.title}
              </label>
            ))}
          </div>
        )}
      </div>

      {error && (
        <div
          className="dashboard__error"
          role="alert"
          style={{ marginBottom: '12px' }}
        >
          [{error.code}] {error.message}
        </div>
      )}

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
        <button type="button" onClick={onCancel} disabled={saving}>
          {mode === 'create' ? 'Back' : 'Cancel'}
        </button>
        <button
          type="submit"
          disabled={!canSubmit || saving}
          style={{
            background: 'var(--color-accent)',
            borderColor: 'var(--color-accent)',
            color: '#fff',
          }}
        >
          {saving ? 'Saving…' : mode === 'create' ? 'Create' : 'Save'}
        </button>
      </div>
    </form>
  );
}
