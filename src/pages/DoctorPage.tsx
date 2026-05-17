import { useEffect, useMemo, useState } from 'react';

import { ApiError, api } from '../lib/api';
import type { DoctorIssue, IssueSeverity, Project } from '../types/domain';

const SEVERITIES: IssueSeverity[] = ['critical', 'warning', 'info'];
const CATEGORIES = ['agent', 'mcp', 'skill', 'sub-agent', 'pi'];

export default function DoctorPage() {
  const [issues, setIssues] = useState<DoctorIssue[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);

  const [severityFilter, setSeverityFilter] = useState<IssueSeverity | ''>('');
  const [categoryFilter, setCategoryFilter] = useState('');
  const [projectFilter, setProjectFilter] = useState('');

  const load = async () => {
    try {
      setLoading(true);
      const [issueList, projectList] = await Promise.all([
        api.doctor.listIssues(
          severityFilter || undefined,
          categoryFilter || undefined,
          projectFilter || undefined,
        ),
        api.projects.list(),
      ]);
      setIssues(issueList);
      setProjects(projectList);
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    } finally {
      setLoading(false);
    }
  };

  const runDoctor = async () => {
    try {
      setRunning(true);
      await api.doctor.runAll();
      await load();
    } catch (err) {
      setError(err as ApiError);
    } finally {
      setRunning(false);
    }
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [severityFilter, categoryFilter, projectFilter]);

  const counts = useMemo(() => {
    const c = { critical: 0, warning: 0, info: 0 };
    issues.forEach((i) => {
      if (i.severity === 'critical') c.critical++;
      if (i.severity === 'warning') c.warning++;
      if (i.severity === 'info') c.info++;
    });
    return c;
  }, [issues]);

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">Doctor</h1>
        <p className="page__subtitle">
          Read-only environment diagnostics. No automatic fixes are applied.
        </p>
      </header>

      {error && (
        <div className="dashboard__error" role="alert" style={{ marginBottom: 16 }}>
          [{error.code}] {error.message}
        </div>
      )}

      <div className="doctor__toolbar">
        <button onClick={runDoctor} disabled={running}>
          {running ? 'Running…' : 'Run Doctor'}
        </button>

        <div className="doctor__filters">
          <select
            value={severityFilter}
            onChange={(e) => setSeverityFilter(e.target.value as IssueSeverity | '')}
          >
            <option value="">All severities</option>
            {SEVERITIES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>

          <select
            value={categoryFilter}
            onChange={(e) => setCategoryFilter(e.target.value)}
          >
            <option value="">All categories</option>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>

          <select
            value={projectFilter}
            onChange={(e) => setProjectFilter(e.target.value)}
          >
            <option value="">All projects</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="doctor__summary">
        <div className="doctor__summary-pill doctor__summary-pill--critical">
          <strong>{counts.critical}</strong> <span>critical</span>
        </div>
        <div className="doctor__summary-pill doctor__summary-pill--warning">
          <strong>{counts.warning}</strong> <span>warning</span>
        </div>
        <div className="doctor__summary-pill doctor__summary-pill--info">
          <strong>{counts.info}</strong> <span>info</span>
        </div>
      </div>

      {loading ? (
        <div className="page__placeholder">Loading issues…</div>
      ) : issues.length === 0 ? (
        <div className="page__placeholder">
          No issues found. Click “Run Doctor” to check again.
        </div>
      ) : (
        <table className="projects__table doctor__table">
          <thead>
            <tr>
              <th>Severity</th>
              <th>Category</th>
              <th>Message</th>
              <th>Target</th>
              <th>Fixable</th>
            </tr>
          </thead>
          <tbody>
            {issues.map((issue) => (
              <tr key={issue.id}>
                <td>
                  <span className={`doctor__badge doctor__badge--${issue.severity}`}>
                    {issue.severity}
                  </span>
                </td>
                <td>{issue.category}</td>
                <td className="doctor__message">{issue.message}</td>
                <td className="doctor__target">
                  {issue.targetRef ? (
                    <div>
                      {issue.targetRef.resourceType && (
                        <div className="doctor__target-line">
                          {issue.targetRef.resourceType}
                        </div>
                      )}
                      {issue.targetRef.configPath && (
                        <div className="doctor__target-line doctor__target-line--path">
                          {issue.targetRef.configPath}
                        </div>
                      )}
                    </div>
                  ) : (
                    <span className="doctor__target-line--empty">—</span>
                  )}
                </td>
                <td>
                  {issue.fixable ? (
                    <span className="doctor__fixable">Yes</span>
                  ) : (
                    <span className="doctor__not-fixable">No</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
