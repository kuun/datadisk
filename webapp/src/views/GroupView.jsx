import React from 'react'
import Split from 'react-split'
import GroupManager from '../components/GroupManager'
import GroupUserManager from '../components/GroupUserManager'
import './GroupView.css'

const GroupView = () => (
  <div className="group-container">
    <Split className="split" sizes={[20, 80]} minSize={200} gutterSize={6}>
      <aside className="group-pane">
        <h3 className="group-header">群组</h3>
        <GroupManager />
      </aside>
      <section className="group-pane">
        <h3 className="group-header">用户</h3>
        <GroupUserManager />
      </section>
    </Split>
  </div>
)

export default GroupView
