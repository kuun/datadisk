import React, { createContext, useContext, useMemo, useState } from 'react'
import http from '../lib/http'

const LoginContext = createContext(null)
const TaskContext = createContext(null)
const ContactsContext = createContext(null)
const GroupsContext = createContext(null)
const FileContext = createContext(null)

// Permission constants
const PERM_FILE = 'file'
const PERM_CONTACTS = 'contacts'
const PERM_ROLE = 'role'
const PERM_GROUP = 'group'
const PERM_AUDIT = 'audit'
const PERM_ALL = '*'

export const LoginProvider = ({ children }) => {
  const [loginUser, setLoginUserState] = useState(localStorage.getItem('loginUser') || '')
  const [userPermissions, setUserPermissionsState] = useState(
    localStorage.getItem('userPermissions') || ''
  )
  const [userAvatar, setUserAvatar] = useState(
    loginUser ? `/api/user/avatar/${loginUser}` : ''
  )
  const [permissionsLoaded, setPermissionsLoaded] = useState(false)

  const setLoginUser = (user, permissions) => {
    setLoginUserState(user)
    setUserAvatar(user ? `/api/user/avatar/${user}` : '')
    if (user) {
      localStorage.setItem('loginUser', user)
    } else {
      localStorage.removeItem('loginUser')
    }
    if (permissions !== undefined) {
      setUserPermissionsState(permissions || '')
      setPermissionsLoaded(true)
      if (permissions) {
        localStorage.setItem('userPermissions', permissions)
      } else {
        localStorage.removeItem('userPermissions')
      }
    }
  }

  const updateAvatar = (timestamp) => {
    if (!loginUser) return
    if (timestamp === null) {
      setUserAvatar('')
    } else {
      setUserAvatar(`/api/user/avatar/${loginUser}?t=${timestamp}`)
    }
  }

  const clearLoginUser = () => {
    setLoginUserState('')
    setUserPermissionsState('')
    setUserAvatar('')
    localStorage.removeItem('loginUser')
    localStorage.removeItem('userPermissions')
  }

  // Permission check helper
  const hasPermission = (perm) => {
    if (userPermissions === PERM_ALL) return true
    return userPermissions.split(',').some((p) => p.trim() === perm)
  }

  const canFile = hasPermission(PERM_FILE)
  const canContacts = hasPermission(PERM_CONTACTS)
  const canRole = hasPermission(PERM_ROLE)
  const canGroup = hasPermission(PERM_GROUP)
  const canAudit = hasPermission(PERM_AUDIT)

  const value = useMemo(
    () => ({
      loginUser,
      userPermissions,
      userAvatar,
      permissionsLoaded,
      canFile,
      canContacts,
      canRole,
      canGroup,
      canAudit,
      hasPermission,
      setLoginUser,
      updateAvatar,
      clearLoginUser
    }),
    [loginUser, userPermissions, userAvatar, permissionsLoaded]
  )

  return <LoginContext.Provider value={value}>{children}</LoginContext.Provider>
}

export const TaskProvider = ({ children }) => {
  const [tasks, setTasks] = useState([])
  const [currentTask, setCurrentTask] = useState(null)

  const calcProgress = (task) => {
    const totalSize = task.totalSize || 0
    if (totalSize > 0) {
      return Math.min(100, Math.max(0, Math.round(((task.copiedSize || 0) / totalSize) * 100)))
    }
    if (task.totalFiles) {
      return Math.min(100, Math.max(0, Math.round(((task.copiedFiles || 0) / task.totalFiles) * 100)))
    }
    return 0
  }

  const updateTasks = (newTasks) => {
    setTasks((prev) => {
      const next = [...prev]
      newTasks.forEach((newTask) => {
        const idx = next.findIndex((task) => task.id === newTask.id)
        const updated = { ...newTask, progress: calcProgress(newTask) }
        if (idx !== -1) {
          next[idx] = { ...next[idx], ...updated }
        } else {
          next.push(updated)
        }
      })

      const statusOrder = {
        running: 1,
        suspended: 1,
        pending: 2,
        starting: 3,
        completed: 4,
        cancelled: 4,
        failed: 4
      }

      return [...next].sort((a, b) => {
        if (statusOrder[a.status] !== statusOrder[b.status]) {
          return statusOrder[a.status] - statusOrder[b.status]
        }
        if (a.status === 'running' || a.status === 'suspended' || a.status === 'pending') {
          return b.createdAt - a.createdAt
        }
        return b.updatedAt - a.updatedAt
      })
    })
  }

  const deleteTask = (taskId) => {
    setTasks((prev) => prev.filter((task) => task.id !== taskId))
  }

  const value = useMemo(
    () => ({
      tasks,
      currentTask,
      setCurrentTask,
      updateTasks,
      deleteTask,
      activeTasks: tasks.filter(
        (task) => !['completed', 'cancelled', 'failed'].includes(task.status)
      ),
      completedTasks: tasks.filter((task) => task.status === 'completed')
    }),
    [tasks, currentTask]
  )

  return <TaskContext.Provider value={value}>{children}</TaskContext.Provider>
}

export const ContactsProvider = ({ children }) => {
  const [selectedContacts, setSelectedContacts] = useState(null)
  const value = useMemo(
    () => ({
      selectedContacts,
      setSelectedContacts
    }),
    [selectedContacts]
  )
  return <ContactsContext.Provider value={value}>{children}</ContactsContext.Provider>
}

export const GroupsProvider = ({ children }) => {
  const [selectedGroups, setSelectedGroups] = useState(null)
  const value = useMemo(
    () => ({
      selectedGroups,
      setSelectedGroups
    }),
    [selectedGroups]
  )
  return <GroupsContext.Provider value={value}>{children}</GroupsContext.Provider>
}

export const FileProvider = ({ children }) => {
  const [currentPath, setCurrentPath] = useState('/')
  const [menuChildren, setMenuChildren] = useState([])

  const updateMenuChildren = async (path) => {
    try {
      const response = await http.get('/api/file/list', { params: { path } })
      const contents = response.data || []
      const children = contents
        .filter((item) => item.type === 'directory')
        .sort((a, b) => new Date(b.lastmod).getTime() - new Date(a.lastmod).getTime())
        .map((item) => ({
          label: item.basename,
          key: `docs${item.filename}`,
          children: []
        }))

      if (path === '/') {
        setMenuChildren(children)
        return
      }

      const parentKey = `docs${path}`
      setMenuChildren((prev) => {
        const next = [...prev]
        const updateNode = (nodes) => {
          for (const node of nodes) {
            if (node.key === parentKey) {
              node.children = children
              return true
            }
            if (node.children?.length) {
              if (updateNode(node.children)) {
                return true
              }
            }
          }
          return false
        }
        updateNode(next)
        return next
      })
    } catch (error) {
      console.error('Failed to load directory tree', error)
    }
  }

  const value = useMemo(
    () => ({
      currentPath,
      setCurrentPath,
      menuChildren,
      updateMenuChildren
    }),
    [currentPath, menuChildren]
  )

  return <FileContext.Provider value={value}>{children}</FileContext.Provider>
}

export const useLogin = () => useContext(LoginContext)
export const useTasks = () => useContext(TaskContext)
export const useContacts = () => useContext(ContactsContext)
export const useGroups = () => useContext(GroupsContext)
export const useFile = () => useContext(FileContext)
