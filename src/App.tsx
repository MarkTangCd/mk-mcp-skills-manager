import { Navigate, Route, Routes } from 'react-router-dom';
import AppLayout from './layout/AppLayout';
import DashboardPage from './pages/DashboardPage';
import ProjectsPage from './pages/ProjectsPage';
import McpServersPage from './pages/McpServersPage';
import SkillsPage from './pages/SkillsPage';
import SubAgentsPage from './pages/SubAgentsPage';
import PiResourcesPage from './pages/PiResourcesPage';
import PromptsPage from './pages/PromptsPage';
import DoctorPage from './pages/DoctorPage';
import BackupsPage from './pages/BackupsPage';
import SettingsPage from './pages/SettingsPage';

export default function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/projects" element={<ProjectsPage />} />
        <Route path="/mcp-servers" element={<McpServersPage />} />
        <Route path="/skills" element={<SkillsPage />} />
        <Route path="/sub-agents" element={<SubAgentsPage />} />
        <Route path="/pi-resources" element={<PiResourcesPage />} />
        <Route path="/prompts" element={<PromptsPage />} />
        <Route path="/doctor" element={<DoctorPage />} />
        <Route path="/backups" element={<BackupsPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Route>
    </Routes>
  );
}
