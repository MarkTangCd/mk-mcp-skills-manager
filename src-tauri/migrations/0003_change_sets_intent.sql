-- Enhance change_sets with intent tracking, risks and validation errors.
-- Keeps the existing columns intact and adds the metadata needed for
-- ChangePlan -> ChangeSet round-trips.

ALTER TABLE change_sets ADD COLUMN intent_json TEXT;
ALTER TABLE change_sets ADD COLUMN target_files_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE change_sets ADD COLUMN risks_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE change_sets ADD COLUMN validation_errors_json TEXT NOT NULL DEFAULT '[]';

-- Ensure we can look up change sets by intent and status efficiently.
CREATE INDEX IF NOT EXISTS idx_change_sets_intent ON change_sets (intent_json, status);
