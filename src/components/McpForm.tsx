import { useEffect, useState } from 'react';

import { ApiError, api } from '../lib/api';
import type { AgentKind, ChangeIntent, McpServer, Project, ScopeType } from '../types/domain';

interface McpFormProps {
  mode: 'create' | 'edit';
  initialData?: McpServer;
  initialAgentKind?: AgentKind;
  initialScopeType?: ScopeType;
  initialProjectId?: string | null;
  onSubmit: (intent: ChangeIntent) => void;
  onCancel: () => void;
}

const AGENT_KINDS: AgentKind[] = ['claude-code', 'codex', 'opencode', 'pi'];
const TRANSPORTS: McpServer['transport'][] = ['stdio', 'sse', 'http'];

export default function McpForm({
  mode,
  initialData,
  initialAgentKind,
  initialScopeType,
  initialProjectId,
  onSubmit,
  onCancel,
}: McpFormProps) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loadingProjects, setLoadingProjects] = useState(true);
  const [projectsError, setProjectsError] = useState<ApiError | null>(null);

  const [name, setName] = useState(initialData?.name ?? '');
  const [transport, setTransport] = useState<McpServer['transport']>(initialData?.transport ?? 'stdio');
  const [command, setCommand] = useState(initialData?.command ?? '');
  const [args, setArgs] = useState(initialData?.args.join(', ') ?? '');
  const [url, setUrl] = useState(initialData?.url ?? '');
  const [envRefs, setEnvRefs] = useState(initialData?.envRefs.join(', ') ?? '');
  const [enabled, setEnabled] = useState(initialData?.enabled ?? true);
  const [agentKind, setAgentKind] = useState<AgentKind>(initialAgentKind ?? 'claude-code');
  const [scopeType, setScopeType] = useState<ScopeType>(initialScopeType ?? 'global');
  const [projectId, setProjectId] = useState(initialProjectId ?? '');

  const [errors, setErrors] = useState<string[]>([]);

  useEffect(() => {
    api.projects
      .list()
      .then((list) => {
        setProjects(list);
        setProjectsError(null);
      })
      .catch((err: ApiError) => {
        setProjectsError(err);
      })
      .finally(() => {
        setLoadingProjects(false);
      });
  }, []);

  const validate = (): boolean => {
    const errs: string[] = [];
    if (!name.trim()) {
      errs.push('Name is required');
    }
    if (!transport) {
      errs.push('Transport is required');
    }
    if (transport === 'stdio' && !command.trim()) {
      errs.push('Command is required for stdio transport');
    }
    if ((transport === 'http' || transport === 'sse') && !url.trim()) {
      errs.push('URL is required for HTTP/SSE transport');
    }
    const envList = envRefs
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    for (const env of envList) {
      if (env.includes('=')) {
        errs.push(`Environment reference "${env}" must not contain '=' (only variable names allowed)`);
      }
    }
    setErrors(errs);
    return errs.length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    const argsList = args
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    const envList = envRefs
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);

    const payload: McpServer = {
      id: initialData?.id ?? '',
      name: name.trim(),
      transport,
      command: command.trim() || null,
      args: argsList,
      url: url.trim() || null,
      envRefs: envList,
      enabled,
    };

    const changeType = mode === 'create' ? 'createMcp' : 'updateMcp';

    const intent: ChangeIntent = {
      id: crypto.randomUUID(),
      changeType,
      agentKind,
      projectId: scopeType === 'project' ? projectId || null : null,
      scopeType,
      resourceId: initialData?.id ?? null,
      payload,
      createdAt: new Date().toISOString(),
    };

    onSubmit(intent);
  };

  return (
    <form className="mcp-form" onSubmit={handleSubmit}>
      {errors.length > 0 && (
        <div className="mcp-form__errors" role="alert">
          {errors.map((err) => (
            <div key={err}>{err}</div>
          ))}
        </div>
      )}

      <div className="projects__form" style={{ marginBottom: 'var(--space-3)' }}>
        <label className="projects__field">
          <span>Name</span>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. fetch"
            required
          />
        </label>

        <label className="projects__field">
          <span>Transport</span>
          <select value={transport} onChange={(e) => setTransport(e.target.value as McpServer['transport'])}>
            {TRANSPORTS.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        </label>

        <label className="projects__field">
          <span>Enabled</span>
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
            style={{ width: 'auto', marginTop: 'var(--space-2)' }}
          />
        </label>
      </div>

      <div className="projects__form" style={{ marginBottom: 'var(--space-3)' }}>
        <label className="projects__field">
          <span>Command {transport === 'stdio' && <strong style={{ color: 'var(--color-danger)' }}>*</strong>}</span>
          <input
            type="text"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            placeholder={transport === 'stdio' ? 'Required' : 'Optional'}
            disabled={transport !== 'stdio'}
          />
        </label>

        <label className="projects__field">
          <span>Args (comma-separated)</span>
          <input
            type="text"
            value={args}
            onChange={(e) => setArgs(e.target.value)}
            placeholder="e.g. --port, 8080"
          />
        </label>

        <label className="projects__field">
          <span>URL {(transport === 'http' || transport === 'sse') && <strong style={{ color: 'var(--color-danger)' }}>*</strong>}</span>
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder={transport === 'http' || transport === 'sse' ? 'Required' : 'Optional'}
            disabled={transport !== 'http' && transport !== 'sse'}
          />
        </label>
      </div>

      <div className="projects__form" style={{ marginBottom: 'var(--space-3)' }}>
        <label className="projects__field">
          <span>Env Refs (comma-separated, no '=')</span>
          <input
            type="text"
            value={envRefs}
            onChange={(e) => setEnvRefs(e.target.value)}
            placeholder="e.g. API_KEY, PATH"
          />
        </label>

        <label className="projects__field">
          <span>Target Agent</span>
          <select value={agentKind} onChange={(e) => setAgentKind(e.target.value as AgentKind)}>
            {AGENT_KINDS.map((k) => (
              <option key={k} value={k}>
                {k}
              </option>
            ))}
          </select>
        </label>

        <label className="projects__field">
          <span>Scope</span>
          <select value={scopeType} onChange={(e) => setScopeType(e.target.value as ScopeType)}>
            <option value="global">Global</option>
            <option value="project">Project</option>
          </select>
        </label>
      </div>

      {scopeType === 'project' && (
        <div className="projects__form" style={{ marginBottom: 'var(--space-3)' }}>
          <label className="projects__field">
            <span>Project</span>
            <select
              value={projectId}
              onChange={(e) => setProjectId(e.target.value)}
              disabled={loadingProjects}
            >
              <option value="">Select a project…</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            {projectsError && (
              <span style={{ color: 'var(--color-danger)', fontSize: '12px' }}>
                {projectsError.message}
              </span>
            )}
          </label>
        </div>
      )}

      <div className="mcp-form__actions" style={{ display: 'flex', gap: 'var(--space-3)', justifyContent: 'flex-end', marginTop: 'var(--space-4)' }}>
        <button type="button" className="diff-preview__btn" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="diff-preview__btn diff-preview__btn--primary">
          {mode === 'create' ? 'Create MCP' : 'Update MCP'}
        </button>
      </div>
    </form>
  );
}
