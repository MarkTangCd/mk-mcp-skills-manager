import { useEffect, useState } from 'react';

import { ApiError, api, type DashboardSnapshot } from '../lib/api';
import type { Agent, DoctorIssue } from '../types/domain';

// Module-level cache so navigating away and back re-uses the last snapshot
// (revalidating in the background) instead of blocking on a fresh IPC round-trip.
let cachedSnapshot: DashboardSnapshot | null = null;
let inFlight: Promise<DashboardSnapshot> | null = null;

function fetchDashboard(): Promise<DashboardSnapshot> {
  if (inFlight) return inFlight;
  inFlight = api.app
    .getDashboard()
    .then((value) => {
      cachedSnapshot = value;
      return value;
    })
    .finally(() => {
      inFlight = null;
    });
  return inFlight;
}

export default function DashboardPage() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(cachedSnapshot);
  const [error, setError] = useState<ApiError | null>(null);
  const [loading, setLoading] = useState(cachedSnapshot === null);

  useEffect(() => {
    let cancelled = false;
    if (cachedSnapshot === null) setLoading(true);
    fetchDashboard()
      .then((value) => {
        if (cancelled) return;
        setSnapshot(value);
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
  }, []);

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">Dashboard</h1>
        <p className="page__subtitle">Overview of agents, scans, issues, and recent changes.</p>
      </header>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      {loading && !snapshot && <div className="page__placeholder">Loading…</div>}

      {snapshot && (
        <>
          <section className="dashboard__grid">
            <Card label="Agents" value={snapshot.agents.length} />
            <Card label="Recent scans" value={snapshot.recentScans.length} />
            <IssueCard issues={snapshot.openIssues} />
            <Card label="Recent changes" value={snapshot.recentChanges.length} />
          </section>

          <section className="dashboard__section">
            <h2 className="dashboard__section-title">Detected Agents</h2>
            <div className="dashboard__agent-grid">
              {snapshot.agents.map((agent) => (
                <AgentCard key={agent.id} agent={agent} />
              ))}
            </div>
          </section>

          <dl className="dashboard__bootstrap">
            <dt>Data dir</dt>
            <dd>{snapshot.bootstrap.dataDir}</dd>
            <dt>Database</dt>
            <dd>{snapshot.bootstrap.databasePath}</dd>
            <dt>Schema version</dt>
            <dd>{snapshot.bootstrap.schemaVersion}</dd>
          </dl>
        </>
      )}
    </div>
  );
}

function AgentCard({ agent }: { agent: Agent }) {
  return (
    <div className="dashboard__agent-card">
      <div className="dashboard__agent-header">
        <div>
          <div className="dashboard__agent-name">{agent.displayName}</div>
          <div className="dashboard__agent-kind">{agent.kind}</div>
        </div>
        <span className={`dashboard__status dashboard__status--${agent.healthStatus}`}>
          {agent.installed ? 'Installed' : 'Not installed'}
        </span>
      </div>
      <div className="dashboard__agent-version">{agent.version ?? 'No version detected'}</div>
    </div>
  );
}

function Card({ label, value }: { label: string; value: number }) {
  return (
    <div className="dashboard__card">
      <div className="dashboard__card-label">{label}</div>
      <div className="dashboard__card-value">{value}</div>
    </div>
  );
}

function IssueCard({ issues }: { issues: DoctorIssue[] }) {
  const critical = issues.filter((i) => i.severity === 'critical').length;
  const warning = issues.filter((i) => i.severity === 'warning').length;
  const info = issues.filter((i) => i.severity === 'info').length;
  return (
    <div className="dashboard__card">
      <div className="dashboard__card-label">Open issues</div>
      <div className="dashboard__card-value">{issues.length}</div>
      <div className="dashboard__issue-breakdown">
        {critical > 0 && (
          <span className="dashboard__issue-dot dashboard__issue-dot--critical">
            {critical} critical
          </span>
        )}
        {warning > 0 && (
          <span className="dashboard__issue-dot dashboard__issue-dot--warning">
            {warning} warning
          </span>
        )}
        {info > 0 && (
          <span className="dashboard__issue-dot dashboard__issue-dot--info">
            {info} info
          </span>
        )}
      </div>
    </div>
  );
}
