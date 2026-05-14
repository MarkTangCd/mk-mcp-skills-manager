// Mirror of the Rust domain model in src-tauri/src/domain.
// Keep these definitions in sync with the Rust structs and serde rename rules.

export type AgentKind = 'claude-code' | 'codex' | 'opencode' | 'pi';

export type ScopeType = 'global' | 'project';

export type ResourceType = 'mcp' | 'skill' | 'sub-agent' | 'pi-resource';

export type PiResourceKind =
  | 'skill'
  | 'prompt-template'
  | 'extension'
  | 'package'
  | 'theme'
  | 'setting';

export type HealthStatus = 'ok' | 'warning' | 'critical' | 'unknown';

export type IssueSeverity = 'info' | 'warning' | 'critical';

export type ChangeStatus =
  | 'draft'
  | 'previewed'
  | 'confirmed'
  | 'applied'
  | 'applied_with_warning'
  | 'failed'
  | 'restored';

export type McpTransport = 'stdio' | 'sse' | 'http';

export interface Agent {
  id: string;
  kind: AgentKind;
  displayName: string;
  installed: boolean;
  version: string | null;
  healthStatus: HealthStatus;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  createdAt: string;
  updatedAt: string;
}

export interface ConfigScope {
  agentKind: AgentKind;
  scopeType: ScopeType;
  projectId: string | null;
  configPath: string;
  writable: boolean;
}

export interface McpServer {
  id: string;
  name: string;
  transport: McpTransport;
  command: string | null;
  args: string[];
  url: string | null;
  envRefs: string[];
  enabled: boolean;
}

export interface Skill {
  id: string;
  slug: string;
  title: string;
  description: string | null;
  tags: string[];
  status: string;
  sourcePath: string | null;
}

export interface SubAgent {
  id: string;
  slug: string;
  role: string;
  agentKinds: AgentKind[];
  boundMcpIds: string[];
  boundSkillIds: string[];
}

export interface PiResource {
  id: string;
  resourceType: PiResourceKind;
  source: string;
  path: string | null;
  enabled: boolean;
  trusted: boolean;
}

export interface PromptTemplate {
  id: string;
  slug: string;
  title: string;
  body: string;
  variables: string[];
  tags: string[];
}

export interface ResourceBinding {
  resourceType: ResourceType;
  resourceId: string;
  agentKind: AgentKind;
  projectId: string | null;
  scopeType: ScopeType;
  enabled: boolean;
}

export interface ScanSummary {
  totalResources: number;
  mcpCount: number;
  skillCount: number;
  subAgentCount: number;
  piResourceCount: number;
  errors: string[];
}

export interface ScanSnapshot {
  id: string;
  projectId: string | null;
  agentKind: AgentKind | null;
  summary: ScanSummary;
  createdAt: string;
}

export interface DoctorTargetRef {
  resourceType: ResourceType | null;
  resourceId: string | null;
  agentKind: AgentKind | null;
  projectId: string | null;
  configPath: string | null;
}

export interface DoctorIssue {
  id: string;
  severity: IssueSeverity;
  category: string;
  message: string;
  targetRef: DoctorTargetRef | null;
  fixable: boolean;
}

export interface FilePatch {
  path: string;
  beforeHash: string | null;
  afterHash: string | null;
  diff: string;
}

export interface DiffSummary {
  filesChanged: number;
  additions: number;
  deletions: number;
}

export interface ChangeOperation {
  kind: string;
  target: string;
  payload: unknown;
}

export interface ChangeSet {
  id: string;
  status: ChangeStatus;
  operations: ChangeOperation[];
  patches: FilePatch[];
  diffSummary: DiffSummary;
  backupId: string | null;
  createdAt: string;
}

export interface Backup {
  id: string;
  changeSetId: string;
  manifestPath: string;
  createdAt: string;
}
