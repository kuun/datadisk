import React, { useEffect, useMemo, useState } from 'react'
import Split from 'react-split'
import { ChevronDown, ChevronRight, Clock, Folder, Star, Tag } from 'lucide-react'
import { Outlet, useLocation, useNavigate, useSearchParams } from 'react-router-dom'
import { useFile } from '../store/providers'
import { t } from '../lib/i18n'
import './FileView.css'

const FileView = () => {
  const navigate = useNavigate()
  const location = useLocation()
  const [searchParams] = useSearchParams()
  const { menuChildren, updateMenuChildren } = useFile()
  const [currentKey, setCurrentKey] = useState('recent')
  const [expandedKeys, setExpandedKeys] = useState(new Set())

  const quickNav = useMemo(
    () => [
      { label: t('menu.recent'), icon: Clock, key: 'recent' },
      { label: t('menu.star'), icon: Star, key: 'star' },
      { label: t('menu.tags'), icon: Tag, key: 'tags' }
    ],
    []
  )
  const docRoot = useMemo(
    () => ({ label: t('menu.mydocs'), icon: Folder, key: 'docs', children: menuChildren }),
    [menuChildren]
  )

  // Calculate which key to highlight: if current key's node is not visible,
  // highlight the nearest expanded parent directory
  const highlightKey = useMemo(() => {
    if (!currentKey.startsWith('docs')) return currentKey
    if (currentKey === 'docs') return 'docs'
    // Check if current key is visible (all parent nodes are expanded)
    const parts = currentKey.replace('docs/', '').split('/')
    let visibleKey = 'docs'
    for (let i = 0; i < parts.length; i += 1) {
      const parentKey = i === 0 ? 'docs' : `docs/${parts.slice(0, i).join('/')}`
      if (!expandedKeys.has(parentKey)) {
        // Parent not expanded, return the last visible parent
        return visibleKey
      }
      visibleKey = `docs/${parts.slice(0, i + 1).join('/')}`
    }
    return currentKey
  }, [currentKey, expandedKeys])

  useEffect(() => {
    const newPath = searchParams.get('path')
    if (newPath) {
      setCurrentKey(newPath === '/' ? 'docs' : `docs${newPath}`)
      return
    }
    if (location.pathname.includes('/ui/file/recent')) setCurrentKey('recent')
    if (location.pathname.includes('/ui/file/star')) setCurrentKey('star')
    if (location.pathname.includes('/ui/file/tags')) setCurrentKey('tags')
  }, [location.pathname, searchParams])


  const expandDocNode = (key) => {
    setExpandedKeys((prev) => {
      const next = new Set(prev)
      next.add('docs')
      next.add(key)
      return next
    })
  }

  const handleNodeClick = (node) => {
    setCurrentKey(node.key)
    if (node.key === 'docs') {
      navigate('/ui/file/mydocs?path=/')
      return
    }
    if (node.key.startsWith('docs/')) {
      const realPath = node.key.replace('docs', '')
      navigate(`/ui/file/mydocs?path=${encodeURIComponent(realPath)}`)
      return
    }
    navigate(`/ui/file/${node.key}`)
  }

  const toggleExpand = (key) => {
    setExpandedKeys((prev) => {
      const next = new Set(prev)
      if (next.has(key)) {
        next.delete(key)
      } else {
        next.add(key)
      }
      return next
    })
  }

  const handleToggle = (node) => {
    const isExpanded = expandedKeys.has(node.key)
    if (!isExpanded) {
      if (node.key === 'docs') {
        updateMenuChildren('/')
      } else if (node.key.startsWith('docs/') && (!node.children || node.children.length === 0)) {
        const realPath = node.key.replace('docs', '')
        updateMenuChildren(realPath)
      }
    }
    toggleExpand(node.key)
  }

  const renderDocChildren = (nodes) => {
    if (!nodes || nodes.length === 0) return null
    return (
      <ul className="file-tree-children">
        {nodes.map((node) => (
          <li key={node.key}>
            <div
              className={`file-tree-node ${highlightKey === node.key ? 'active' : ''}`}
              onClick={() => handleNodeClick(node)}
              role="button"
              tabIndex={0}
            >
              <button
                type="button"
                className="file-tree-toggle"
                onClick={(event) => {
                  event.stopPropagation()
                  handleToggle(node)
                }}
                aria-label="toggle"
              >
                {expandedKeys.has(node.key) ? (
                  <ChevronDown className="file-tree-toggle-icon" />
                ) : (
                  <ChevronRight className="file-tree-toggle-icon" />
                )}
              </button>
              <button className="file-tree-label" type="button">
                <Folder className="file-tree-icon" />
                <span>{node.label}</span>
              </button>
            </div>
            {expandedKeys.has(node.key) && renderDocChildren(node.children)}
          </li>
        ))}
      </ul>
    )
  }

  const DocIcon = docRoot.icon

  return (
    <div className="file-container">
      <Split className="split" sizes={[15, 85]} minSize={200} gutterSize={6}>
        <aside className="file-nav">
          <div className="file-nav-header">
            {quickNav.map((item) => {
              const Icon = item.icon
              return (
                <button
                  key={item.key}
                  className={`file-nav-item ${currentKey === item.key ? 'active' : ''}`}
                  onClick={() => handleNodeClick(item)}
                  type="button"
                >
                  <Icon className="file-tree-icon" />
                  <span>{item.label}</span>
                </button>
              )
            })}
          </div>
          <div className="file-tree-scroll">
            <ul className="file-tree">
              <li>
                <div
                  className={`file-tree-node ${highlightKey === docRoot.key ? 'active' : ''}`}
                  onClick={() => handleNodeClick(docRoot)}
                  role="button"
                  tabIndex={0}
                >
                  <button
                    type="button"
                    className="file-tree-toggle"
                    onClick={(event) => {
                      event.stopPropagation()
                      handleToggle(docRoot)
                    }}
                    aria-label="toggle"
                  >
                    {expandedKeys.has(docRoot.key) ? (
                      <ChevronDown className="file-tree-toggle-icon" />
                    ) : (
                      <ChevronRight className="file-tree-toggle-icon" />
                    )}
                  </button>
                  <button className="file-tree-label" type="button">
                    <DocIcon className="file-tree-icon" />
                    <span>{docRoot.label}</span>
                  </button>
                </div>
                {expandedKeys.has(docRoot.key) && renderDocChildren(docRoot.children)}
              </li>
            </ul>
          </div>
        </aside>
        <section className="file-content">
          <Outlet />
        </section>
      </Split>
    </div>
  )
}

export default FileView
