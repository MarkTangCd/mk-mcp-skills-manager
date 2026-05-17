import { useState } from 'react';
import type { ChangePlan } from '../types/domain';

interface DiffPreviewProps {
  plan: ChangePlan;
  onConfirm?: () => void;
  onCancel?: () => void;
}

export default function DiffPreview({ plan, onConfirm, onCancel }: DiffPreviewProps) {
  const [showConfirm, setShowConfirm] = useState(false);
  const hasRisks = plan.risks.length > 0;
  const isPreviewed = plan.status === 'previewed';
  const isConfirmed = plan.status === 'confirmed';
  const canConfirm = isPreviewed && plan.validationErrors.length === 0;
  const canApply = isConfirmed && plan.validationErrors.length === 0;
  const actionLabel = isPreviewed ? 'Confirm' : 'Apply';
  const canAction = canConfirm || canApply;

  return (
    <div className="diff-preview">
      <header className="diff-preview__header">
        <h3 className="diff-preview__title">Change Plan</h3>
        <span className={`diff-preview__status diff-preview__status--${plan.status}`}>
          {plan.status}
        </span>
      </header>

      {plan.targetFiles.length > 0 && (
        <div className="diff-preview__files">
          <strong>Target files:</strong>
          <ul>
            {plan.targetFiles.map((f) => (
              <li key={f} className="diff-preview__file">
                {f}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="diff-preview__summary">
        <span>{plan.diffSummary.filesChanged} file(s) changed</span>
        <span className="diff-preview__additions">+{plan.diffSummary.additions}</span>
        <span className="diff-preview__deletions">-{plan.diffSummary.deletions}</span>
      </div>

      {hasRisks && (
        <div className="diff-preview__risks">
          <strong>Risks:</strong>
          <ul>
            {plan.risks.map((risk, idx) => (
              <li key={idx} className="diff-preview__risk">
                {risk}
              </li>
            ))}
          </ul>
        </div>
      )}

      {plan.validationErrors.length > 0 && (
        <div className="diff-preview__errors">
          <strong>Validation errors:</strong>
          <ul>
            {plan.validationErrors.map((err, idx) => (
              <li key={idx}>{err}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="diff-preview__patches">
        {plan.patches.map((patch) => (
          <details key={patch.path} className="diff-preview__patch" open>
            <summary className="diff-preview__patch-title">{patch.path}</summary>
            <pre className="diff-preview__diff">
              <code>{patch.diff || 'No diff available.'}</code>
            </pre>
          </details>
        ))}
        {plan.patches.length === 0 && (
          <div className="page__placeholder">No file patches in this plan.</div>
        )}
      </div>

      <div className="diff-preview__actions">
        {onCancel && (
          <button type="button" className="diff-preview__btn" onClick={onCancel}>
            Cancel
          </button>
        )}
        {onConfirm && (
          <button
            type="button"
            className="diff-preview__btn diff-preview__btn--primary"
            disabled={!canAction}
            onClick={() => {
              if (hasRisks && !showConfirm) {
                setShowConfirm(true);
                return;
              }
              onConfirm();
            }}
          >
            {showConfirm ? 'Confirm Apply' : actionLabel}
          </button>
        )}
      </div>

      {showConfirm && hasRisks && (
        <div className="diff-preview__confirm" role="alert">
          <strong>Dangerous change detected.</strong> Please review the risks above before
          confirming.
        </div>
      )}

      {!canApply && plan.status !== 'confirmed' && (
        <div className="diff-preview__hint">
          This plan must be confirmed before it can be applied.
        </div>
      )}
    </div>
  );
}
