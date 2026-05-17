import { useCallback, useState } from 'react';

import { api } from '../lib/api';
import type { AgentKind, ChangeIntent, ChangePlan, McpServer, ResourceRecord, ScopeType } from '../types/domain';

type Mode = 'list' | 'form' | 'preview';

interface EditData {
  data: McpServer;
  agentKind: AgentKind;
  scopeType: ScopeType;
  projectId: string | null;
}

export function useMcpChangeFlow(load: () => Promise<void>) {
  const [mode, setMode] = useState<Mode>('list');
  const [editData, setEditData] = useState<EditData | undefined>(undefined);
  const [plan, setPlan] = useState<ChangePlan | null>(null);
  const [planProjectId, setPlanProjectId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<Error | null>(null);

  const handleAdd = useCallback(() => {
    setEditData(undefined);
    setMode('form');
    setActionError(null);
  }, []);

  const handleEdit = useCallback((resource: ResourceRecord) => {
    const payload = resource.payload as McpServer | undefined;
    if (!payload) return;
    const binding = resource.bindings[0];
    setEditData({
      data: payload,
      agentKind: binding?.agentKind ?? 'claude-code',
      scopeType: binding?.scopeType ?? 'global',
      projectId: binding?.projectId ?? null,
    });
    setMode('form');
    setActionError(null);
  }, []);

  const handleCreatePlan = useCallback(
    async (intent: ChangeIntent) => {
      setActionError(null);
      try {
        const created = await api.changes.createChangePlan(intent);
        const previewed = await api.changes.transition(created.id, 'previewed');
        setPlan(previewed);
        setPlanProjectId(intent.projectId ?? null);
        setMode('preview');
      } catch (err) {
        setActionError(err as Error);
      }
    },
    [],
  );

  const handleConfirmPlan = useCallback(async () => {
    if (!plan) return;
    setActionError(null);
    try {
      const confirmed = await api.changes.transition(plan.id, 'confirmed');
      setPlan(confirmed);
    } catch (err) {
      setActionError(err as Error);
    }
  }, [plan]);

  const handleApplyPlan = useCallback(async () => {
    if (!plan) return;
    setActionError(null);
    try {
      await api.changes.applyPlan(plan.id);
      setPlan(null);
      setMode('list');
      if (planProjectId) {
        try {
          await api.projects.rescan(planProjectId);
        } catch {
          // Best-effort rescan.
        }
      }
      setPlanProjectId(null);
      await load();
    } catch (err) {
      setActionError(err as Error);
    }
  }, [plan, planProjectId, load]);

  const handleToggle = useCallback(
    async (resource: ResourceRecord, enable: boolean) => {
      const payload = resource.payload as McpServer | undefined;
      if (!payload) return;
      const intent: ChangeIntent = {
        id: crypto.randomUUID(),
        changeType: enable ? 'enableMcp' : 'disableMcp',
        agentKind: resource.bindings[0]?.agentKind ?? 'claude-code',
        projectId: resource.bindings[0]?.projectId ?? null,
        scopeType: resource.bindings[0]?.scopeType ?? 'global',
        resourceId: resource.id,
        payload: { id: resource.id, enabled: enable },
        createdAt: new Date().toISOString(),
      };
      await handleCreatePlan(intent);
    },
    [handleCreatePlan],
  );

  const handleDelete = useCallback(
    async (resource: ResourceRecord) => {
      const intent: ChangeIntent = {
        id: crypto.randomUUID(),
        changeType: 'deleteMcp',
        agentKind: resource.bindings[0]?.agentKind ?? 'claude-code',
        projectId: resource.bindings[0]?.projectId ?? null,
        scopeType: resource.bindings[0]?.scopeType ?? 'global',
        resourceId: resource.id,
        payload: { id: resource.id },
        createdAt: new Date().toISOString(),
      };
      await handleCreatePlan(intent);
    },
    [handleCreatePlan],
  );

  const handleCancel = useCallback(() => {
    setPlan(null);
    setMode('list');
    setActionError(null);
  }, []);

  return {
    mode,
    editData,
    plan,
    actionError,
    handleAdd,
    handleEdit,
    handleCreatePlan,
    handleConfirmPlan,
    handleApplyPlan,
    handleToggle,
    handleDelete,
    handleCancel,
  };
}
