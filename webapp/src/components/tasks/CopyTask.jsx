import React, { useEffect, useMemo, useRef, useState } from 'react'
import { Pause, Play, X } from 'lucide-react'
import http from '../../lib/http'
import { alertError, alertSuccess } from '../../lib/utils'
import { Badge } from '../ui/badge'
import { Button } from '../ui/button'
import { Card } from '../ui/card'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '../ui/dialog'
import { Checkbox } from '../ui/checkbox'
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/tooltip'
import { Progress } from '../ui/progress'
import './CopyTask.css'

const statusFormatter = (row) => {
  switch (row.status) {
    case 'pending':
      return '等待中'
    case 'starting':
      return '启动中'
    case 'running':
      return '运行中'
    case 'suspended':
      return '暂停'
    case 'completed':
      return '已完成'
    case 'cancelled':
      return '已取消'
    case 'failed':
      return '失败'
    default:
      return '未知'
  }
}

const statusTagType = (status) => {
  switch (status) {
    case 'running':
      return 'default'
    case 'suspended':
    case 'pending':
    case 'starting':
      return 'secondary'
    case 'failed':
      return 'destructive'
    default:
      return 'outline'
  }
}

const formatDate = (timestamp) => {
  if (!timestamp) return '-'
  return new Date(timestamp * 1000).toLocaleString()
}

const formatFileSize = (size) => {
  if (!size && size !== 0) return '-'
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(2)} KB`
  if (size < 1024 * 1024 * 1024) return `${(size / (1024 * 1024)).toFixed(2)} MB`
  return `${(size / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

const shortPath = (value, limit = 46) => {
  if (!value) return '-'
  if (value.length <= limit) return value
  const keep = Math.floor((limit - 1) / 2)
  return `${value.slice(0, keep)}…${value.slice(-keep)}`
}

const calculateRunningTime = (taskRow) => {
  const startedAt = taskRow.startedAt
  const updatedAt = taskRow.updatedAt
  if (!startedAt || !updatedAt) return '0s'
  const diff = Math.max(0, updatedAt - startedAt)
  const minutes = Math.floor(diff / 60)
  const seconds = Math.floor(diff % 60)
  if (minutes === 0) return `${seconds}s`
  return `${minutes}m ${seconds}s`
}

const displayProgress = (taskRow) => {
  if (taskRow.progress !== undefined && taskRow.progress !== null) {
    return Math.min(100, Math.max(0, Math.round(taskRow.progress)))
  }
  if (taskRow.totalSize) {
    const ratio = (taskRow.copiedSize || 0) / taskRow.totalSize
    return Math.min(100, Math.max(0, Math.round(ratio * 100)))
  }
  return 0
}

const CopyTask = ({ task, onTaskDeleted, onTaskUpdated }) => {
  const [conflictDialogVisible, setConflictDialogVisible] = useState(false)
  const [rememberChoice, setRememberChoice] = useState(false)
  const prevStatusRef = useRef(task.status)

  const sourceWithFile = useMemo(() => {
    const firstFile = task.files && task.files.length > 0 ? task.files[0] : ''
    const dir = task.source || ''
    if (!dir && !firstFile) return '-'
    if (!firstFile) return shortPath(dir)
    const trimmed = dir.endsWith('/') ? dir.slice(0, -1) : dir
    const combined = trimmed ? `${trimmed}/${firstFile}` : `/${firstFile}`
    return shortPath(combined)
  }, [task.files, task.source])

  useEffect(() => {
    if (task.conflictInfo?.needConfirm) {
      setRememberChoice(false)
      setConflictDialogVisible(true)
    }
  }, [task.conflictInfo])

  useEffect(() => {
    const prev = prevStatusRef.current
    if (task.status === 'completed' && prev !== 'completed') {
      alertSuccess(`${task.isCopy ? '复制' : '剪切'}已完成`)
    } else if (task.status === 'failed' && prev !== 'failed') {
      alertError(`${task.isCopy ? '复制' : '剪切'}失败`)
    } else if (task.status === 'cancelled' && prev !== 'cancelled') {
      alertSuccess('任务已取消')
    }
    prevStatusRef.current = task.status
  }, [task.status, task.isCopy])

  const pauseTask = async (taskId) => {
    try {
      await http.post('/api/task/suspend', null, { params: { id: taskId } })
      onTaskUpdated?.()
    } catch (error) {
      console.error('Error suspending task:', error)
      alertError('暂停任务失败')
    }
  }

  const resumeTask = async (taskId) => {
    try {
      await http.post('/api/task/resume', null, { params: { id: taskId } })
      onTaskUpdated?.()
    } catch (error) {
      console.error('Error resuming task:', error)
      alertError('恢复任务失败')
    }
  }

  const cancelTask = async (taskId) => {
    try {
      await http.post('/api/task/cancel', null, { params: { id: taskId } })
      alertSuccess('任务已取消')
      onTaskUpdated?.()
    } catch (error) {
      console.error('Error cancelling task:', error)
      alertError('取消任务失败')
    }
  }

  const deleteTask = async (taskId) => {
    try {
      await http.delete('/api/task/delete', { params: { id: taskId } })
      alertSuccess('任务已删除')
      onTaskDeleted?.(taskId)
    } catch (error) {
      console.error('Error deleting task:', error)
      alertError(`删除任务失败: ${error.response?.data?.error || error.message}`)
    }
  }

  const handleConflict = async (policy) => {
    try {
      await http.post('/api/file/copy/conflict', {
        taskId: task.id,
        policy,
        remember: rememberChoice
      })
      setConflictDialogVisible(false)
      onTaskUpdated?.()
    } catch (error) {
      console.error('Error handling conflict:', error)
      alertError('处理冲突失败')
    }
  }

  return (
    <>
      <Card className="task-card">
        <div className="task-row task-row--header">
          <div className="task-title">
            <span className="task-chip">{task.isCopy ? '复制' : '剪切'}</span>
            <Badge variant={statusTagType(task.status)}>{statusFormatter(task)}</Badge>
          </div>
          <div className="task-actions">
            {task.status === 'running' && (
              <Button variant="secondary" size="icon" onClick={() => pauseTask(task.id)}>
                <Pause className="h-4 w-4" />
              </Button>
            )}
            {task.status === 'suspended' && (
              <Button variant="default" size="icon" onClick={() => resumeTask(task.id)}>
                <Play className="h-4 w-4" />
              </Button>
            )}
            {['running', 'suspended', 'starting', 'pending'].includes(task.status) && (
              <Button variant="destructive" size="icon" onClick={() => cancelTask(task.id)}>
                <X className="h-4 w-4" />
              </Button>
            )}
            {['completed', 'failed', 'cancelled'].includes(task.status) && (
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="outline"
                      size="icon"
                      className="task-delete-button rounded-full h-6 w-6"
                      onClick={() => deleteTask(task.id)}
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>删除</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            )}
          </div>
        </div>

        <div className="task-subline">更新: {formatDate(task.updatedAt)}</div>

        <div className="task-paths">
          <div className="task-path-row">
            <span className="label">源</span>
            <Popover>
              <PopoverTrigger asChild>
                <span className="mono cursor-pointer">{sourceWithFile}</span>
              </PopoverTrigger>
              <PopoverContent className="w-[520px]">
                <p className="section-title">源目录</p>
                <div className="section-body">{task.source || '未提供路径'}</div>
                <p className="section-title">源文件</p>
                <div className="section-body">
                  {(task.files || []).slice(0, 8).map((file) => (
                    <div key={file} className="mono">
                      {file}
                    </div>
                  ))}
                  {(task.files || []).length > 8 && <div className="mono">...</div>}
                </div>
              </PopoverContent>
            </Popover>
          </div>
          <div className="task-path-row">
            <span className="label">目标</span>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="mono cursor-pointer">{shortPath(task.target)}</span>
                </TooltipTrigger>
                <TooltipContent>
                  <div className="mono">{task.target}</div>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
        </div>

        <div className="task-progress-block">
          <div className="task-progress-meta">
            <span>文件 {task.copiedFiles || 0}/{task.totalFiles || 0}</span>
            <span>耗时 {calculateRunningTime(task)}</span>
          </div>
          <Progress value={displayProgress(task)} />
        </div>

        {task.error && (
          <div className="task-error">
            <strong>错误:</strong>
            <span>{task.error}</span>
          </div>
        )}
      </Card>

      <Dialog open={conflictDialogVisible} onOpenChange={setConflictDialogVisible}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>文件冲突</DialogTitle>
          </DialogHeader>
          {task.conflictInfo && (
            <div>
              <div className="mb-4">
                <h3 className="font-medium mb-2">源文件:</h3>
                <div className="bg-slate-50 p-2 rounded">
                  <p>名称: {task.conflictInfo.srcFile.name}</p>
                  <p>大小: {formatFileSize(task.conflictInfo.srcFile.size)}</p>
                  <p>修改时间: {formatDate(task.conflictInfo.srcFile.modifyTime)}</p>
                </div>
              </div>
              <div className="mb-4">
                <h3 className="font-medium mb-2">目标文件:</h3>
                <div className="bg-slate-50 p-2 rounded">
                  <p>名称: {task.conflictInfo.dstFile.name}</p>
                  <p>大小: {formatFileSize(task.conflictInfo.dstFile.size)}</p>
                  <p>修改时间: {formatDate(task.conflictInfo.dstFile.modifyTime)}</p>
                </div>
              </div>
              <div className="mb-4 flex items-center gap-2">
                <Checkbox
                  id={`remember-${task.id}`}
                  checked={rememberChoice}
                  onCheckedChange={(value) => setRememberChoice(Boolean(value))}
                />
                <label htmlFor={`remember-${task.id}`}>记住我的选择</label>
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant="secondary" onClick={() => handleConflict('skip')}>
              跳过
            </Button>
            <Button variant="secondary" onClick={() => handleConflict('rename')}>
              重命名
            </Button>
            <Button variant="outline" onClick={() => handleConflict('overwrite')}>
              覆盖
            </Button>
            <Button variant="destructive" onClick={() => handleConflict('abort')}>
              中止
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}

export default CopyTask
