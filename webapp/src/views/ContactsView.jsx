import React, { useEffect, useState } from 'react'
import Split from 'react-split'
import DepartmentManager from '../components/DepartmentManager'
import RoleManager from '../components/RoleManager'
import UserManager from '../components/UserManager'
import http from '../lib/http'
import { useLogin } from '../store/providers'
import './ContactsView.css'

const ContactsView = () => {
  const [roles, setRoles] = useState([])
  const { canContacts, canRole } = useLogin()

  useEffect(() => {
    if (!canContacts) return
    http
      .get('/api/role/list')
      .then((resp) => {
        if (resp.data.success) {
          setRoles(resp.data.data || [])
        }
      })
      .catch(() => {})
  }, [canContacts])

  if (!canContacts && !canRole) {
    return (
      <div className="contacts-container">
        <div className="contacts-pane">
          <h3 className="contacts-header">权限不足</h3>
        </div>
      </div>
    )
  }

  if (!canContacts && canRole) {
    return (
      <div className="contacts-container">
        <section className="contacts-pane">
          <h3 className="contacts-header">角色</h3>
          <RoleManager onRolesChange={setRoles} />
        </section>
      </div>
    )
  }

  return (
    <div className="contacts-container">
      <Split className="split" sizes={[20, 80]} minSize={200} gutterSize={6}>
        <aside className="contacts-pane">
          {canRole ? (
            <Split
              className="split-vertical"
              direction="vertical"
              sizes={[60, 40]}
              minSize={100}
              gutterSize={6}
            >
              <div className="contacts-section">
                <h3 className="contacts-header">组织结构</h3>
                <DepartmentManager />
              </div>
              <div className="contacts-section">
                <h3 className="contacts-header">角色</h3>
                <RoleManager onRolesChange={setRoles} />
              </div>
            </Split>
          ) : (
            <div className="contacts-section">
              <h3 className="contacts-header">组织结构</h3>
              <DepartmentManager />
            </div>
          )}
        </aside>
        <section className="contacts-pane">
          <h3 className="contacts-header">用户</h3>
          <UserManager roles={roles} />
        </section>
      </Split>
    </div>
  )
}

export default ContactsView
