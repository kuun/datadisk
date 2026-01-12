import React, { useEffect, useState } from 'react'
import Split from 'react-split'
import DepartmentManager from '../components/DepartmentManager'
import UserManager from '../components/UserManager'
import http from '../lib/http'
import { useLogin } from '../store/providers'
import './ContactsView.css'

const ContactsView = () => {
  const [roles, setRoles] = useState([])
  const { canContacts } = useLogin()

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

  if (!canContacts) {
    return (
      <div className="contacts-container">
        <div className="contacts-pane">
          <h3 className="contacts-header">权限不足</h3>
        </div>
      </div>
    )
  }

  return (
    <div className="contacts-container">
      <Split className="split" sizes={[20, 80]} minSize={200} gutterSize={6}>
        <aside className="contacts-pane">
          <div className="contacts-section">
            <h3 className="contacts-header">组织结构</h3>
            <DepartmentManager />
          </div>
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
