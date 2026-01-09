import React, { useEffect, useMemo, useRef, useState } from 'react'
import { Clock } from 'lucide-react'
import http from '../lib/http'
import { useTasks } from '../store/providers'
import { Badge } from './ui/badge'
import { Button } from './ui/button'
import { Popover, PopoverContent, PopoverTrigger } from './ui/popover'
import { ScrollArea } from './ui/scroll-area'
import CopyTask from './tasks/CopyTask'
import './TaskManager.css'

const TaskManager = () => {
  const { tasks, activeTasks, updateTasks, deleteTask } = useTasks()
  const [refreshing, setRefreshing] = useState(false)
  const [pulse, setPulse] = useState(false)
  const prevActiveRef = useRef(0)

  const taskCounts = useMemo(
    () => ({
      active: activeTasks.length,
      total: tasks.length
    }),
    [activeTasks.length, tasks.length]
  )

  const refreshTasks = async () => {
    if (refreshing) return
    try {
      setRefreshing(true)
      const { data } = await http.get('/api/task/query')
      updateTasks(data)
    } catch (error) {
      console.error('Error fetching tasks:', error)
    } finally {
      setRefreshing(false)
    }
  }

  useEffect(() => {
    refreshTasks()
  }, [])

  useEffect(() => {
    const prevActive = prevActiveRef.current
    if (taskCounts.active > prevActive) {
      setPulse(true)
      const timer = setTimeout(() => setPulse(false), 650)
      prevActiveRef.current = taskCounts.active
      return () => clearTimeout(timer)
    }
    prevActiveRef.current = taskCounts.active
  }, [taskCounts.active])

  return (
    <Popover onOpenChange={(open) => open && refreshTasks()}>
      <PopoverTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className={`task-trigger rounded-full ${
            taskCounts.active > 0 ? 'task-trigger--active' : ''
          } ${pulse ? 'task-trigger--pulse' : ''}`}
        >
          <Clock className="h-4 w-4" />
          {taskCounts.active > 0 && (
            <span className="task-trigger-count">{taskCounts.active}</span>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        className="task-popover border-0 shadow-none"
        style={{
          width: 600,
          border: '0.5px solid rgba(148, 163, 184, 0.25)',
          boxShadow: '0 24px 60px rgba(15, 23, 42, 0.22)',
          padding: '5px'
        }}
      >
        <div className="task-header">
          <div>
            <div className="task-title">
              任务 <Badge variant="secondary">{taskCounts.total}</Badge>
            </div>
            <div className="task-subtitle">进行中 {taskCounts.active}</div>
          </div>
          <Button size="sm" variant="ghost" onClick={refreshTasks} disabled={refreshing}>
            刷新
          </Button>
        </div>
        {tasks.length === 0 ? (
          <div className="task-empty">无任务</div>
        ) : (
          <ScrollArea className="task-list">
            {tasks.map((task) => (
              <div key={task.id} className="task-card">
                <CopyTask task={task} onTaskDeleted={deleteTask} onTaskUpdated={refreshTasks} />
              </div>
            ))}
          </ScrollArea>
        )}
      </PopoverContent>
    </Popover>
  )
}

export default TaskManager
