import { useCallback, useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';

import { ApiError, ProjectScanReport, api } from '../lib/api';
import type { Project, ScanSnapshot } from '../types/domain';

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [project, setProject] = useState<Project | null>(null);
  const [snapshots, setSnapshots] = useState<ScanSnapshot[]>([]);
  const [report, setReport] = useState<ProjectScanReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);

  const load = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const [p, latest] = await Promise.all([
        api.projects.get(id),
        api.projects.latestScans(id),
      ]);
      setProject(p);
      setSnapshots(latest);
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    void load();
  }, [load]);

  async function onRescan() {
    if (!id) return;
    setScanning(true);
    try {
      const r = await api.projects.rescan(id);
      setReport(r);
      setSnapshots(r.snapshots);
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    } finally {
      setScanning(false);
    }
  }

  if (loading && !project) {
    return <div className="page__placeholder">Loading…</div>;
  }

  if (!project) {
    return (
      <div className="page">
        <div className="dashboard__error">Project not found.</div>
        <Link to="/projects">Back to projects</Link>
      </div>
    );
  }

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">{project.name}</h1>
        <p className="page__subtitle">{project.path}</p>
      </header>

      <div className="project-detail__actions">
        <button type="button" onClick={onRescan} disabled={scanning}>
          {scanning ? 'Scanning…' : 'Rescan'}
        </button>
        <Link to="/projects" className="project-detail__back">
          Back to projects
        </Link>
      </div>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      {report && report.adapterErrors.length > 0 && (
        <div className="dashboard__error" role="status">
          {report.adapterErrors.length} adapter(s) reported errors:
          <ul>
            {report.adapterErrors.map((e) => (
              <li key={e.agentKind}>
                {e.agentKind}: {e.message}
              </li>
            ))}
          </ul>
        </div>
      )}

      <section>
        <h2>Latest scans</h2>
        {snapshots.length === 0 ? (
          <div className="page__placeholder">No scans yet. Click Rescan.</div>
        ) : (
          <table className="projects__table">
            <thead>
              <tr>
                <th>Agent</th>
                <th>Scanned at</th>
                <th>MCP</th>
                <th>Skills</th>
                <th>Sub-agents</th>
                <th>Pi</th>
                <th>Errors</th>
              </tr>
            </thead>
            <tbody>
              {snapshots.map((s) => (
                <tr key={s.id}>
                  <td>{s.agentKind ?? '—'}</td>
                  <td>{s.createdAt.slice(0, 19).replace('T', ' ')}</td>
                  <td>{s.summary.mcpCount}</td>
                  <td>{s.summary.skillCount}</td>
                  <td>{s.summary.subAgentCount}</td>
                  <td>{s.summary.piResourceCount}</td>
                  <td>{s.summary.errors.length === 0 ? '—' : s.summary.errors.join('; ')}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
