import { NavLink } from 'react-router-dom';

const NAV_ITEMS = [
  { to: '/dashboard', label: 'Dashboard' },
  { to: '/projects', label: 'Projects' },
  { to: '/mcp-servers', label: 'MCP Servers' },
  { to: '/skills', label: 'Skills' },
  { to: '/sub-agents', label: 'Sub-agents' },
  { to: '/pi-resources', label: 'Pi Resources' },
  { to: '/prompts', label: 'Prompts' },
  { to: '/doctor', label: 'Doctor' },
  { to: '/changes', label: 'Changes' },
  { to: '/backups', label: 'Backups' },
  { to: '/settings', label: 'Settings' },
] as const;

export default function Sidebar() {
  return (
    <aside className="sidebar">
      <div className="sidebar__brand">AgentHub Local</div>
      <nav className="sidebar__nav">
        {NAV_ITEMS.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              isActive ? 'sidebar__link sidebar__link--active' : 'sidebar__link'
            }
          >
            {item.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
