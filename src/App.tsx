import { Navigate, Route, Routes } from 'react-router-dom';
import AppLayout from './layout/AppLayout';
import BackupsPage from './pages/BackupsPage';
import ChangesPage from './pages/ChangesPage';
import DashboardPage from './pages/DashboardPage';
import DoctorPage from './pages/DoctorPage';
import McpServersPage from './pages/McpServersPage';
import PiResourcesPage from './pages/PiResourcesPage';
import ProjectDetailPage from './pages/ProjectDetailPage';
import ProjectsPage from './pages/ProjectsPage';
import PromptsPage from './pages/PromptsPage';
import SettingsPage from './pages/SettingsPage';
import SkillsPage from './pages/SkillsPage';
import SubAgentsPage from './pages/SubAgentsPage';

export default function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/projects" element={<ProjectsPage />} />
        <Route path="/projects/:id" element={<ProjectDetailPage />} />
        <Route path="/mcp-servers" element={<McpServersPage />} />
        <Route path="/skills" element={<SkillsPage />} />
        <Route path="/sub-agents" element={<SubAgentsPage />} />
        <Route path="/pi-resources" element={<PiResourcesPage />} />
        <Route path="/prompts" element={<PromptsPage />} />
        <Route path="/doctor" element={<DoctorPage />} />
        <Route path="/changes" element={<ChangesPage />} />
        <Route path="/backups" element={<BackupsPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Route>
    </Routes>
  );
}
