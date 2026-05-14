import { useEffect, useState } from 'react';

import { ApiError, api, type DashboardSnapshot } from '../lib/api';

export default function DashboardPage() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    api.app
      .getDashboard()
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
            <Card label="Open issues" value={snapshot.openIssues.length} />
            <Card label="Recent changes" value={snapshot.recentChanges.length} />
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

function Card({ label, value }: { label: string; value: number }) {
  return (
    <div className="dashboard__card">
      <div className="dashboard__card-label">{label}</div>
      <div className="dashboard__card-value">{value}</div>
    </div>
  );
}
