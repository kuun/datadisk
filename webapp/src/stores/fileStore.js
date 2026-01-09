import { defineStore } from 'pinia'
import { ref, h } from 'vue'
import axios from 'axios'
import { Folder } from '@element-plus/icons-vue'

export const useFileStore = defineStore('file', () => {
  const currentPath = ref('')
  const menuChildren = ref([])

  const updateMenuChildren = async (path) => {
    try {
      const response = await axios.get('/api/file/list', {
        params: { path }
      })
      const contents = response.data || []
      const children = contents
        .filter(item => item.type === 'directory')
        .sort((a, b) => {
          // 使用lastmod作为排序依据，按时间倒序排列
          return new Date(b.lastmod).getTime() - new Date(a.lastmod).getTime()
        })
        .map(item => ({
          label: item.basename,
          icon: h(Folder, { style: 'color: #f5c000' }),
          key: 'docs' + item.filename,
          children: []
        }))
      
      if (path === '/') {
        menuChildren.value = children
      } else {
        const parentKey = 'docs' + path
        const updateNode = (nodes) => {
          for (const node of nodes) {
            if (node.key === parentKey) {
              node.children = children
              return true
            }
            if (node.children?.length) {
              if (updateNode(node.children)) {
                return true
              }
            }
          }
          return false
        }
        
        updateNode(menuChildren.value)
      }
    } catch (error) {
      console.error('获取目录失败:', error)
    }
  }

  const setCurrentPath = (path) => {
    currentPath.value = path
  }

  return {
    currentPath,
    menuChildren,
    updateMenuChildren,
    setCurrentPath
  }
})
