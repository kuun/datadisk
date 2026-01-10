import { MoreHorizontal, Pencil, Plus, RefreshCw, Trash2 } from 'lucide-react'
import React, { useEffect, useRef, useState } from 'react'
import http from '../lib/http'
import { alertError, alertSuccess } from '../lib/utils'
import { Button } from './ui/button'
import { Checkbox } from './ui/checkbox'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from './ui/dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger
} from './ui/dropdown-menu'
import { Input } from './ui/input'
import { Label } from './ui/label'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from './ui/table'
import './RoleManager.css'

const DEFAULT_PERMISSION_OPTIONS = [
  { key: 'file', label: '文件管理', description: '上传、下载、创建、删除文件' },
  { key: 'contacts', label: '通讯录', description: '管理用户、部门' },
  { key: 'role', label: '角色管理', description: '管理角色与角色权限' },
  { key: 'group', label: '群组', description: '管理群组及群组成员' },
  { key: 'audit', label: '审计', description: '查看操作日志' }
]

const RoleManager = ({ onRolesChange }) => {
  const [roles, setRoles] = useState([])
  const [selectedRole, setSelectedRole] = useState(null)
  const [permissionOptions, setPermissionOptions] = useState(DEFAULT_PERMISSION_OPTIONS)
  const [dialogVisible, setDialogVisible] = useState(false)
  const [dialogTitle, setDialogTitle] = useState('添加角色')
  const [roleForm, setRoleForm] = useState({ name: '', description: '', permissions: [] })
  const [visibleCount, setVisibleCount] = useState(4)
  const containerRef = useRef(null)

  const loadRoles = async () => {
    try {
      const resp = await http.get('/api/role/list')
      if (resp.data.success) {
        setRoles(resp.data.data || [])
        if (onRolesChange) {
          onRolesChange(resp.data.data || [])
        }
      }
    } catch (error) {
      alertError('加载角色列表失败')
    }
  }

  useEffect(() => {
    loadRoles()
    http
      .get('/api/role/permissions')
      .then((resp) => {
        if (resp.data.success) {
          const options = (resp.data.data || []).map((item) => ({
            key: item.key,
            label: item.label || item.name,
            description: item.description || ''
          }))
          setPermissionOptions(options.length ? options : DEFAULT_PERMISSION_OPTIONS)
        }
      })
      .catch(() => {})
  }, [])

  const preAdd = () => {
    setDialogTitle('添加角色')
    setRoleForm({ name: '', description: '', permissions: [] })
    setDialogVisible(true)
  }

  const preEdit = () => {
    if (!selectedRole) {
      alertError('请先选择角色')
      return
    }
    setDialogTitle('编辑角色')
    setRoleForm({
      oldName: selectedRole.name,
      name: selectedRole.name,
      description: selectedRole.description || '',
      permissions: selectedRole.permissionList || []
    })
    setDialogVisible(true)
  }

  const handlePermissionChange = (key, checked) => {
    setRoleForm((prev) => {
      const newPerms = checked
        ? [...prev.permissions, key]
        : prev.permissions.filter((p) => p !== key)
      return { ...prev, permissions: newPerms }
    })
  }

  const save = async () => {
    if (!roleForm.name.trim()) {
      alertError('角色名称不能为空')
      return
    }

    try {
      const payload = {
        ...roleForm,
        permissions: roleForm.permissions.join(',')
      }

      let resp
      if (roleForm.oldName) {
        // Editing existing role
        resp = await http.post('/api/role/update', payload)
      } else {
        // Adding new role
        resp = await http.post('/api/role/add', payload)
      }

      if (resp.data.code) {
        setDialogVisible(false)
        alertSuccess('保存成功')
        await loadRoles()
        setSelectedRole(null)
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.message || '保存失败')
    }
  }

  const deleteRole = async () => {
    if (!selectedRole) {
      alertError('请先选择角色')
      return
    }
    if (selectedRole.name === 'admin' || selectedRole.name === 'user') {
      alertError('不能删除内置角色')
      return
    }
    const confirmed = window.confirm(`确认删除角色 "${selectedRole.name}" 吗？`)
    if (!confirmed) return

    try {
      const resp = await http.post(`/api/role/delete?name=${encodeURIComponent(selectedRole.name)}`)
      if (resp.data.code) {
        alertSuccess('删除成功')
        await loadRoles()
        setSelectedRole(null)
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.message || '删除失败')
    }
  }

  const actions = [
    { key: 'add', label: '添加', icon: Plus, onClick: preAdd },
    { key: 'edit', label: '编辑', icon: Pencil, onClick: preEdit },
    { key: 'delete', label: '删除', icon: Trash2, onClick: deleteRole, danger: true },
    { key: 'refresh', label: '刷新', icon: RefreshCw, onClick: loadRoles }
  ]

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const calculateVisibleCount = () => {
      const width = container.offsetWidth
      const btnWidth = 75
      const moreWidth = 40
      const available = width - moreWidth - 16
      const count = Math.floor(available / btnWidth)
      setVisibleCount(Math.max(0, Math.min(count, actions.length)))
    }

    const observer = new ResizeObserver(calculateVisibleCount)
    observer.observe(container)
    calculateVisibleCount()

    return () => observer.disconnect()
  }, [])

  const formatPermissions = (permList) => {
    if (!permList || permList.length === 0) return '-'
    return permList
      .map((p) => {
        const opt = permissionOptions.find((o) => o.key === p)
        return opt ? opt.label : p
      })
      .join(', ')
  }

  return (
    <div className="role-manager">
      <div className="role-actions" ref={containerRef}>
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
      <div className="role-table">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>角色名称</TableHead>
              <TableHead>权限</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {roles.map((role) => (
              <TableRow
                key={role.name}
                className={selectedRole?.name === role.name ? 'selected' : ''}
                onClick={() => setSelectedRole(role)}
              >
                <TableCell className="font-medium">{role.name}</TableCell>
                <TableCell className="text-muted-foreground text-sm">
                  {formatPermissions(role.permissionList)}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <Dialog open={dialogVisible} onOpenChange={setDialogVisible}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{dialogTitle}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="role-name">角色名称</Label>
              <Input
                id="role-name"
                value={roleForm.name}
                onChange={(e) => setRoleForm((prev) => ({ ...prev, name: e.target.value }))}
                placeholder="请输入角色名称"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="role-desc">描述</Label>
              <Input
                id="role-desc"
                value={roleForm.description}
                onChange={(e) => setRoleForm((prev) => ({ ...prev, description: e.target.value }))}
                placeholder="请输入角色描述（可选）"
              />
            </div>
            <div className="space-y-2">
              <Label>权限</Label>
              <div className="grid grid-cols-2 gap-3">
                {permissionOptions.map((perm) => (
                  <div key={perm.key} className="flex items-start space-x-2">
                    <Checkbox
                      id={`perm-${perm.key}`}
                      checked={roleForm.permissions.includes(perm.key)}
                      onCheckedChange={(checked) => handlePermissionChange(perm.key, checked)}
                    />
                    <div className="grid gap-0.5 leading-none">
                      <Label htmlFor={`perm-${perm.key}`} className="text-sm font-medium">
                        {perm.label}
                      </Label>
                      <p className="text-xs text-muted-foreground">{perm.description}</p>
                    </div>
                  </div>
                ))}
              </div>
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
    </div>
  )
}

export default RoleManager
