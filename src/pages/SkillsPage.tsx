import { useEffect, useMemo, useState } from 'react';

import DiffPreview from '../components/DiffPreview';
import { ApiError, api } from '../lib/api';
import type { AgentKind, ChangeIntent, ChangePlan, LibraryEntry, LibraryEntryDetail, Project, ScopeType } from '../types/domain';

// ------------------------------------------------------------------
// SkillsPage: Skill Library management UI
// ------------------------------------------------------------------

type ModalMode = 'detail' | 'create' | 'import' | null;

export default function SkillsPage() {
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
    api.skills
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

  const handleCreated = () => {
    closeModal();
    load();
  };

  const handleImported = () => {
    closeModal();
    load();
  };

  const handleDeleted = () => {
    closeModal();
    load();
  };

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">Skill Library</h1>
        <p className="page__subtitle">Create, import and manage reusable skills.</p>
      </header>

      <div className="resource-toolbar">
        <label className="projects__field">
          <span>Search</span>
          <input
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Slug, title, description, tags"
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
        <div className="projects__field" style={{ display: 'flex', gap: '8px', alignItems: 'flex-end' }}>
          <button onClick={() => setModalMode('create')}>Create Skill</button>
          <button onClick={() => setModalMode('import')}>Import Skill</button>
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
          No skills found. Create a new skill or import an existing one.
        </div>
      ) : (
        <div className="dashboard__grid">
          {entries.map((entry) => (
            <SkillCard
              key={entry.slug}
              entry={entry}
              onClick={() => openDetail(entry.slug)}
            />
          ))}
        </div>
      )}

      {modalMode === 'detail' && selectedSlug && (
        <DetailModal slug={selectedSlug} onClose={closeModal} onDelete={handleDeleted} />
      )}
      {modalMode === 'create' && (
        <CreateModal onClose={closeModal} onCreated={handleCreated} />
      )}
      {modalMode === 'import' && (
        <ImportModal onClose={closeModal} onImported={handleImported} />
      )}
    </div>
  );
}

// ------------------------------------------------------------------
// SkillCard
// ------------------------------------------------------------------

function SkillCard({
  entry,
  onClick,
}: {
  entry: LibraryEntry;
  onClick: () => void;
}) {
  const { metadata } = entry;
  const desc = metadata.description ?? '';
  const summary = desc.length > 120 ? desc.slice(0, 120) + '…' : desc;

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
      {summary && (
        <div style={{ color: 'var(--color-text-muted)', fontSize: '12px', marginBottom: '8px' }}>
          {summary}
        </div>
      )}
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
}: {
  slug: string;
  onClose: () => void;
  onDelete: () => void;
}) {
  const [detail, setDetail] = useState<LibraryEntryDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);

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
    api.skills
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

  const entryContent = useMemo(() => {
    if (!detail) return null;
    const fileName = detail.metadata.entryFile;
    if (!fileName) return null;
    return detail.files[fileName] ?? null;
  }, [detail]);

  const handleDelete = () => {
    if (!confirm(`Delete skill "${slug}"? This cannot be undone.`)) return;
    api.skills
      .delete(slug)
      .then(() => onDelete())
      .catch((err: ApiError) => setError(err));
  };

  const buildIntent = (changeType: 'enableSkill' | 'disableSkill'): ChangeIntent => {
    const payload: Record<string, unknown> = {
      slug,
      title: detail?.metadata.title ?? slug,
      description: detail?.metadata.description ?? null,
      tags: detail?.metadata.tags ?? [],
    };
    return {
      id: crypto.randomUUID(),
      changeType,
      agentKind,
      projectId,
      scopeType,
      resourceId: `library:skill:${slug}`,
      payload,
      createdAt: new Date().toISOString(),
    };
  };

  const handleCreatePlan = async (changeType: 'enableSkill' | 'disableSkill') => {
    if (agentKind === 'opencode') {
      setPlanActionError(new ApiError({
        code: 'unsupported',
        message: 'opencode skill sync is not supported',
        recoverable: true,
      }));
      return;
    }
    setPlanLoading(true);
    setPlanActionError(null);
    try {
      const created = await (changeType === 'enableSkill'
        ? api.skills.enable(buildIntent(changeType))
        : api.skills.disable(buildIntent(changeType)));
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

  const isUnsupportedAgent = agentKind === 'opencode';

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <h2 style={{ margin: 0, fontSize: '16px' }}>Skill Detail</h2>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', color: 'var(--color-text-muted)', cursor: 'pointer' }}>
            Close
          </button>
        </div>

        {error && (
          <div className="dashboard__error" role="alert" style={{ marginBottom: '12px' }}>
            [{error.code}] {error.message}
          </div>
        )}

        {loading ? (
          <div className="page__placeholder">Loading…</div>
        ) : !detail ? (
          <div className="page__placeholder">Skill not found.</div>
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
                opencode skill sync is not supported.
              </div>
            )}
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
              <button type="button" onClick={() => setShowEnableForm(false)} disabled={planLoading}>
                Cancel
              </button>
              <button
                type="button"
                onClick={() => handleCreatePlan('enableSkill')}
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
        ) : (
          <div>
            <div style={{ marginBottom: '12px' }}>
              <label style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>Title</label>
              <div style={{ fontWeight: 600 }}>{detail.metadata.title}</div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>Slug</label>
              <div className="resource-list__meta">{detail.slug}</div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>Description</label>
              <div>{detail.metadata.description || '—'}</div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>Tags</label>
              <div>
                {detail.metadata.tags.length === 0
                  ? '—'
                  : detail.metadata.tags.map((tag) => (
                      <span
                        key={tag}
                        style={{
                          fontSize: '11px',
                          padding: '2px 6px',
                          borderRadius: '4px',
                          border: '1px solid var(--color-border)',
                          color: 'var(--color-text-muted)',
                          marginRight: '4px',
                        }}
                      >
                        {tag}
                      </span>
                    ))}
              </div>
            </div>
            <div style={{ marginBottom: '12px' }}>
              <label style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>Entry File</label>
              <div className="resource-list__meta">{detail.metadata.entryFile || '—'}</div>
            </div>
            {entryContent !== null && (
              <div style={{ marginBottom: '12px' }}>
                <label style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>Entry File Preview</label>
                <pre
                  style={{
                    background: 'var(--color-bg)',
                    border: '1px solid var(--color-border)',
                    borderRadius: '4px',
                    padding: '12px',
                    maxHeight: '300px',
                    overflow: 'auto',
                    fontSize: '12px',
                    marginTop: '4px',
                  }}
                >
                  {entryContent || '(empty file)'}
                </pre>
              </div>
            )}
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px', marginTop: '16px' }}>
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
  const [slug, setSlug] = useState('');
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [tags, setTags] = useState('');
  const [entryFile, setEntryFile] = useState('skill.md');
  const [error, setError] = useState<ApiError | null>(null);
  const [saving, setSaving] = useState(false);

  const slugValid = /^[a-z0-9]+(-[a-z0-9]+)*$/.test(slug);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!slugValid) return;
    setSaving(true);
    setError(null);
    const tagList = tags
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean);
    api.skills
      .create(slug, title || slug, description, tagList, entryFile || undefined)
      .then(() => onCreated())
      .catch((err: ApiError) => {
        setError(err);
        setSaving(false);
      });
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <h2 style={{ margin: 0, fontSize: '16px' }}>Create Skill</h2>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', color: 'var(--color-text-muted)', cursor: 'pointer' }}>
            Close
          </button>
        </div>

        {error && (
          <div className="dashboard__error" role="alert" style={{ marginBottom: '12px' }}>
            [{error.code}] {error.message}
          </div>
        )}

        <form onSubmit={handleSubmit}>
          <label className="projects__field" style={{ marginBottom: '12px' }}>
            <span>Slug *</span>
            <input
              type="text"
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="my-skill"
              required
              pattern="^[a-z0-9]+(-[a-z0-9]+)*$"
              title="Lowercase kebab-case, e.g. my-skill"
            />
            {!slugValid && slug.length > 0 && (
              <span style={{ color: 'var(--color-danger)', fontSize: '12px' }}>
                Must be lowercase kebab-case.
              </span>
            )}
          </label>
          <label className="projects__field" style={{ marginBottom: '12px' }}>
            <span>Title</span>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="My Skill"
            />
          </label>
          <label className="projects__field" style={{ marginBottom: '12px' }}>
            <span>Description</span>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="What does this skill do?"
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
          <label className="projects__field" style={{ marginBottom: '16px' }}>
            <span>Entry File Name</span>
            <input
              type="text"
              value={entryFile}
              onChange={(e) => setEntryFile(e.target.value)}
              placeholder="skill.md"
            />
          </label>
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
            <button type="button" onClick={onClose} disabled={saving}>
              Cancel
            </button>
            <button
              type="submit"
              disabled={!slugValid || saving}
              style={{
                background: 'var(--color-accent)',
                borderColor: 'var(--color-accent)',
                color: '#fff',
              }}
            >
              {saving ? 'Creating…' : 'Create'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ------------------------------------------------------------------
// ImportModal
// ------------------------------------------------------------------

function ImportModal({
  onClose,
  onImported,
}: {
  onClose: () => void;
  onImported: () => void;
}) {
  const [sourcePath, setSourcePath] = useState('');
  const [slug, setSlug] = useState('');
  const [error, setError] = useState<ApiError | null>(null);
  const [saving, setSaving] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!sourcePath.trim()) return;
    setSaving(true);
    setError(null);
    api.skills
      .import(sourcePath.trim(), slug.trim() || undefined)
      .then(() => onImported())
      .catch((err: ApiError) => {
        setError(err);
        setSaving(false);
      });
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <h2 style={{ margin: 0, fontSize: '16px' }}>Import Skill</h2>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', color: 'var(--color-text-muted)', cursor: 'pointer' }}>
            Close
          </button>
        </div>

        {error && (
          <div className="dashboard__error" role="alert" style={{ marginBottom: '12px' }}>
            [{error.code}] {error.message}
          </div>
        )}

        <form onSubmit={handleSubmit}>
          <label className="projects__field" style={{ marginBottom: '12px' }}>
            <span>Source Directory Path *</span>
            <input
              type="text"
              value={sourcePath}
              onChange={(e) => setSourcePath(e.target.value)}
              placeholder="/path/to/my-skill"
              required
            />
          </label>
          <label className="projects__field" style={{ marginBottom: '16px' }}>
            <span>Slug (optional)</span>
            <input
              type="text"
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="my-skill"
            />
            <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
              Leave empty to derive from directory name.
            </span>
          </label>
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
            <button type="button" onClick={onClose} disabled={saving}>
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              style={{
                background: 'var(--color-accent)',
                borderColor: 'var(--color-accent)',
                color: '#fff',
              }}
            >
              {saving ? 'Importing…' : 'Import'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
