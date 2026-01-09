import { Ban, Check, KeyRound, Pencil, Plus, RefreshCw, Trash2 } from 'lucide-react'
import React, { useEffect, useState } from 'react'
import http from '../lib/http'
import { alertError, alertSuccess } from '../lib/utils'
import { useContacts } from '../store/providers'
import { Button } from './ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from './ui/dialog'
import { Input } from './ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from './ui/table'
import './UserManager.css'

const UserManager = ({ roles = [] }) => {
  const { selectedContacts } = useContacts()
  const [userList, setUserList] = useState([])
  const [selectedUsers, setSelectedUsers] = useState([])
  const [dialogVisible, setDialogVisible] = useState(false)
  const [title, setTitle] = useState('添加')
  const [userForm, setUserForm] = useState({
    username: '',
    fullName: '',
    phone: '',
    email: '',
    password: '',
    nextPwd: '',
    quota: 1,
    quotaUnit: 'GB',
    role: null
  })
  const [contextMenu, setContextMenu] = useState({ visible: false, x: 0, y: 0, user: null })
  const [pwdDialogVisible, setPwdDialogVisible] = useState(false)
  const [pwdForm, setPwdForm] = useState({ password: '', nextPwd: '' })
  const [pwdUser, setPwdUser] = useState(null)

  const refreshUser = async () => {
    if (!selectedContacts?.id) {
      setUserList([])
      setSelectedUsers([])
      return
    }
    const resp = await http.get(`/api/user/query?departmentId=${selectedContacts.id}`)
    const newList = resp.data.data || []
    setUserList(newList)
    // 同步更新选中用户的数据
    setSelectedUsers((prev) => {
      if (prev.length === 0) return prev
      const newMap = new Map(newList.map((u) => [u.id, u]))
      return prev.map((u) => newMap.get(u.id)).filter(Boolean)
    })
  }

  useEffect(() => {
    refreshUser()
  }, [selectedContacts?.id])

  const preAdd = () => {
    if (!selectedContacts) {
      alertError('请先选择一个部门')
      return
    }
    setTitle('添加')
    setUserForm({
      username: '',
      fullName: '',
      phone: '',
      email: '',
      password: '',
      nextPwd: '',
      quota: 1,
      quotaUnit: 'GB',
      role: null
    })
    setDialogVisible(true)
  }

  const save = () => {
    if (title === '添加') {
      addUser()
    } else {
      modifyUser()
    }
  }

  const addUser = async () => {
    if (!userForm.username) {
      alertError('用户名不能为空')
      return
    }
    if (!userForm.fullName) {
      alertError('姓名不能为空')
      return
    }
    if (!userForm.password || !userForm.nextPwd || userForm.password !== userForm.nextPwd) {
      alertError('两次输入的密码不一致！')
      return
    }
    const data = {
      username: userForm.username,
      fullName: userForm.fullName,
      phone: userForm.phone,
      email: userForm.email,
      password: userForm.password,
      departmentId: selectedContacts.id,
      role: userForm.role || null,
      quota: `${userForm.quota} ${userForm.quotaUnit}`
    }
    try {
      const resp = await http.post('/api/user/add', data)
      if (resp.data.code) {
        setDialogVisible(false)
        refreshUser()
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.message || '添加用户失败')
    }
  }

  const modifyUser = async () => {
    if (!userForm.fullName) {
      alertError('姓名不能为空')
      return
    }
    const data = {
      id: userForm.id,
      username: userForm.username,
      fullName: userForm.fullName,
      phone: userForm.phone,
      email: userForm.email,
      departmentId: selectedContacts.id,
      deptName: userForm.deptName,
      role: userForm.role || null,
      quota: `${userForm.quota} ${userForm.quotaUnit}`
    }
    const resp = await http.post('/api/user/update', data)
    if (resp.data.code) {
      setDialogVisible(false)
      refreshUser()
    } else {
      alertError(resp.data.message)
    }
  }

  const deleteUserByList = async (userList) => {
    if (userList.length === 0) {
      alertError('请选择要删除的用户')
      return
    }
    const confirmMsg = userList.length === 1
      ? `确认删除用户 "${userList[0].username}" 吗？`
      : `确认删除选中的 ${userList.length} 个用户吗？`
    const confirmed = window.confirm(confirmMsg)
    if (!confirmed) return
    const resp = await http.post('/api/user/delete', userList)
    if (resp.data.code) {
      alertSuccess(resp.data.message)
      setSelectedUsers([])
      refreshUser()
    } else {
      alertError(resp.data.message)
    }
  }

  const deleteUser = () => deleteUserByList(selectedUsers)

  const enableUserByList = async (userList) => {
    const users = userList.filter((item) => item.status === 2)
    if (users.length === 0) {
      alertError('请选择要启用的用户')
      return
    }
    const resp = await http.post('/api/user/enable', users)
    if (resp.data.code) {
      alertSuccess(resp.data.message)
      refreshUser()
    } else {
      alertError(resp.data.message)
    }
  }

  const disableUserByList = async (userList) => {
    const users = userList.filter((item) => item.status !== 2)
    if (users.length === 0) {
      alertError('请选择要禁用的用户')
      return
    }
    const resp = await http.post('/api/user/disable', users)
    if (resp.data.code) {
      alertSuccess(resp.data.message)
      refreshUser()
    } else {
      alertError(resp.data.message)
    }
  }

  const enableUser = () => enableUserByList(selectedUsers)
  const disableUser = () => disableUserByList(selectedUsers)

  const toggleSelection = (row) => {
    setSelectedUsers((prev) => {
      const exists = prev.find((item) => item.id === row.id)
      if (exists) return prev.filter((item) => item.id !== row.id)
      return [...prev, row]
    })
  }

  const isSelected = (row) => selectedUsers.some((item) => item.id === row.id)

  const isAllSelected = userList.length > 0 && selectedUsers.length === userList.length

  const toggleSelectAll = () => {
    if (isAllSelected) {
      setSelectedUsers([])
    } else {
      setSelectedUsers([...userList])
    }
  }

  // 右键菜单处理
  const handleContextMenu = (e, user) => {
    e.preventDefault()
    e.stopPropagation()
    setContextMenu({
      visible: true,
      x: e.clientX,
      y: e.clientY,
      user
    })
  }

  const closeContextMenu = () => {
    setContextMenu({ visible: false, x: 0, y: 0, user: null })
  }

  // 点击其他地方关闭菜单
  useEffect(() => {
    const handleClick = () => closeContextMenu()
    if (contextMenu.visible) {
      document.addEventListener('click', handleClick)
      return () => document.removeEventListener('click', handleClick)
    }
  }, [contextMenu.visible])

  // 右键菜单 - 修改用户
  const handleEditFromContext = () => {
    const user = contextMenu.user
    closeContextMenu()
    const [quotaValue, quotaUnit] = (user.quota || '1 GB').split(' ')
    setTitle('修改')
    setUserForm({
      ...user,
      password: '',
      nextPwd: '',
      quota: quotaValue,
      quotaUnit: quotaUnit || 'GB',
      role: user.role || null
    })
    setDialogVisible(true)
  }

  // Helper to get role display name
  const getRoleDisplayName = (role) => {
    if (!role) return '-'
    return role
  }

  // 右键菜单 - 修改密码
  const handleChangePassword = () => {
    const user = contextMenu.user
    closeContextMenu()
    setPwdUser(user)
    setPwdForm({ password: '', nextPwd: '' })
    setPwdDialogVisible(true)
  }

  const savePassword = async () => {
    if (!pwdForm.password || !pwdForm.nextPwd) {
      alertError('请输入密码')
      return
    }
    if (pwdForm.password !== pwdForm.nextPwd) {
      alertError('两次输入的密码不一致！')
      return
    }
    try {
      const resp = await http.post('/api/user/reset-password', {
        id: pwdUser.id,
        username: pwdUser.username,
        password: pwdForm.password
      })
      if (resp.data.code) {
        alertSuccess('密码修改成功')
        setPwdDialogVisible(false)
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.message || '修改密码失败')
    }
  }

  // 右键菜单 - 删除用户
  const handleDeleteFromContext = () => {
    const user = contextMenu.user
    closeContextMenu()
    deleteUserByList([user])
  }

  // 右键菜单 - 启用用户
  const handleEnableFromContext = () => {
    const user = contextMenu.user
    closeContextMenu()
    enableUserByList([user])
  }

  // 右键菜单 - 禁用用户
  const handleDisableFromContext = () => {
    const user = contextMenu.user
    closeContextMenu()
    disableUserByList([user])
  }

  return (
    <div className="user-manager">
      <div className="user-actions">
        <div className="toolbar-segment">
          <button type="button" className="segment-btn" onClick={preAdd}>
            <Plus className="mr-1 h-3.5 w-3.5" />
            添加
          </button>
          <button type="button" className="segment-btn" onClick={enableUser}>
            <Check className="mr-1 h-3.5 w-3.5" />
            启用
          </button>
          <button type="button" className="segment-btn" onClick={disableUser}>
            <Ban className="mr-1 h-3.5 w-3.5" />
            禁用
          </button>
          <button type="button" className="segment-btn danger" onClick={deleteUser}>
            <Trash2 className="mr-1 h-3.5 w-3.5" />
            删除
          </button>
          <button type="button" className="segment-btn" onClick={refreshUser}>
            <RefreshCw className="mr-1 h-3.5 w-3.5" />
            刷新
          </button>
        </div>
      </div>
      <div className="user-table">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[60px]">
                <input
                  type="checkbox"
                  checked={isAllSelected}
                  onChange={toggleSelectAll}
                />
              </TableHead>
              <TableHead>用户名</TableHead>
              <TableHead>姓名</TableHead>
              <TableHead>部门</TableHead>
              <TableHead className="w-[100px]">角色</TableHead>
              <TableHead className="w-[200px]">手机号码</TableHead>
              <TableHead className="w-[200px]">电子邮箱</TableHead>
              <TableHead className="w-[100px]">配额</TableHead>
              <TableHead className="w-[100px]">状态</TableHead>
              <TableHead className="w-[150px]">最后登录时间</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {userList.map((row) => (
              <TableRow
                key={row.id}
                className={`${isSelected(row) ? 'selected' : ''} ${contextMenu.visible && contextMenu.user?.id === row.id ? 'context-active' : ''}`}
                onContextMenu={(e) => handleContextMenu(e, row)}
              >
                <TableCell>
                  <input
                    type="checkbox"
                    checked={isSelected(row)}
                    onChange={() => toggleSelection(row)}
                  />
                </TableCell>
                <TableCell>{row.username}</TableCell>
                <TableCell>{row.fullName}</TableCell>
                <TableCell>{row.deptName}</TableCell>
                <TableCell>{getRoleDisplayName(row.role)}</TableCell>
                <TableCell>{row.phone}</TableCell>
                <TableCell>{row.email}</TableCell>
                <TableCell>{row.quota}</TableCell>
                <TableCell>{row.status === 0 ? '未登录' : row.status === 1 ? '正常' : row.status === 2 ? '禁用' : '未知'}</TableCell>
                <TableCell>{row.lastLogin ? new Date(row.lastLogin * 1000).toLocaleString('zh-CN') : '-'}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <Dialog open={dialogVisible} onOpenChange={setDialogVisible}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
          </DialogHeader>
          <div className="user-form-grid">
            <label>用户名</label>
            <Input
              value={userForm.username}
              disabled={title === '修改'}
              onChange={(event) => setUserForm((prev) => ({ ...prev, username: event.target.value }))}
            />
            <label>姓名</label>
            <Input
              value={userForm.fullName}
              onChange={(event) => setUserForm((prev) => ({ ...prev, fullName: event.target.value }))}
            />
            <label>邮箱</label>
            <Input
              value={userForm.email}
              onChange={(event) => setUserForm((prev) => ({ ...prev, email: event.target.value }))}
            />
            <label>电话</label>
            <Input
              value={userForm.phone}
              onChange={(event) => setUserForm((prev) => ({ ...prev, phone: event.target.value }))}
            />
            {title === '添加' && (
              <>
                <label>密码</label>
                <Input
                  type="password"
                  value={userForm.password}
                  onChange={(event) => setUserForm((prev) => ({ ...prev, password: event.target.value }))}
                />
                <label>确认密码</label>
                <Input
                  type="password"
                  value={userForm.nextPwd}
                  onChange={(event) => setUserForm((prev) => ({ ...prev, nextPwd: event.target.value }))}
                />
              </>
            )}
            <label>角色</label>
            <Select
              value={userForm.role || 'none'}
              onValueChange={(value) =>
                setUserForm((prev) => ({
                  ...prev,
                  role: value === 'none' ? null : value
                }))
              }
            >
              <SelectTrigger>
                <SelectValue placeholder="选择角色" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">无角色</SelectItem>
                {roles.map((role) => (
                  <SelectItem key={role.name} value={role.name}>
                    {role.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <label>配额</label>
            <div className="quota-row">
              <Input
                type="number"
                value={userForm.quota}
                onChange={(event) => setUserForm((prev) => ({ ...prev, quota: event.target.value }))}
              />
              <select
                value={userForm.quotaUnit}
                onChange={(event) => setUserForm((prev) => ({ ...prev, quotaUnit: event.target.value }))}
              >
                <option value="GB">GB</option>
                <option value="MB">MB</option>
              </select>
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

      {/* 修改密码对话框 */}
      <Dialog open={pwdDialogVisible} onOpenChange={setPwdDialogVisible}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>修改密码 - {pwdUser?.username}</DialogTitle>
          </DialogHeader>
          <div className="user-form-grid">
            <label>新密码</label>
            <Input
              type="password"
              value={pwdForm.password}
              onChange={(event) => setPwdForm((prev) => ({ ...prev, password: event.target.value }))}
            />
            <label>确认密码</label>
            <Input
              type="password"
              value={pwdForm.nextPwd}
              onChange={(event) => setPwdForm((prev) => ({ ...prev, nextPwd: event.target.value }))}
            />
          </div>
          <DialogFooter>
            <Button variant="secondary" onClick={() => setPwdDialogVisible(false)}>
              取消
            </Button>
            <Button onClick={savePassword}>保存</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 右键菜单 */}
      {contextMenu.visible && (
        <div
          className="user-context-menu"
          style={{
            position: 'fixed',
            left: contextMenu.x,
            top: contextMenu.y,
            zIndex: 1000
          }}
        >
          <div className="user-context-menu-item" onClick={handleEditFromContext}>
            <Pencil className="h-4 w-4 mr-2" />
            修改用户信息
          </div>
          <div className="user-context-menu-item" onClick={handleChangePassword}>
            <KeyRound className="h-4 w-4 mr-2" />
            修改密码
          </div>
          {contextMenu.user?.status === 2 ? (
            <div className="user-context-menu-item" onClick={handleEnableFromContext}>
              <Check className="h-4 w-4 mr-2" />
              启用
            </div>
          ) : (
            <div className="user-context-menu-item" onClick={handleDisableFromContext}>
              <Ban className="h-4 w-4 mr-2" />
              禁用
            </div>
          )}
          <div className="user-context-menu-item danger" onClick={handleDeleteFromContext}>
            <Trash2 className="h-4 w-4 mr-2" />
            删除
          </div>
        </div>
      )}
    </div>
  )
}

export default UserManager
