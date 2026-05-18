import { useCallback, useEffect, useMemo, useState } from 'react';
import type React from 'react';

import { ApiError, api } from '../lib/api';
import type { PromptTemplate } from '../types/domain';

const EMPTY_PROMPT: PromptTemplate = {
  id: '',
  slug: '',
  title: '',
  category: '',
  body: 'Review {{input}} and return concise findings.',
  variables: ['input'],
  tags: [],
  favorite: false,
};

export default function PromptsPage() {
  const [prompts, setPrompts] = useState<PromptTemplate[]>([]);
  const [selectedSlug, setSelectedSlug] = useState<string | null>(null);
  const [draft, setDraft] = useState<PromptTemplate>(EMPTY_PROMPT);
  const [search, setSearch] = useState('');
  const [tagText, setTagText] = useState('');
  const [values, setValues] = useState<Record<string, string>>({});
  const [rendered, setRendered] = useState('');
  const [missingVariables, setMissingVariables] = useState<string[]>([]);
  const [error, setError] = useState<ApiError | null>(null);
  const [copyStatus, setCopyStatus] = useState('');

  const load = useCallback(async () => {
    try {
      const list = await api.prompts.list(search || undefined);
      setPrompts(list);
      setError(null);
      const firstPrompt = list[0];
      if (!selectedSlug && firstPrompt) {
        setSelectedSlug(firstPrompt.slug);
        setDraft(firstPrompt);
        setTagText(firstPrompt.tags.join(', '));
      }
    } catch (err) {
      setError(err as ApiError);
    }
  }, [search, selectedSlug]);

  useEffect(() => {
    void load();
  }, [load]);

  const variables = useMemo(() => extractVariables(draft.body), [draft.body]);

  useEffect(() => {
    setDraft((current) => ({ ...current, variables }));
  }, [variables]);

  useEffect(() => {
    const content = renderPrompt(draft.body, values);
    setRendered(content.content);
    setMissingVariables(content.missing);
  }, [draft.body, values]);

  function selectPrompt(prompt: PromptTemplate) {
    setSelectedSlug(prompt.slug);
    setDraft(prompt);
    setTagText(prompt.tags.join(', '));
    setValues({});
    setCopyStatus('');
  }

  function startNewPrompt() {
    setSelectedSlug(null);
    setDraft({ ...EMPTY_PROMPT, id: '', slug: '', variables: extractVariables(EMPTY_PROMPT.body) });
    setTagText('');
    setValues({});
    setCopyStatus('');
  }

  async function handleSave(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const tags = tagText
      .split(',')
      .map((tag) => tag.trim())
      .filter(Boolean);
    try {
      const saved = selectedSlug
        ? await api.prompts.update(
            selectedSlug,
            draft.title,
            draft.category ?? '',
            tags,
            draft.favorite,
            draft.body,
          )
        : await api.prompts.create(
            draft.slug,
            draft.title,
            draft.category ?? '',
            tags,
            draft.favorite,
            draft.body,
          );
      setSelectedSlug(saved.slug);
      setDraft(saved);
      setTagText(saved.tags.join(', '));
      await load();
      setError(null);
    } catch (err) {
      setError(err as ApiError);
    }
  }

  async function handleDelete() {
    if (!selectedSlug) return;
    try {
      await api.prompts.delete(selectedSlug);
      setSelectedSlug(null);
      setDraft(EMPTY_PROMPT);
      setTagText('');
      setValues({});
      await load();
    } catch (err) {
      setError(err as ApiError);
    }
  }

  async function handleCopy() {
    const next = renderPrompt(draft.body, values);
    setRendered(next.content);
    setMissingVariables(next.missing);
    if (next.missing.length > 0) {
      setCopyStatus('Fill missing variables before copying.');
      return;
    }
    await navigator.clipboard.writeText(next.content);
    setCopyStatus('Copied rendered prompt.');
  }

  return (
    <div className="page page--wide">
      <header className="page__header">
        <h1 className="page__title">Prompts</h1>
        <p className="page__subtitle">Reusable prompt templates with variables and copy preview.</p>
      </header>

      {error && (
        <div className="dashboard__error" role="alert">
          [{error.code}] {error.message}
        </div>
      )}

      <div className="prompts__layout">
        <aside className="prompts__list">
          <div className="prompts__toolbar">
            <input
              type="search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search prompts"
            />
            <button type="button" onClick={startNewPrompt}>
              New
            </button>
          </div>
          {prompts.length === 0 ? (
            <div className="page__placeholder">No prompts yet.</div>
          ) : (
            prompts.map((prompt) => (
              <button
                key={prompt.slug}
                type="button"
                className={
                  selectedSlug === prompt.slug ? 'prompts__item prompts__item--active' : 'prompts__item'
                }
                onClick={() => selectPrompt(prompt)}
              >
                <span>{prompt.favorite ? '* ' : ''}{prompt.title}</span>
                <small>{prompt.category ?? 'uncategorized'} · {prompt.variables.length} vars</small>
              </button>
            ))
          )}
        </aside>

        <main className="prompts__editor">
          <form className="prompts__form" onSubmit={handleSave}>
            <div className="prompts__form-grid">
              <label className="projects__field">
                <span>Slug</span>
                <input
                  value={draft.slug}
                  disabled={Boolean(selectedSlug)}
                  onChange={(event) => setDraft({ ...draft, slug: event.target.value })}
                  placeholder="review-pr"
                />
              </label>
              <label className="projects__field">
                <span>Title</span>
                <input
                  value={draft.title}
                  onChange={(event) => setDraft({ ...draft, title: event.target.value })}
                  placeholder="Review PR"
                />
              </label>
              <label className="projects__field">
                <span>Category</span>
                <input
                  value={draft.category ?? ''}
                  onChange={(event) => setDraft({ ...draft, category: event.target.value })}
                  placeholder="review"
                />
              </label>
              <label className="prompts__favorite">
                <input
                  type="checkbox"
                  checked={draft.favorite}
                  onChange={(event) => setDraft({ ...draft, favorite: event.target.checked })}
                />
                <span>Favorite</span>
              </label>
            </div>

            <label className="projects__field">
              <span>Tags</span>
              <input
                value={tagText}
                onChange={(event) => setTagText(event.target.value)}
                placeholder="review, quality"
              />
            </label>

            <label className="projects__field">
              <span>Template</span>
              <textarea
                className="prompts__textarea"
                value={draft.body}
                onChange={(event) => setDraft({ ...draft, body: event.target.value })}
              />
            </label>

            <div className="prompts__actions">
              {selectedSlug && (
                <button type="button" className="projects__remove" onClick={handleDelete}>
                  Delete
                </button>
              )}
              <button type="submit" disabled={!draft.slug.trim() || !draft.title.trim()}>
                Save
              </button>
            </div>
          </form>

          <section className="prompts__preview">
            <h2 className="dashboard__section-title">Variables</h2>
            {variables.length === 0 ? (
              <div className="page__placeholder">This prompt has no variables.</div>
            ) : (
              <div className="prompts__variables">
                {variables.map((variable) => (
                  <label key={variable} className="projects__field">
                    <span>{variable}</span>
                    <input
                      value={values[variable] ?? ''}
                      onChange={(event) =>
                        setValues({ ...values, [variable]: event.target.value })
                      }
                    />
                  </label>
                ))}
              </div>
            )}

            {missingVariables.length > 0 && (
              <div className="prompts__missing">Missing: {missingVariables.join(', ')}</div>
            )}

            <h2 className="dashboard__section-title">Final Prompt</h2>
            <pre className="prompts__rendered">{rendered}</pre>
            <div className="prompts__actions">
              <button type="button" onClick={handleCopy}>
                Copy rendered prompt
              </button>
              {copyStatus && <span className="resource-list__meta">{copyStatus}</span>}
            </div>
          </section>
        </main>
      </div>
    </div>
  );
}

function extractVariables(body: string) {
  const variables: string[] = [];
  const regex = /\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}/g;
  for (const match of body.matchAll(regex)) {
    const name = match[1];
    if (name && !variables.includes(name)) variables.push(name);
  }
  return variables;
}

function renderPrompt(body: string, values: Record<string, string>) {
  const missing: string[] = [];
  const content = body.replace(/\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}/g, (_token, name: string | undefined) => {
    if (!name) return _token;
    const value = values[name];
    if (!value) {
      if (!missing.includes(name)) missing.push(name);
      return `{{${name}}}`;
    }
    return value;
  });
  return { content, missing };
}
