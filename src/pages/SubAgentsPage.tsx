import ResourceListPage from '../components/ResourceListPage';

export default function SubAgentsPage() {
  return (
    <ResourceListPage
      title="Sub-agents"
      subtitle="Read-only index of Claude Code and Codex sub-agents."
      resourceType="sub-agent"
    />
  );
}
