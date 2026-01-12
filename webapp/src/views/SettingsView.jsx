import React from 'react'
import { NavLink, Outlet } from 'react-router-dom'
import { useLogin } from '../store/providers'
import './SettingsView.css'

const SettingsView = () => {
  const { canRole } = useLogin()

  return (
    <div className="settings-layout">
      <aside className="settings-aside">
        <div className="settings-aside-header">
          <div className="settings-aside-title">设置中心</div>
          <div className="settings-aside-subtitle">账户与偏好</div>
        </div>
        <nav className="settings-menu">
          <div className="settings-group">
            <div className="settings-group-title">账户</div>
            <NavLink to="/ui/settings/user" className={({ isActive }) => (isActive ? 'active' : '')}>
              用户设置
            </NavLink>
          </div>
          {canRole && (
            <div className="settings-group">
              <div className="settings-group-title">权限</div>
              <NavLink to="/ui/settings/roles" className={({ isActive }) => (isActive ? 'active' : '')}>
                角色管理
              </NavLink>
            </div>
          )}
        </nav>
      </aside>
      <main className="settings-main">
        <div className="settings-page">
          <div className="settings-header">
            <div>
              <div className="settings-title">设置</div>
              <div className="settings-subtitle">管理账户信息与常用偏好</div>
            </div>
          </div>
          <div className="settings-card">
            <Outlet />
          </div>
        </div>
      </main>
    </div>
  )
}

export default SettingsView
