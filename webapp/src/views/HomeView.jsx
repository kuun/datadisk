import React, { useEffect, useMemo, useState } from 'react'
import { Outlet, useLocation, useNavigate } from 'react-router-dom'
import { LogOut, Settings } from 'lucide-react'
import http from '../lib/http'
import bus from '../components/uploader/bus'
import { useLogin, useTasks } from '../store/providers'
import { Button } from '../components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger
} from '../components/ui/dropdown-menu'
import TaskManager from '../components/TaskManager'
import { t } from '../lib/i18n'
import './HomeView.css'

const HomeView = () => {
  const navigate = useNavigate()
  const location = useLocation()
  const { setLoginUser, userAvatar, canFile, canContacts, canRole, canGroup, canAudit } = useLogin()
  const { updateTasks, deleteTask } = useTasks()
  const [username, setUsername] = useState('')
  const [activeIndex, setActiveIndex] = useState('1')
  const [isAuthChecking, setIsAuthChecking] = useState(true)

  const viewMap = useMemo(
    () => [
      { url: '/ui/file', activeIndex: '1' },
      { url: '/ui/contacts', activeIndex: '2' },
      { url: '/ui/audit', activeIndex: '3' },
      { url: '/ui/group', activeIndex: '4' },
      { url: '/ui/settings', activeIndex: '-1' }
    ],
    []
  )

  useEffect(() => {
    http
      .get('/api/user/current')
      .then((res) => {
        setUsername(res.data.username)
        setLoginUser(res.data.username, res.data.permissions)
        setIsAuthChecking(false)

        // Check if user has permission for current route, redirect if not
        const permissions = res.data.permissions || ''
        const hasAll = permissions === '*'
        const permList = permissions.split(',').map((p) => p.trim())
        const hasPerm = (p) => hasAll || permList.includes(p)

        const path = location.pathname
        const canAccessContacts = hasPerm('contacts') || hasPerm('role')
        const needsRedirect =
          (path.includes('/ui/file') && !hasPerm('file')) ||
          (path.includes('/ui/contacts') && !canAccessContacts) ||
          (path.includes('/ui/audit') && !hasPerm('audit')) ||
          (path.includes('/ui/group') && !hasPerm('group'))

        if (needsRedirect || path === '/' || path === '') {
          // Redirect to first available permission
          if (hasPerm('file')) {
            navigate('/ui/file', { replace: true })
          } else if (hasPerm('contacts') || hasPerm('role')) {
            navigate('/ui/contacts', { replace: true })
          } else if (hasPerm('audit')) {
            navigate('/ui/audit', { replace: true })
          } else if (hasPerm('group')) {
            navigate('/ui/group', { replace: true })
          } else {
            navigate('/ui/settings', { replace: true })
          }
        }
      })
      .catch(() => {
        // http interceptor will redirect to login on 401
      })

    viewMap.forEach((view) => {
      if (location.pathname.includes(view.url)) {
        setActiveIndex(view.activeIndex)
      }
    })
  }, [])

  useEffect(() => {
    viewMap.forEach((view) => {
      if (location.pathname.includes(view.url)) {
        setActiveIndex(view.activeIndex)
      }
    })
  }, [location.pathname, viewMap])

  useEffect(() => {
    const host = window.location.host
    const ws = new WebSocket(`ws://${host}/api/ws`)
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data)
      if (message.type === 'task') {
        updateTasks([message.data])
      } else if (message.type === 'task_deleted') {
        deleteTask(message.data)
      }
    }
    return () => ws.close()
  }, [updateTasks, deleteTask])

  const logout = () => {
    bus.emit('closeUploadPanel', true)
    http.post('/api/logout').finally(() => navigate('/ui/login'))
  }

  const goToSettings = () => {
    navigate('/ui/settings')
    setActiveIndex('-1')
  }

  const handleSelect = (index) => {
    setActiveIndex(index)
    const target = viewMap.find((view) => view.activeIndex === index)
    if (target) {
      navigate(target.url)
    }
  }

  if (isAuthChecking) {
    return null
  }

  return (
    <div className="app-layout">
      <header className="app-header">
        <div className="app-logo">
          <img src="/assets/img/datadisk-logo.png" alt="dataDISK" />
        </div>
        <nav className="app-nav">
          {canFile && (
            <button
              type="button"
              className={`nav-pill ${activeIndex === '1' ? 'active' : ''}`}
              onClick={() => handleSelect('1')}
            >
              {t('menu.files')}
            </button>
          )}
          {(canContacts || canRole) && (
            <button
              type="button"
              className={`nav-pill ${activeIndex === '2' ? 'active' : ''}`}
              onClick={() => handleSelect('2')}
            >
              {t('menu.contacts')}
            </button>
          )}
          {canAudit && (
            <button
              type="button"
              className={`nav-pill ${activeIndex === '3' ? 'active' : ''}`}
              onClick={() => handleSelect('3')}
            >
              {t('menu.audit')}
            </button>
          )}
          {canGroup && (
            <button
              type="button"
              className={`nav-pill ${activeIndex === '4' ? 'active' : ''}`}
              onClick={() => handleSelect('4')}
            >
              {t('menu.group')}
            </button>
          )}
        </nav>
        <div className="app-actions">
          <TaskManager />
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                className="avatar-trigger rounded-full focus-visible:ring-0 focus-visible:ring-offset-0"
              >
                <span className="avatar-ring">
                  {userAvatar ? (
                    <span
                      className="avatar-photo"
                      style={{ backgroundImage: `url(${userAvatar})` }}
                      aria-label={username}
                    />
                  ) : (
                    <span className="avatar-fallback">
                      {username.slice(0, 2).toUpperCase()}
                    </span>
                  )}
                </span>
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="user-menu">
              <div className="user-menu-header">
                <span className="user-menu-avatar">
                  {userAvatar ? (
                    <span
                      className="user-menu-photo"
                      style={{ backgroundImage: `url(${userAvatar})` }}
                      aria-label={username}
                    />
                  ) : (
                    <span className="user-menu-fallback">
                      {username.slice(0, 2).toUpperCase()}
                    </span>
                  )}
                </span>
                <div className="user-menu-meta">
                  <div className="user-menu-name">{username || 'User'}</div>
                  <div className="user-menu-handle">
                    {username ? `@${username}` : '@user'}
                  </div>
                </div>
              </div>
              <DropdownMenuSeparator className="user-menu-sep" />
              <DropdownMenuItem className="user-menu-item" onClick={goToSettings}>
                <Settings className="mr-2 h-4 w-4" />
                {t('menu.setting')}
              </DropdownMenuItem>
              <DropdownMenuItem className="user-menu-item danger" onClick={logout}>
                <LogOut className="mr-2 h-4 w-4" />
                {t('menu.logout')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </header>
      <main className="app-main">
        <Outlet />
      </main>
    </div>
  )
}

export default HomeView
