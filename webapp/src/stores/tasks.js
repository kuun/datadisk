import { ref, computed } from 'vue'
import { defineStore } from 'pinia'

export const useTaskStore = defineStore('tasks', () => {
  const tasks = ref([
/*     {
      id: '1',
      name: 'Task 1',
      status: 'failed',
      createdAt: 1741682942,
      startedAt: 1741683942,
      updatedAt: 1741684942,
      type: 'copy',
      isCopy: true,
      source: '/path/to/source',
      target: '/path/to/destination',
      files: ['file1', 'file2'],
      progress: 50,
      currentFile: 'file1',
      currentFileSize: 1024,
      currentFileCopiedSize: 512,
      totalFiles : 2,
      copiedFiles: 1,
      totalSize: 10240,
      copiedSize: 512,
      error: '无法复制文件'
    } */
  ])
  // 显示在对话框中的任务
  const currentTask = ref(null)

  const allTasks = computed(() => tasks.value)

  const completedTasks = computed(
    () => tasks.value.filter(task => task.status === 'completed')
  );

  const activeTasks = computed(
    () => tasks.value.filter(task => {
      return task.status !== 'completed' && task.status !== 'cancelled' && task.status !== 'failed'
    })
  )

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

  function updateTasks(newTasks) {
    newTasks.forEach(newTask => {
      const index = tasks.value.findIndex(task => task.id === newTask.id)
      newTask.progress = calcProgress(newTask)
      if (index !== -1) {
        // 保留原有任务的引用，只更新内容
        Object.assign(tasks.value[index], newTask)
      } else {
        tasks.value.push({...newTask})
      }
    })

    // 更新排序
    tasks.value = [...tasks.value].sort((a, b) => {
      const statusOrder = {
        running: 1,
        suspended: 1,
        pending: 2,
        starting: 3,
        completed: 4,
        cancelled: 4,
        failed: 4,
      }

      if (statusOrder[a.status] !== statusOrder[b.status]) {
        return statusOrder[a.status] - statusOrder[b.status]
      }

      if (a.status === 'running' || a.status === 'suspended' || a.status === 'pending') {
        return b.createdAt - a.createdAt
      }

      return b.updatedAt - a.updatedAt
    })
  }
  // delete task from list by id
  function deleteTask(taskId) {
    const index = tasks.value.findIndex(task => task.id === taskId) 
    if (index !== -1) {
      tasks.value.splice(index, 1)
    }
  }

  return { tasks, allTasks, currentTask, completedTasks, activeTasks, updateTasks, deleteTask }
})
