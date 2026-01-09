import React, { forwardRef, useImperativeHandle, useMemo, useState } from 'react'
import { File, Folder } from 'lucide-react'
import moment from 'moment'
import http from '../../lib/http'
import { alertError } from '../../lib/utils'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../ui/table'
import './ZipViewer.css'

const ZipViewer = forwardRef((_, ref) => {
  const [visible, setVisible] = useState(false)
  const [fileList, setFileList] = useState([])
  const [currentPath, setCurrentPath] = useState('')
  const [breadcrumbs, setBreadcrumbs] = useState([{ name: '根目录', path: '' }])
  const [loading, setLoading] = useState(false)

  const currentDirFiles = useMemo(() => {
    // 收集当前目录下的直接子项（包括虚拟目录）
    const itemsMap = new Map()

    fileList.forEach((file) => {
      if (!file.path) return

      const filePath = file.path
      const prefix = currentPath ? currentPath + '/' : ''

      // 检查是否在当前目录下
      if (currentPath === '') {
        // 根目录：获取第一级
        const firstPart = filePath.split('/')[0]
        if (filePath.includes('/')) {
          // 是子目录中的文件，创建虚拟目录
          if (!itemsMap.has(firstPart)) {
            itemsMap.set(firstPart, {
              name: firstPart,
              path: firstPart,
              size: 0,
              dir: true,
              date: file.date
            })
          }
        } else {
          // 直接在根目录的文件
          itemsMap.set(filePath, file)
        }
      } else if (filePath.startsWith(prefix)) {
        // 在当前子目录下
        const relativePath = filePath.slice(prefix.length)
        const parts = relativePath.split('/')
        const firstPart = parts[0]

        if (parts.length > 1) {
          // 是更深层的文件，创建虚拟目录
          const dirPath = currentPath + '/' + firstPart
          if (!itemsMap.has(dirPath)) {
            itemsMap.set(dirPath, {
              name: firstPart,
              path: dirPath,
              size: 0,
              dir: true,
              date: file.date
            })
          }
        } else {
          // 直接子项
          itemsMap.set(filePath, file)
        }
      }
    })

    const items = Array.from(itemsMap.values())
    return items.sort((a, b) => {
      if (a.dir !== b.dir) return a.dir ? -1 : 1
      return a.name.localeCompare(b.name)
    })
  }, [fileList, currentPath])

  const updateBreadcrumbs = (path) => {
    const next = [{ name: '根目录', path: '' }]
    if (path) {
      const parts = path.split('/')
      let working = ''
      parts.forEach((part) => {
        if (part) {
          working += (working ? '/' : '') + part
          next.push({ name: part, path: working })
        }
      })
    }
    setBreadcrumbs(next)
  }

  const resetState = () => {
    setCurrentPath('')
    setBreadcrumbs([{ name: '根目录', path: '' }])
    setFileList([])
    setLoading(false)
  }

  const open = async (filePath) => {
    setVisible(true)
    resetState()
    try {
      setLoading(true)
      const response = await http.get('/api/archive/preview', {
        params: { path: filePath }
      })
      const array = response.data.map((file) => ({
        ...file,
        path: file.path?.replace(/\/$/, '')
      }))
      setFileList(array)
    } catch (error) {
      console.error('打开压缩文件错误:', error)
      alertError(error.response?.data?.error || '无法打开此压缩文件')
      setVisible(false)
      resetState()
    } finally {
      setLoading(false)
    }
  }

  useImperativeHandle(ref, () => ({ open }))

  const handleBreadcrumbClick = (index) => {
    const item = breadcrumbs[index]
    setBreadcrumbs(breadcrumbs.slice(0, index + 1))
    setCurrentPath(item.path)
  }

  const handleRowClick = (row) => {
    if (row.dir) {
      setCurrentPath(row.path)
      updateBreadcrumbs(row.path)
    }
  }

  const formatFileSize = (size) => {
    if (!size || Number.isNaN(size)) return '-'
    if (size < 1024) return `${size} B`
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(2)} KB`
    if (size < 1024 * 1024 * 1024) return `${(size / 1024 / 1024).toFixed(2)} MB`
    return `${(size / 1024 / 1024 / 1024).toFixed(2)} GB`
  }

  const formatDate = (date) => moment(date).format('YYYY-MM-DD HH:mm:ss')

  const handleOpenChange = (nextOpen) => {
    if (!nextOpen) {
      setVisible(false)
      resetState()
      return
    }
    setVisible(true)
  }

  return (
    <Dialog open={visible} onOpenChange={handleOpenChange}>
      <DialogContent className="zip-dialog">
        <DialogHeader>
          <DialogTitle>压缩文件预览</DialogTitle>
        </DialogHeader>
        <div className="zip-container">
          <div className="breadcrumb-container">
            {breadcrumbs.map((item, index) => (
              <button
                key={item.path || index}
                className="breadcrumb-item"
                onClick={() => handleBreadcrumbClick(index)}
                type="button"
              >
                {item.name}
              </button>
            ))}
          </div>

          <div className="zip-table">
            {loading ? (
              <div className="zip-loading">正在解析文件...</div>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>文件名</TableHead>
                    <TableHead className="w-[120px]">大小</TableHead>
                    <TableHead className="w-[180px]">修改时间</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {currentDirFiles.map((row) => (
                    <TableRow
                      key={row.path}
                      onClick={() => handleRowClick(row)}
                      className={row.dir ? 'cursor-pointer' : ''}
                    >
                      <TableCell>
                        <span className="zip-file-name">
                          {row.dir ? (
                            <Folder className="zip-icon zip-icon-folder" />
                          ) : (
                            <File className="zip-icon" />
                          )}
                          {row.name}
                        </span>
                      </TableCell>
                      <TableCell>{row.dir ? '-' : formatFileSize(row.size)}</TableCell>
                      <TableCell>{formatDate(row.date)}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
})

ZipViewer.displayName = 'ZipViewer'

export default ZipViewer
