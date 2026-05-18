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

export type LibraryKind = 'skills' | 'sub-agents' | 'prompts' | 'mcp-templates';

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
  status?: string;
  configPath?: string | null;
}

export interface ResourceBindingRecord extends ResourceBinding {
  id: string;
  projectName: string | null;
  status: string;
  configPath: string | null;
  updatedAt: string;
}

export interface ResourceRecord {
  id: string;
  resourceType: ResourceType;
  name: string;
  slug: string | null;
  agentKind: AgentKind | null;
  sourcePath: string | null;
  status: string;
  payload: unknown;
  updatedAt: string;
  bindings: ResourceBindingRecord[];
}

export interface MatrixSource {
  resourceId: string;
  resourceName: string;
  scopeType: ScopeType;
  projectId: string | null;
  configPath: string | null;
  sourcePath: string | null;
  enabled: boolean;
  status: string;
}

export interface MatrixCell {
  agentKind: AgentKind;
  status: 'enabled' | 'disabled' | 'missing' | 'unknown';
  sources: MatrixSource[];
}

export interface MatrixRow {
  key: string;
  name: string;
  resourceType: ResourceType;
  cells: MatrixCell[];
}

export interface PiResourceKindSummary {
  resourceType: PiResourceKind;
  total: number;
  enabled: number;
  disabled: number;
  missing: number;
  untrusted: number;
}

export interface PiResourceSummary {
  total: number;
  enabled: number;
  disabled: number;
  missing: number;
  untrusted: number;
  byKind: PiResourceKindSummary[];
}

export interface ProjectMatrix {
  projectId: string;
  agents: AgentKind[];
  mcpMatrix: MatrixRow[];
  skillsMatrix: MatrixRow[];
  subAgentMatrix: MatrixRow[];
  piResourceSummary: PiResourceSummary;
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

export interface ChangeIntent {
  id: string;
  changeType: string;
  agentKind: AgentKind | null;
  projectId: string | null;
  scopeType: ScopeType | null;
  resourceId: string | null;
  payload: unknown;
  createdAt: string;
}

export interface ChangePlan {
  id: string;
  intentId: string;
  status: ChangeStatus;
  agentKind: AgentKind | null;
  targetFiles: string[];
  operations: ChangeOperation[];
  patches: FilePatch[];
  diffSummary: DiffSummary;
  risks: string[];
  validationErrors: string[];
  createdAt: string;
  updatedAt: string;
}

export interface ChangeSet {
  id: string;
  intentId: string | null;
  status: ChangeStatus;
  targetFiles: string[];
  operations: ChangeOperation[];
  patches: FilePatch[];
  diffSummary: DiffSummary;
  risks: string[];
  validationErrors: string[];
  backupId: string | null;
  projectId: string | null;
  agentKind: AgentKind | null;
  createdAt: string;
  updatedAt: string;
}

export interface Backup {
  id: string;
  changeSetId: string;
  manifestPath: string;
  createdAt: string;
}

export interface LibraryMetadata {
  slug: string;
  title: string;
  description: string | null;
  tags: string[];
  entryFile: string | null;
  role: string | null;
  agentKinds: AgentKind[];
  boundMcpIds: string[];
  boundSkillIds: string[];
  createdAt: string;
  updatedAt: string;
}

export interface SubAgentMetadata {
  slug: string;
  title: string;
  role: string;
  description: string;
  tags: string[];
  agentKinds: AgentKind[];
  boundMcpIds: string[];
  boundSkillIds: string[];
}

export interface LibraryEntry {
  kind: LibraryKind;
  slug: string;
  metadata: LibraryMetadata;
}

export interface LibraryEntryDetail {
  kind: LibraryKind;
  slug: string;
  metadata: LibraryMetadata;
  files: Record<string, string>;
}
