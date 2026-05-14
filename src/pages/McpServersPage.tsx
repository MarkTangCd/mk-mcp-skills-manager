import ResourceListPage from '../components/ResourceListPage';

export default function McpServersPage() {
  return (
    <ResourceListPage
      title="MCP Servers"
      subtitle="Read-only index of configured MCP servers."
      resourceType="mcp"
    />
  );
}
