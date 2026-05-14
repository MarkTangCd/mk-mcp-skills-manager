import { useLocation } from 'react-router-dom';

const TITLES: Record<string, string> = {
  '/dashboard': 'Dashboard',
  '/projects': 'Projects',
  '/mcp-servers': 'MCP Servers',
  '/skills': 'Skills',
  '/sub-agents': 'Sub-agents',
  '/pi-resources': 'Pi Resources',
  '/prompts': 'Prompts',
  '/doctor': 'Doctor',
  '/backups': 'Backups',
  '/settings': 'Settings',
};

export default function TopBar() {
  const { pathname } = useLocation();
  const title = TITLES[pathname] ?? 'AgentHub Local';
  return (
    <header className="topbar">
      <div className="topbar__title">{title}</div>
      <div className="topbar__status">
        <span className="topbar__status-dot" />
        <span>Local</span>
      </div>
    </header>
  );
}
