// Thin wrapper around Tauri's `invoke` that normalizes errors into the
// shared CommandError shape exposed by the Rust command surface.

import { invoke } from '@tauri-apps/api/core';

import type {
  Agent,
  AgentKind,
  Backup,
  ChangeSet,
  DoctorIssue,
  Project,
  PromptTemplate,
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
  },
  doctor: {
    listIssues: () => call<DoctorIssue[]>('doctor_list_issues'),
  },
  changes: {
    list: () => call<ChangeSet[]>('changes_list'),
  },
  backups: {
    list: () => call<Backup[]>('backups_list'),
  },
  prompts: {
    list: () => call<PromptTemplate[]>('prompts_list'),
  },
};
