import { ChevronDown, ChevronRight, Folder, Plus, RefreshCw, Trash2, UserRound } from 'lucide-react'
import React, { useEffect, useMemo, useState } from 'react'
import http from '../lib/http'
import { alertError } from '../lib/utils'
import { useGroups } from '../store/providers'
import { Button } from './ui/button'
import { Checkbox } from './ui/checkbox'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from './ui/dialog'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from './ui/table'
import './GroupUserManager.css'

// Build tree structure from flat data
const buildDeptUserTree = (items) => {
  const deptMap = new Map()
  const usersByDept = new Map()

  // Separate departments and users
  items.forEach((item) => {
    if (item.isDept) {
      deptMap.set(item.id, { ...item, children: [], users: [] })
    } else {
      const deptId = item.parentId
      if (!usersByDept.has(deptId)) {
        usersByDept.set(deptId, [])
      }
      usersByDept.get(deptId).push(item)
    }
  })

  // Attach users to departments
  usersByDept.forEach((users, deptId) => {
    if (deptMap.has(deptId)) {
      deptMap.get(deptId).users = users
    }
  })

  // Build department tree
  const roots = []
  deptMap.forEach((dept) => {
    if (dept.parentId && deptMap.has(dept.parentId)) {
      deptMap.get(dept.parentId).children.push(dept)
    } else {
      roots.push(dept)
    }
  })

  return roots
}

// Get all users in a department and its sub-departments
const getAllUsersInDept = (dept) => {
  let users = [...(dept.users || [])]
  ;(dept.children || []).forEach((child) => {
    users = users.concat(getAllUsersInDept(child))
  })
  return users
}

// Tree node component for department and user selection
const DeptUserTreeNode = ({ node, level, expandedKeys, selectedUserIds, onToggle, onSelectUser, onSelectDept }) => {
  const hasChildren = (node.children && node.children.length > 0) || (node.users && node.users.length > 0)
  const isExpanded = expandedKeys.has(node.id)

  const allUsers = getAllUsersInDept(node)
  const selectedCount = allUsers.filter((u) => selectedUserIds.has(u.id)).length
  const isAllSelected = allUsers.length > 0 && selectedCount === allUsers.length
  const isPartialSelected = selectedCount > 0 && selectedCount < allUsers.length

  return (
    <>
      <div className="dept-user-tree-node" style={{ paddingLeft: `${level * 18 + 8}px` }}>
        <button
          type="button"
          className="dept-user-tree-toggle"
          onClick={() => hasChildren && onToggle(node.id)}
          style={{ visibility: hasChildren ? 'visible' : 'hidden' }}
        >
          {isExpanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        </button>
        {allUsers.length > 0 && (
          <Checkbox
            checked={isAllSelected}
            className={isPartialSelected ? 'partial' : ''}
            onCheckedChange={() => onSelectDept(node, allUsers, isAllSelected)}
          />
        )}
        <Folder className="h-4 w-4 text-amber-500 flex-shrink-0" />
        <span className="dept-user-tree-label dept">{node.name}</span>
        {allUsers.length > 0 && (
          <span className="dept-user-count">
            ({selectedCount}/{allUsers.length})
          </span>
        )}
      </div>
      {hasChildren && isExpanded && (
        <div className="dept-user-tree-children">
          {node.children.map((child) => (
            <DeptUserTreeNode
              key={`dept_${child.id}`}
              node={child}
              level={level + 1}
              expandedKeys={expandedKeys}
              selectedUserIds={selectedUserIds}
              onToggle={onToggle}
              onSelectUser={onSelectUser}
              onSelectDept={onSelectDept}
            />
          ))}
          {node.users.map((user) => (
            <div
              key={`user_${user.id}`}
              className="dept-user-tree-node user"
              style={{ paddingLeft: `${(level + 1) * 18 + 8}px` }}
            >
              <span className="dept-user-tree-toggle" style={{ visibility: 'hidden' }} />
              <Checkbox checked={selectedUserIds.has(user.id)} onCheckedChange={() => onSelectUser(user)} />
              <UserRound className="h-4 w-4 text-blue-500 flex-shrink-0" />
              <span className="dept-user-tree-label">{user.name}</span>
            </div>
          ))}
        </div>
      )}
    </>
  )
}

const GroupUserManager = () => {
  const { selectedGroups } = useGroups()
  const [userList, setUserList] = useState([])
  const [contacts, setContacts] = useState([])
  const [dialogVisible, setDialogVisible] = useState(false)
  const [selectedUsers, setSelectedUsers] = useState([])
  const [selectedUserIds, setSelectedUserIds] = useState(new Set())
  const [expandedKeys, setExpandedKeys] = useState(new Set())
  const showBtn = selectedGroups?.owner

  // Build tree from contacts
  const treeData = useMemo(() => buildDeptUserTree(contacts), [contacts])

  const refreshUser = async () => {
    if (!selectedGroups?.id) {
      setUserList([])
      return
    }
    const resp = await http.get('/api/group/query/users', { params: { groupId: selectedGroups.id } })
    setUserList(resp.data.data || [])
  }

  useEffect(() => {
    if (selectedGroups) {
      refreshUser()
    } else {
      setUserList([])
    }
  }, [selectedGroups?.id])

  const preAdd = async () => {
    if (!selectedGroups) {
      alertError('请先选择一个群组')
      return
    }
    setDialogVisible(true)
    setSelectedUserIds(new Set())
    setExpandedKeys(new Set())
    try {
      const resp = await http.get('/api/department/query/all')
      setContacts(resp.data.data || [])
    } catch (error) {
      alertError(error.response?.data?.error || '加载联系人失败')
    }
  }

  const save = async () => {
    const ids = Array.from(selectedUserIds)
    if (ids.length === 0) {
      alertError('请选择一个或多个用户')
      return
    }
    try {
      const resp = await http.post(`/api/group/addUsers?groupId=${selectedGroups.id}`, ids)
      if (resp.data.code) {
        setDialogVisible(false)
        refreshUser()
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.error || '添加用户失败')
    }
  }

  const deleteUser = async () => {
    if (selectedUsers.length === 0) {
      alertError('请选择一个或多个用户')
      return
    }
    const confirmed = window.confirm('确认从群组中删除该用户吗？')
    if (!confirmed) return
    try {
      const ids = selectedUsers.map((item) => item.id)
      const resp = await http.post(`/api/group/deleteUsers?groupId=${selectedGroups.id}`, ids)
      if (resp.data.code) {
        refreshUser()
        setSelectedUsers([])
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.error || '删除失败')
    }
  }

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

  const handleSelectUser = (user) => {
    setSelectedUserIds((prev) => {
      const next = new Set(prev)
      if (next.has(user.id)) {
        next.delete(user.id)
      } else {
        next.add(user.id)
      }
      return next
    })
  }

  const handleSelectDept = (_dept, allUsers, isAllSelected) => {
    setSelectedUserIds((prev) => {
      const next = new Set(prev)
      if (isAllSelected) {
        // Deselect all
        allUsers.forEach((u) => next.delete(u.id))
      } else {
        // Select all
        allUsers.forEach((u) => next.add(u.id))
      }
      return next
    })
  }

  const toggleSelection = (row, _selected, setSelected) => {
    setSelected((prev) => {
      const exists = prev.find((item) => item.id === row.id)
      if (exists) return prev.filter((item) => item.id !== row.id)
      return [...prev, row]
    })
  }

  const isSelected = (row, selected) => selected.some((item) => item.id === row.id)

  // Check if all users are selected
  const isAllUsersSelected = userList.length > 0 && selectedUsers.length === userList.length

  // Toggle select all
  const toggleSelectAll = () => {
    if (isAllUsersSelected) {
      setSelectedUsers([])
    } else {
      setSelectedUsers([...userList])
    }
  }

  return (
    <div className="group-user-manager">
      {showBtn && (
        <div className="group-user-actions">
          <div className="toolbar-segment">
            <button type="button" className="segment-btn" onClick={preAdd}>
              <Plus className="mr-1 h-3.5 w-3.5" />
              添加
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
      )}
      <div className="group-user-table">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[40px]">
                <input
                  type="checkbox"
                  checked={isAllUsersSelected}
                  onChange={toggleSelectAll}
                  disabled={userList.length === 0}
                />
              </TableHead>
              <TableHead>用户名</TableHead>
              <TableHead>姓名</TableHead>
              <TableHead>手机号码</TableHead>
              <TableHead>电子邮箱</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {userList.map((row) => (
              <TableRow key={row.id} className={isSelected(row, selectedUsers) ? 'selected' : ''}>
                <TableCell>
                  <input
                    type="checkbox"
                    checked={isSelected(row, selectedUsers)}
                    onChange={() => toggleSelection(row, selectedUsers, setSelectedUsers)}
                  />
                </TableCell>
                <TableCell>{row.username}</TableCell>
                <TableCell>{row.fullName}</TableCell>
                <TableCell>{row.phone}</TableCell>
                <TableCell>{row.email}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <Dialog open={dialogVisible} onOpenChange={setDialogVisible}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>添加用户到群组</DialogTitle>
          </DialogHeader>
          <div className="dept-user-tree-container">
            {treeData.map((node) => (
              <DeptUserTreeNode
                key={`dept_${node.id}`}
                node={node}
                level={0}
                expandedKeys={expandedKeys}
                selectedUserIds={selectedUserIds}
                onToggle={toggleExpand}
                onSelectUser={handleSelectUser}
                onSelectDept={handleSelectDept}
              />
            ))}
          </div>
          <div className="selected-count">已选择 {selectedUserIds.size} 个用户</div>
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

export default GroupUserManager
