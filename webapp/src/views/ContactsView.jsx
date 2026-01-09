import React, { useState } from 'react'
import Split from 'react-split'
import DepartmentManager from '../components/DepartmentManager'
import RoleManager from '../components/RoleManager'
import UserManager from '../components/UserManager'
import './ContactsView.css'

const ContactsView = () => {
  const [roles, setRoles] = useState([])

  return (
    <div className="contacts-container">
      <Split className="split" sizes={[20, 80]} minSize={200} gutterSize={6}>
        <aside className="contacts-pane">
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
