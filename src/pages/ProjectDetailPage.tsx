import { useCallback, useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';

import MatrixTable from '../components/MatrixTable';
import { ApiError, ProjectScanReport, api } from '../lib/api';
import type { MatrixCell, MatrixRow, Project, ProjectMatrix, ScanSnapshot } from '../types/domain';

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [project, setProject] = useState<Project | null>(null);
  const [snapshots, setSnapshots] = useState<ScanSnapshot[]>([]);
  const [matrix, setMatrix] = useState<ProjectMatrix | null>(null);
  const [report, setReport] = useState<ProjectScanReport | null>(null);
  const [matrixFilter, setMatrixFilter] = useState('');
  const [selectedCell, setSelectedCell] = useState<{
    cell: MatrixCell;
    row: MatrixRow;
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);

  const load = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const [p, latest, projectMatrix] = await Promise.all([
        api.projects.get(id),
        api.projects.latestScans(id),
        api.projects.getMatrix(id),
      ]);
      setProject(p);
      setSnapshots(latest);
      setMatrix(projectMatrix);
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
      const projectMatrix = await api.projects.getMatrix(id);
      setReport(r);
      setSnapshots(r.snapshots);
      setMatrix(projectMatrix);
      setSelectedCell(null);
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

      <div className="resource-toolbar resource-toolbar--compact">
        <label className="projects__field">
          <span>Matrix filter</span>
          <input
            type="search"
            value={matrixFilter}
            onChange={(e) => setMatrixFilter(e.target.value)}
            placeholder="Filter resources"
          />
        </label>
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

      {matrix && (
        <>
          <section className="pi-summary">
            <h2 className="dashboard__section-title">Pi Resource Summary</h2>
            <div className="pi-summary__grid">
              <SummaryPill label="Total" value={matrix.piResourceSummary.total} />
              <SummaryPill label="Enabled" value={matrix.piResourceSummary.enabled} />
              <SummaryPill label="Disabled" value={matrix.piResourceSummary.disabled} />
              <SummaryPill label="Missing" value={matrix.piResourceSummary.missing} />
              <SummaryPill label="Untrusted" value={matrix.piResourceSummary.untrusted} />
            </div>
            {matrix.piResourceSummary.byKind.length > 0 && (
              <table className="projects__table pi-summary__table">
                <thead>
                  <tr>
                    <th>Kind</th>
                    <th>Total</th>
                    <th>Enabled</th>
                    <th>Disabled</th>
                    <th>Missing</th>
                    <th>Untrusted</th>
                  </tr>
                </thead>
                <tbody>
                  {matrix.piResourceSummary.byKind.map((item) => (
                    <tr key={item.resourceType}>
                      <td>{item.resourceType}</td>
                      <td>{item.total}</td>
                      <td>{item.enabled}</td>
                      <td>{item.disabled}</td>
                      <td>{item.missing}</td>
                      <td>{item.untrusted}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>

          <MatrixTable
            title="MCP Matrix"
            rows={matrix.mcpMatrix}
            agents={matrix.agents}
            filter={matrixFilter}
            onSelectCell={(cell, row) => setSelectedCell({ cell, row })}
          />
          <MatrixTable
            title="Skills Matrix"
            rows={matrix.skillsMatrix}
            agents={matrix.agents}
            filter={matrixFilter}
            onSelectCell={(cell, row) => setSelectedCell({ cell, row })}
          />
          <MatrixTable
            title="Sub-agent Matrix"
            rows={matrix.subAgentMatrix}
            agents={matrix.agents}
            filter={matrixFilter}
            onSelectCell={(cell, row) => setSelectedCell({ cell, row })}
          />

          {selectedCell && (
            <section className="source-panel">
              <div className="source-panel__header">
                <h2 className="dashboard__section-title">
                  Source Details: {selectedCell.row.name}
                </h2>
                <button type="button" onClick={() => setSelectedCell(null)}>
                  Close
                </button>
              </div>
              {selectedCell.cell.sources.length === 0 ? (
                <div className="page__placeholder">No source for this cell.</div>
              ) : (
                <table className="projects__table">
                  <thead>
                    <tr>
                      <th>Agent</th>
                      <th>Scope</th>
                      <th>Status</th>
                      <th>Enabled</th>
                      <th>Config path</th>
                      <th>Source path</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedCell.cell.sources.map((source) => (
                      <tr key={`${source.resourceId}-${source.scopeType}-${source.configPath}`}>
                        <td>{selectedCell.cell.agentKind}</td>
                        <td>{source.scopeType}</td>
                        <td>{source.status}</td>
                        <td>{source.enabled ? 'yes' : 'no'}</td>
                        <td className="projects__path">{source.configPath ?? '—'}</td>
                        <td className="projects__path">{source.sourcePath ?? '—'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>
          )}
        </>
      )}
    </div>
  );
}

function SummaryPill({ label, value }: { label: string; value: number }) {
  return (
    <div className="pi-summary__pill">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
