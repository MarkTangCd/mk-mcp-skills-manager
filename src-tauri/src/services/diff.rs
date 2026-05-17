// DiffService: utilities for formatting and truncating unified diff output
// so that the frontend can render before/after previews safely.

use crate::domain::{ChangePlan, FilePatch};

/// Maximum lines to show for a single patch before truncation.
const MAX_PATCH_LINES: usize = 500;
/// Maximum total lines across all patches before the preview is marked truncated.
const MAX_TOTAL_LINES: usize = 2_000;

#[derive(Debug, Clone)]
pub struct DiffPreview {
    pub patches: Vec<PatchPreview>,
    pub summary: DiffPreviewSummary,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct PatchPreview {
    pub path: String,
    pub diff_text: String,
    pub truncated: bool,
    pub total_lines: usize,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone)]
pub struct DiffPreviewSummary {
    pub files_changed: usize,
    pub total_additions: usize,
    pub total_deletions: usize,
}

pub struct DiffService;

impl DiffService {
    /// Build a `DiffPreview` from a `ChangePlan`, applying line-count
    /// truncation per patch and across the whole plan.
    pub fn preview(plan: &ChangePlan) -> DiffPreview {
        let mut patches = Vec::with_capacity(plan.patches.len());
        let mut total_additions = 0usize;
        let mut total_deletions = 0usize;
        let mut total_lines = 0usize;

        for patch in &plan.patches {
            let preview = Self::preview_patch(patch);
            total_additions += preview.additions;
            total_deletions += preview.deletions;
            total_lines += preview.total_lines;
            patches.push(preview);
        }

        let truncated = total_lines > MAX_TOTAL_LINES || patches.iter().any(|p| p.truncated);

        DiffPreview {
            patches,
            summary: DiffPreviewSummary {
                files_changed: plan.patches.len(),
                total_additions,
                total_deletions,
            },
            truncated,
        }
    }

    fn preview_patch(patch: &FilePatch) -> PatchPreview {
        let lines: Vec<&str> = patch.diff.lines().collect();
        let additions = lines.iter().filter(|l| l.starts_with('+')).count();
        let deletions = lines.iter().filter(|l| l.starts_with('-')).count();
        let total_lines = lines.len();
        let truncated = total_lines > MAX_PATCH_LINES;

        let diff_text = if truncated {
            let visible = &lines[..MAX_PATCH_LINES];
            let mut text = visible.join("\n");
            text.push_str("\n\n... (truncated)");
            text
        } else {
            patch.diff.clone()
        };

        PatchPreview {
            path: patch.path.clone(),
            diff_text,
            truncated,
            total_lines,
            additions,
            deletions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DiffSummary, FilePatch};

    fn dummy_plan_with_patches(patches: Vec<FilePatch>) -> ChangePlan {
        ChangePlan {
            id: "p1".into(),
            intent_id: "i1".into(),
            status: crate::domain::ChangeStatus::Draft,
            agent_kind: None,
            target_files: patches.iter().map(|p| p.path.clone()).collect(),
            operations: vec![],
            patches,
            diff_summary: DiffSummary {
                files_changed: 0,
                additions: 0,
                deletions: 0,
            },
            risks: vec![],
            validation_errors: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn preview_counts_additions_and_deletions() {
        let patch = FilePatch {
            path: "a.txt".into(),
            before_hash: None,
            after_hash: None,
            diff: "- old line\n+ new line\n context".into(),
        };
        let plan = dummy_plan_with_patches(vec![patch]);
        let preview = DiffService::preview(&plan);
        assert_eq!(preview.patches.len(), 1);
        assert_eq!(preview.patches[0].additions, 1);
        assert_eq!(preview.patches[0].deletions, 1);
        assert_eq!(preview.summary.total_additions, 1);
        assert_eq!(preview.summary.total_deletions, 1);
    }

    #[test]
    fn large_patch_is_truncated() {
        let big_diff = (0..MAX_PATCH_LINES + 10)
            .map(|i| format!("+ line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let patch = FilePatch {
            path: "big.txt".into(),
            before_hash: None,
            after_hash: None,
            diff: big_diff,
        };
        let plan = dummy_plan_with_patches(vec![patch]);
        let preview = DiffService::preview(&plan);
        assert!(preview.patches[0].truncated);
        assert!(preview.truncated);
        assert!(preview.patches[0].diff_text.ends_with("... (truncated)"));
    }

    #[test]
    fn empty_plan_preview_is_empty() {
        let plan = dummy_plan_with_patches(vec![]);
        let preview = DiffService::preview(&plan);
        assert!(preview.patches.is_empty());
        assert_eq!(preview.summary.files_changed, 0);
        assert!(!preview.truncated);
    }
}
