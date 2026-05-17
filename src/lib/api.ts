// Thin wrapper around Tauri's `invoke` that normalizes errors into the
// shared CommandError shape exposed by the Rust command surface.

import { invoke } from '@tauri-apps/api/core';

import type {
  Agent,
  AgentKind,
  Backup,
  ChangeIntent,
  ChangePlan,
  ChangeSet,
  ChangeStatus,
  DoctorIssue,
  IssueSeverity,
  Project,
  ProjectMatrix,
  PromptTemplate,
  ResourceRecord,
  ResourceType,
  ScanSnapshot,
} from '../types/domain';

export interface AdapterErrorEntry {
  agentKind: AgentKind;
  message: string;
}

export interface ProjectScanReport {
  snapshots: ScanSnapshot[];
  adapterErrors: AdapterErrorEntry[];
}

export interface CommandError {
  code: string;
  message: string;
  target?: string;
  recoverable: boolean;
  details?: unknown;
}

export interface BootstrapInfo {
  dataDir: string;
  databasePath: string;
  schemaVersion: number;
}

export interface DashboardSnapshot {
  agents: Agent[];
  recentScans: ScanSnapshot[];
  openIssues: DoctorIssue[];
  recentChanges: ChangeSet[];
  bootstrap: BootstrapInfo;
}

export class ApiError extends Error {
  readonly code: string;
  readonly target?: string;
  readonly recoverable: boolean;
  readonly details?: unknown;

  constructor(error: CommandError) {
    super(error.message);
    this.name = 'ApiError';
    this.code = error.code;
    this.target = error.target;
    this.recoverable = error.recoverable;
    this.details = error.details;
  }
}

function normalizeError(raw: unknown): ApiError {
  if (raw && typeof raw === 'object' && 'code' in raw && 'message' in raw) {
    return new ApiError(raw as CommandError);
  }
  return new ApiError({
    code: 'unknown',
    message: typeof raw === 'string' ? raw : 'Unknown error',
    recoverable: true,
  });
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw) {
    throw normalizeError(raw);
  }
}

export const api = {
  app: {
    getDashboard: () => call<DashboardSnapshot>('app_get_dashboard'),
  },
  agents: {
    list: () => call<Agent[]>('agents_list'),
  },
  projects: {
    list: () => call<Project[]>('projects_list'),
    add: (path: string, name?: string) =>
      call<Project>('projects_add', { path, name: name ?? null }),
    get: (id: string) => call<Project>('projects_get', { id }),
    remove: (id: string) => call<void>('projects_remove', { id }),
    rescan: (id: string) => call<ProjectScanReport>('projects_rescan', { id }),
    latestScans: (id: string) => call<ScanSnapshot[]>('projects_latest_scans', { id }),
    getMatrix: (id: string) => call<ProjectMatrix>('projects_get_matrix', { id }),
  },
  resources: {
    list: (resourceType?: ResourceType) =>
      call<ResourceRecord[]>('resources_list', { resourceType: resourceType ?? null }),
  },
  doctor: {
    listIssues: (severity?: IssueSeverity, category?: string, projectId?: string) =>
      call<DoctorIssue[]>('doctor_list_issues', {
        severity: severity ?? null,
        category: category ?? null,
        projectId: projectId ?? null,
      }),
    run: (projectId?: string) =>
      call<{ issues: DoctorIssue[]; summary: { total: number; critical: number; warning: number; info: number } }>('doctor_run', { projectId: projectId ?? null }),
    runAll: () =>
      call<{ issues: DoctorIssue[]; summary: { total: number; critical: number; warning: number; info: number } }>('doctor_run_all'),
    issueSummary: () =>
      call<{ total: number; critical: number; warning: number; info: number }>('doctor_issue_summary'),
  },
  changes: {
    list: () => call<ChangeSet[]>('changes_list'),
    getPlan: (id: string) => call<ChangePlan>('changes_get_plan', { id }),
    transition: (id: string, status: ChangeStatus) =>
      call<ChangePlan>('changes_transition', { id, status }),
    createChangePlan: (intent: ChangeIntent) =>
      call<ChangePlan>('changes_create_plan', { intent }),
    applyPlan: (planId: string) => call<ChangePlan>('changes_apply_plan', { planId }),
  },
  backups: {
    list: () => call<Backup[]>('backups_list'),
    restore: (id: string) => call<Backup>('backups_restore', { id }),
  },
  prompts: {
    list: () => call<PromptTemplate[]>('prompts_list'),
  },
};
