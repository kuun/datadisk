import React from 'react'
import { Navigate, Route, Routes } from 'react-router-dom'
import HomeView from './views/HomeView'
import LoginView from './views/LoginView'
import SettingsView from './views/SettingsView'
import UserSettings from './views/settings/UserSettings'
import RoleSettings from './views/settings/RoleSettings'
import FileView from './views/FileView'
import RecentFileView from './views/file/RecentFileView'
import MyDocsView from './views/file/MyDocsView'
import FileStarView from './views/file/FileStarView'
import FileTagsView from './views/file/FileTagsView'
import ContactsView from './views/ContactsView'
import AuditView from './views/AuditView'
import GroupView from './views/GroupView'
import GlobalUploader from './components/uploader/GlobalUploader'
import DefaultRedirect from './components/DefaultRedirect'

const App = () => (
  <>
    <Routes>
      <Route path="/ui/login" element={<LoginView />} />
      <Route path="/" element={<HomeView />}>
        <Route index element={<DefaultRedirect />} />
        <Route path="ui/file" element={<FileView />}>
          <Route index element={<Navigate to="recent" replace />} />
          <Route path="recent" element={<RecentFileView />} />
          <Route path="mydocs" element={<MyDocsView />} />
          <Route path="star" element={<FileStarView />} />
          <Route path="tags" element={<FileTagsView />} />
        </Route>
        <Route path="ui/contacts" element={<ContactsView />} />
        <Route path="ui/audit" element={<AuditView />} />
        <Route path="ui/group" element={<GroupView />} />
        <Route path="ui/settings" element={<SettingsView />}>
          <Route index element={<Navigate to="user" replace />} />
          <Route path="user" element={<UserSettings />} />
          <Route path="roles" element={<RoleSettings />} />
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/ui/login" replace />} />
    </Routes>
    <GlobalUploader />
  </>
)

export default App
