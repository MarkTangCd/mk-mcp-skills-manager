import { useCallback, useEffect, useState } from 'react';

import { ApiError, api } from '../lib/api';
import type { Backup } from '../types/domain';

export default function BackupsPage() {
  const [backups, setBackups] = useState<Backup[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [confirmId, setConfirmId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.backups.list();
      setBackups(list);
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

  async function handleRestore(id: string) {
    try {
      await api.backups.restore(id);
      setConfirmId(null);
      setError(null);
      await load();
    } catch (err) {
      setError(err as ApiError);
    }
  }

  if (loading && backups.length === 0) {
    return <div className="page__placeholder">Loading backups…</div>;
  }

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">Backups</h1>
      </header>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      {backups.length === 0 ? (
        <div className="page__placeholder">No backups yet.</div>
      ) : (
        <table className="projects__table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Change Set</th>
              <th>Manifest</th>
              <th>Created</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {backups.map((b) => (
              <tr key={b.id}>
                <td className="changes__id">{b.id.slice(0, 8)}…</td>
                <td className="changes__id">{b.changeSetId.slice(0, 8)}…</td>
                <td className="projects__path">{b.manifestPath}</td>
                <td>{b.createdAt.slice(0, 19).replace('T', ' ')}</td>
                <td>
                  {confirmId === b.id ? (
                    <div className="backup__confirm">
                      <span>Are you sure?</span>
                      <button
                        type="button"
                        className="projects__remove"
                        onClick={() => handleRestore(b.id)}
                      >
                        Restore
                      </button>
                      <button
                        type="button"
                        className="projects__remove"
                        onClick={() => setConfirmId(null)}
                      >
                        Cancel
                      </button>
                    </div>
                  ) : (
                    <button
                      type="button"
                      className="projects__remove"
                      onClick={() => setConfirmId(b.id)}
                    >
                      Restore
                    </button>
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
