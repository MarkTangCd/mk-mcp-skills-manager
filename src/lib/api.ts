// Thin wrapper around Tauri's `invoke` that normalizes errors into the
// shared CommandError shape exposed by the Rust command surface.

import { invoke } from '@tauri-apps/api/core';

import type {
  Agent,
  Backup,
  ChangeSet,
  DoctorIssue,
  Project,
  PromptTemplate,
  ScanSnapshot,
} from '../types/domain';

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
