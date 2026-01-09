import { ChevronDown, ChevronRight, MoreHorizontal, Pencil, Plus, RefreshCw, Trash2 } from 'lucide-react'
import React, { useEffect, useMemo, useRef, useState } from 'react'
import http from '../lib/http'
import { alertError, alertSuccess } from '../lib/utils'
import { useContacts } from '../store/providers'
import { Button } from './ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from './ui/dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger
} from './ui/dropdown-menu'
import { Input } from './ui/input'
import { Label } from './ui/label'
import './DepartmentManager.css'

const buildTree = (items) => {
  const map = new Map()
  items.forEach((item) => map.set(item.id, { ...item, children: [] }))
  const roots = []
  map.forEach((item) => {
    if (item.parentId && map.has(item.parentId)) {
      map.get(item.parentId).children.push(item)
    } else {
      roots.push(item)
    }
  })
  return roots
}

const TreeNode = ({ node, level, expandedKeys, selectedId, contextMenuNodeId, onToggle, onSelect, onContextMenu }) => {
  const hasChildren = node.children && node.children.length > 0
  const isExpanded = expandedKeys.has(node.id)
  const isSelected = selectedId === node.id
  const isContextActive = contextMenuNodeId === node.id

  const handleContextMenu = (e) => {
    e.preventDefault()
    e.stopPropagation()
    onSelect(node)
    onContextMenu(e, node)
  }

  return (
    <>
      <div
        className={`dept-tree-node ${isSelected ? 'active' : ''} ${isContextActive ? 'context-active' : ''}`}
        style={{ paddingLeft: `${level * 18 + 8}px` }}
        onClick={() => onSelect(node)}
        onContextMenu={handleContextMenu}
        role="button"
        tabIndex={0}
      >
        <button
          type="button"
          className="dept-tree-toggle"
          onClick={(e) => {
            e.stopPropagation()
            if (hasChildren) onToggle(node.id)
          }}
          style={{ visibility: hasChildren ? 'visible' : 'hidden' }}
        >
          {isExpanded ? (
            <ChevronDown className="dept-tree-icon" />
          ) : (
            <ChevronRight className="dept-tree-icon" />
          )}
        </button>
        <span className="dept-tree-label">{node.name}</span>
      </div>
      {hasChildren && isExpanded && (
        <div className="dept-tree-children">
          {node.children.map((child) => (
            <TreeNode
              key={child.id}
              node={child}
              level={level + 1}
              expandedKeys={expandedKeys}
              selectedId={selectedId}
              contextMenuNodeId={contextMenuNodeId}
              onToggle={onToggle}
              onSelect={onSelect}
              onContextMenu={onContextMenu}
            />
          ))}
        </div>
      )}
    </>
  )
}

const DepartmentManager = () => {
  const { selectedContacts, setSelectedContacts } = useContacts()
  const [departments, setDepartments] = useState([])
  const [dialogVisible, setDialogVisible] = useState(false)
  const [dialogTitle, setDialogTitle] = useState('添加')
  const [departmentForm, setDepartmentForm] = useState({ name: '' })
  const [visibleCount, setVisibleCount] = useState(4)
  const [expandedKeys, setExpandedKeys] = useState(new Set())
  const containerRef = useRef(null)
  const [contextMenu, setContextMenu] = useState({ visible: false, x: 0, y: 0, node: null })

  const loadDepartments = async () => {
    const resp = await http.get('/api/department/query')
    setDepartments(resp.data.data || [])
    setSelectedContacts(null)
  }

  useEffect(() => {
    loadDepartments()
  }, [])

  const treeData = useMemo(() => buildTree(departments), [departments])

  const toggleExpand = (id) => {
    setExpandedKeys((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }

  // 右键菜单处理
  const handleContextMenu = (e, node) => {
    setContextMenu({
      visible: true,
      x: e.clientX,
      y: e.clientY,
      node
    })
  }

  const closeContextMenu = () => {
    setContextMenu({ visible: false, x: 0, y: 0, node: null })
  }

  // 点击其他地方关闭菜单
  useEffect(() => {
    const handleClick = () => closeContextMenu()
    if (contextMenu.visible) {
      document.addEventListener('click', handleClick)
      return () => document.removeEventListener('click', handleClick)
    }
  }, [contextMenu.visible])

  // 右键菜单 - 修改
  const handleEditFromContext = () => {
    const node = contextMenu.node
    closeContextMenu()
    setDialogTitle('修改')
    setDepartmentForm({
      id: node.id,
      name: node.name,
      level: node.level,
      parentId: node.parentId,
      parentName: node.parentName
    })
    setDialogVisible(true)
  }

  // 右键菜单 - 删除
  const handleDeleteFromContext = async () => {
    const node = contextMenu.node
    closeContextMenu()
    const confirmed = window.confirm(`确认删除部门 "${node.name}" 吗？`)
    if (!confirmed) return
    try {
      const resp = await http.post(`/api/department/delete?id=${node.id}`)
      if (resp.data.code) {
        alertSuccess('删除成功')
        await loadDepartments()
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.error || '删除失败')
    }
  }

  // 添加部门（无参数时添加根部门，有参数时添加子部门）
  const preAdd = (parentNode = null) => {
    closeContextMenu()
    setDialogTitle('添加')
    if (parentNode) {
      setDepartmentForm({ name: '', parentId: parentNode.id, level: parentNode.level + 1 })
    } else {
      setDepartmentForm({ name: '' })
    }
    setDialogVisible(true)
  }

  const save = () => {
    if (dialogTitle === '添加') {
      addDepartment()
    } else {
      modifyDepartment()
    }
  }

  const addDepartment = async () => {
    if (!departmentForm.name) {
      alertError('部门名称不能为空')
      return
    }
    try {
      // 如果是添加子部门，使用表单中的 parentId 和 level
      // 如果是普通添加，使用 selectedContacts 或默认值
      const parentId = departmentForm.parentId !== undefined ? departmentForm.parentId : (selectedContacts?.id || 0)
      const level = departmentForm.level !== undefined ? departmentForm.level : ((selectedContacts?.level || 0) + 1)
      const payload = {
        name: departmentForm.name,
        parentId: parentId,
        level: level
      }
      const resp = await http.post('/api/departments/add', payload)
      if (resp.data.code) {
        setDialogVisible(false)
        await loadDepartments()
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.error || '添加失败')
    }
  }

  const modifyDepartment = async () => {
    if (!departmentForm.name) {
      alertError('部门名称不能为空')
      return
    }
    try {
      const resp = await http.post('/api/department/update', departmentForm)
      if (resp.data.code) {
        setDialogVisible(false)
        await loadDepartments()
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.error || '修改失败')
    }
  }

  const deleteDepartment = async () => {
    if (!selectedContacts) {
      alertError('请先选择部门')
      return
    }
    const confirmed = window.confirm('确认删除该部门吗？')
    if (!confirmed) return
    const resp = await http.post(`/api/department/delete?id=${selectedContacts.id}`)
    if (resp.data.code) {
      alertSuccess('删除成功')
      await loadDepartments()
    } else {
      alertError(resp.data.message)
    }
  }

  // 按钮配置 - 放在函数定义之后，这样可以正确引用
  const actions = [
    { key: 'add', label: '添加', icon: Plus, onClick: () => preAdd() },
    { key: 'refresh', label: '刷新', icon: RefreshCw, onClick: loadDepartments }
  ]

  // 监听容器宽度变化
  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const calculateVisibleCount = () => {
      const width = container.offsetWidth
      const btnWidth = 75 // 每个按钮大约宽度
      const moreWidth = 40 // 更多按钮宽度
      const available = width - moreWidth - 16 // 留出间隙
      const count = Math.floor(available / btnWidth)
      setVisibleCount(Math.max(0, Math.min(count, actions.length)))
    }

    const observer = new ResizeObserver(calculateVisibleCount)
    observer.observe(container)
    calculateVisibleCount()

    return () => observer.disconnect()
  }, [])

  return (
    <div className="department-manager">
      <div className="department-actions" ref={containerRef}>
        {visibleCount > 0 && (
          <div className="toolbar-segment">
            {actions.slice(0, visibleCount).map((action) => (
              <button
                key={action.key}
                type="button"
                className={`segment-btn ${action.danger ? 'danger' : ''}`}
                onClick={action.onClick}
              >
                <action.icon className="mr-1 h-3.5 w-3.5" />
                {action.label}
              </button>
            ))}
          </div>
        )}
        {visibleCount < actions.length && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button type="button" className="toolbar-collapse">
                <MoreHorizontal className="h-4 w-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              {actions.slice(visibleCount).map((action) => (
                <DropdownMenuItem
                  key={action.key}
                  onClick={action.onClick}
                  className={action.danger ? 'text-red-600' : ''}
                >
                  <action.icon className="mr-2 h-4 w-4" />
                  {action.label}
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>
      <div className="ship-border" />
      <div className="department-tree">
        {treeData.map((node) => (
          <TreeNode
            key={node.id}
            node={node}
            level={0}
            expandedKeys={expandedKeys}
            selectedId={selectedContacts?.id}
            contextMenuNodeId={contextMenu.visible ? contextMenu.node?.id : null}
            onToggle={toggleExpand}
            onSelect={setSelectedContacts}
            onContextMenu={handleContextMenu}
          />
        ))}
      </div>

      <Dialog open={dialogVisible} onOpenChange={setDialogVisible}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{dialogTitle}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="dept-name">名称</Label>
              <Input
                id="dept-name"
                value={departmentForm.name}
                onChange={(event) => setDepartmentForm((prev) => ({ ...prev, name: event.target.value }))}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setDialogVisible(false)}>
              取消
            </Button>
            <Button onClick={save}>保存</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 右键菜单 */}
      {contextMenu.visible && (
        <div
          className="dept-context-menu"
          style={{
            position: 'fixed',
            left: contextMenu.x,
            top: contextMenu.y,
            zIndex: 1000
          }}
        >
          <div className="dept-context-menu-item" onClick={() => preAdd(contextMenu.node)}>
            <Plus className="h-4 w-4 mr-2" />
            添加子部门
          </div>
          <div className="dept-context-menu-item" onClick={handleEditFromContext}>
            <Pencil className="h-4 w-4 mr-2" />
            修改
          </div>
          <div className="dept-context-menu-item danger" onClick={handleDeleteFromContext}>
            <Trash2 className="h-4 w-4 mr-2" />
            删除
          </div>
        </div>
      )}
    </div>
  )
}

export default DepartmentManager
