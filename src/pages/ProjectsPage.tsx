import { FormEvent, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';

import { ApiError, api } from '../lib/api';
import type { Project } from '../types/domain';

export default function ProjectsPage() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [path, setPath] = useState('');
  const [name, setName] = useState('');
  const [submitting, setSubmitting] = useState(false);

  async function refresh() {
    setLoading(true);
    try {
      const list = await api.projects.list();
      setProjects(list);
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const trimmed = path.trim();
    if (!trimmed) return;
    setSubmitting(true);
    try {
      await api.projects.add(trimmed, name.trim() || undefined);
      setPath('');
      setName('');
      setError(null);
      await refresh();
    } catch (err) {
      setError(err as ApiError);
    } finally {
      setSubmitting(false);
    }
  }

  async function onRemove(id: string) {
    try {
      await api.projects.remove(id);
      await refresh();
    } catch (err) {
      setError(err as ApiError);
    }
  }

  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">Projects</h1>
        <p className="page__subtitle">Local directories managed by AgentHub.</p>
      </header>

      <form className="projects__form" onSubmit={onSubmit}>
        <label className="projects__field">
          <span>Path</span>
          <input
            type="text"
            value={path}
            onChange={(e) => setPath(e.target.value)}
            placeholder="/absolute/path/to/project"
            required
          />
        </label>
        <label className="projects__field">
          <span>Name (optional)</span>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Defaults to directory name"
          />
        </label>
        <button type="submit" disabled={submitting}>
          {submitting ? 'Adding…' : 'Add project'}
        </button>
      </form>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      {loading ? (
        <div className="page__placeholder">Loading…</div>
      ) : projects.length === 0 ? (
        <div className="page__placeholder">No projects yet. Add one above.</div>
      ) : (
        <table className="projects__table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Path</th>
              <th>Added</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {projects.map((p) => (
              <tr key={p.id}>
                <td>
                  <Link to={`/projects/${p.id}`}>{p.name}</Link>
                </td>
                <td className="projects__path">{p.path}</td>
                <td>{p.createdAt.slice(0, 19).replace('T', ' ')}</td>
                <td>
                  <button
                    type="button"
                    className="projects__remove"
                    onClick={() => onRemove(p.id)}
                  >
                    Remove
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
