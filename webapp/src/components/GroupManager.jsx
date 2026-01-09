import { MoreHorizontal, Plus, RefreshCw, Trash2 } from 'lucide-react'
import React, { useEffect, useRef, useState } from 'react'
import http from '../lib/http'
import { alertError } from '../lib/utils'
import { useGroups } from '../store/providers'
import { Button } from './ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from './ui/dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger
} from './ui/dropdown-menu'
import { Input } from './ui/input'
import { Table, TableBody, TableCell, TableRow } from './ui/table'
import './GroupManager.css'

const GroupManager = () => {
  const { selectedGroups, setSelectedGroups } = useGroups()
  const [groups, setGroups] = useState([])
  const [dialogVisible, setDialogVisible] = useState(false)
  const [groupForm, setGroupForm] = useState({ name: '' })
  const [visibleCount, setVisibleCount] = useState(3)
  const containerRef = useRef(null)

  const refreshGroups = async () => {
    const resp = await http.get('/api/group/query')
    setGroups(resp.data.data || [])
    setSelectedGroups(null)
  }

  useEffect(() => {
    refreshGroups()
  }, [])

  const save = async () => {
    try {
      const resp = await http.post('/api/group/add', groupForm)
      if (resp.data.code) {
        setDialogVisible(false)
        setGroupForm({ name: '' })
        refreshGroups()
      } else {
        alertError(resp.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.error || '保存失败')
    }
  }

  const deleteGroup = async () => {
    if (!selectedGroups) {
      alertError('请先选择群组')
      return
    }
    const confirmed = window.confirm('确认删除该群组吗？')
    if (!confirmed) return
    const resp = await http.post(`/api/group/delete?id=${selectedGroups.id}`)
    if (resp.data.code) {
      refreshGroups()
    } else {
      alertError(resp.data.message)
    }
  }

  const preAdd = () => {
    setGroupForm({ name: '' })
    setDialogVisible(true)
  }

  const actions = [
    { key: 'add', label: '添加', icon: Plus, onClick: preAdd },
    { key: 'delete', label: '删除', icon: Trash2, onClick: deleteGroup, danger: true },
    { key: 'refresh', label: '刷新', icon: RefreshCw, onClick: refreshGroups }
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

  return (
    <div className="group-manager">
      <div className="group-actions" ref={containerRef}>
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
      <div className="group-table">
        <Table>
          <TableBody>
            {groups.map((row) => (
              <TableRow
                key={row.id}
                className={selectedGroups?.id === row.id ? 'selected' : ''}
                onClick={() => setSelectedGroups(row)}
              >
                <TableCell>{row.name}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <Dialog open={dialogVisible} onOpenChange={setDialogVisible}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>添加</DialogTitle>
          </DialogHeader>
          <Input value={groupForm.name} onChange={(event) => setGroupForm({ name: event.target.value })} />
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

export default GroupManager
