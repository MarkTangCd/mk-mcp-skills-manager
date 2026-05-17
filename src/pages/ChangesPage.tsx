import { useCallback, useEffect, useState } from 'react';

import DiffPreview from '../components/DiffPreview';
import { ApiError, api } from '../lib/api';
import type { ChangePlan, ChangeSet } from '../types/domain';

export default function ChangesPage() {
  const [sets, setSets] = useState<ChangeSet[]>([]);
  const [selectedPlan, setSelectedPlan] = useState<ChangePlan | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.changes.list();
      setSets(list);
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

  async function handleSelect(id: string) {
    try {
      const plan = await api.changes.getPlan(id);
      setSelectedPlan(plan);
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    }
  }

  async function handleConfirm() {
    if (!selectedPlan) return;
    try {
      await api.changes.transition(selectedPlan.id, 'confirmed');
      const plan = await api.changes.getPlan(selectedPlan.id);
      setSelectedPlan(plan);
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    }
  }

  async function handleApply() {
    if (!selectedPlan) return;
    try {
      await api.changes.transition(selectedPlan.id, 'applied');
      const plan = await api.changes.getPlan(selectedPlan.id);
      setSelectedPlan(plan);
      setError(null);
      await load();
    } catch (err) {
      setError(err as ApiError);
    }
  }

  if (loading && sets.length === 0) {
    return <div className="page__placeholder">Loading change sets…</div>;
  }

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">Changes</h1>
      </header>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      <div className="changes__layout">
        <section className="changes__list">
          <h2 className="dashboard__section-title">Change Sets</h2>
          {sets.length === 0 ? (
            <div className="page__placeholder">No change sets yet.</div>
          ) : (
            <table className="projects__table">
              <thead>
                <tr>
                  <th>ID</th>
                  <th>Status</th>
                  <th>Files</th>
                  <th>Risks</th>
                  <th>Created</th>
                </tr>
              </thead>
              <tbody>
                {sets.map((cs) => (
                  <tr
                    key={cs.id}
                    className={
                      selectedPlan?.id === cs.id ? 'changes__row--active' : undefined
                    }
                    onClick={() => handleSelect(cs.id)}
                  >
                    <td className="changes__id">{cs.id.slice(0, 8)}…</td>
                    <td>
                      <span className={`changes__status changes__status--${cs.status}`}>
                        {cs.status}
                      </span>
                    </td>
                    <td>{cs.targetFiles.length}</td>
                    <td>{cs.risks.length}</td>
                    <td>{cs.createdAt.slice(0, 19).replace('T', ' ')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </section>

        <section className="changes__detail">
          {selectedPlan ? (
            <DiffPreview
              plan={selectedPlan}
              onConfirm={
                selectedPlan.status === 'previewed'
                  ? handleConfirm
                  : selectedPlan.status === 'confirmed'
                    ? handleApply
                    : undefined
              }
              onCancel={() => setSelectedPlan(null)}
            />
          ) : (
            <div className="page__placeholder">Select a change set to preview its diff.</div>
          )}
        </section>
      </div>
    </div>
  );
}
