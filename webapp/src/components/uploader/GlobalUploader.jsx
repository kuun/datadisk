import React, { useEffect, useMemo, useRef, useState } from 'react'
import { ChevronDown, X } from 'lucide-react'
import http from '../../lib/http'
import bus from './bus'
import { Button } from '../ui/button'
import { ScrollArea } from '../ui/scroll-area'
import { Progress } from '../ui/progress'
import { cn } from '../../lib/cn'
import './global-uploader.css'

const statusText = {
  success: '上传成功',
  error: '上传失败',
  uploading: '上传中',
  waiting: '等待上传',
  rejected: '已拒绝'
}

const formatFileSize = (bytes) => {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB'
}

const formatSpeed = (bytesPerSecond) => {
  if (bytesPerSecond < 1024) return bytesPerSecond.toFixed(0) + ' B/s'
  if (bytesPerSecond < 1024 * 1024) return (bytesPerSecond / 1024).toFixed(1) + ' KB/s'
  if (bytesPerSecond < 1024 * 1024 * 1024) return (bytesPerSecond / (1024 * 1024)).toFixed(1) + ' MB/s'
  return (bytesPerSecond / (1024 * 1024 * 1024)).toFixed(2) + ' GB/s'
}

const GlobalUploader = ({ global = true }) => {
  const [panelShow, setPanelShow] = useState(false)
  const [collapse, setCollapse] = useState(false)
  const [files, setFiles] = useState([])
  const [customParams, setCustomParams] = useState({})
  const [customOptions, setCustomOptions] = useState({})
  const [maxUploadSize, setMaxUploadSize] = useState(0)
  const fileInputRef = useRef(null)
  const folderInputRef = useRef(null)
  const configFetchedRef = useRef(false)
  const abortControllersRef = useRef({})

  // Fetch upload config (called when uploader is opened, not on mount)
  const fetchConfig = async () => {
    if (configFetchedRef.current) return
    try {
      const response = await http.get('/api/config')
      if (response.data?.maxUploadSize) {
        setMaxUploadSize(response.data.maxUploadSize)
        configFetchedRef.current = true
      }
    } catch (error) {
      console.error('Failed to fetch config', error)
    }
  }

  const options = useMemo(
    () => ({
      target: '/api/file/upload',
      ...(customOptions || {})
    }),
    [customOptions]
  )

  const addFiles = (fileList) => {
    const incoming = Array.from(fileList).map((file) => {
      const id = `${file.name}-${file.size}-${file.lastModified}-${Math.random()}`
      // Check file size limit
      const exceedsLimit = maxUploadSize > 0 && file.size > maxUploadSize
      return {
        id,
        file,
        name: file.name,
        size: file.size,
        progress: 0,
        status: exceedsLimit ? 'rejected' : 'waiting',
        errorMessage: exceedsLimit
          ? `文件大小超过限制，最大允许 ${formatFileSize(maxUploadSize)}`
          : '',
        params: { ...customParams }
      }
    })

    setFiles((prev) => [...prev, ...incoming])
    setPanelShow(true)

    // Only emit fileAdded for files that are not rejected
    incoming
      .filter((f) => f.status !== 'rejected')
      .forEach((fileWrapper) => {
        bus.emit('fileAdded', {
          ...fileWrapper,
          resume: () => uploadFile(fileWrapper),
          cancel: () => cancelFile(fileWrapper.id)
        })
      })
  }

  const uploadFile = async (fileWrapper) => {
    let lastLoaded = 0
    let lastTime = Date.now()

    // Create AbortController for this upload
    const abortController = new AbortController()
    abortControllersRef.current[fileWrapper.id] = abortController

    setFiles((prev) =>
      prev.map((item) => (item.id === fileWrapper.id ? { ...item, status: 'uploading', errorMessage: '', speed: 0 } : item))
    )

    const formData = new FormData()
    formData.append('file', fileWrapper.file)
    formData.append('totalSize', String(fileWrapper.file.size || 0))
    Object.entries(fileWrapper.params || {}).forEach(([key, value]) => {
      formData.append(key, value)
    })

    try {
      await http.post(options.target, formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
        signal: abortController.signal,
        onUploadProgress: (event) => {
          if (!event.total) return
          const now = Date.now()
          const timeDiff = (now - lastTime) / 1000
          const percent = Math.round((event.loaded / event.total) * 100)

          // Calculate speed (update every 500ms to avoid flickering)
          let speed = 0
          if (timeDiff >= 0.5) {
            const bytesDiff = event.loaded - lastLoaded
            speed = bytesDiff / timeDiff
            lastLoaded = event.loaded
            lastTime = now
          }

          setFiles((prev) =>
            prev.map((item) => {
              if (item.id !== fileWrapper.id) return item
              return {
                ...item,
                progress: percent,
                speed: speed > 0 ? speed : item.speed
              }
            })
          )
        }
      })
      delete abortControllersRef.current[fileWrapper.id]
      setFiles((prev) =>
        prev.map((item) =>
          item.id === fileWrapper.id ? { ...item, status: 'success', progress: 100 } : item
        )
      )
      bus.emit('fileSuccess', fileWrapper)
    } catch (error) {
      delete abortControllersRef.current[fileWrapper.id]

      // Check if upload was cancelled
      if (error.name === 'CanceledError' || error.code === 'ERR_CANCELED') {
        return
      }

      console.error('Upload failed', error)
      // Extract error message from server response
      let errorMessage = '上传失败'

      // Check HTTP status first (DefaultBodyLimit returns 413 with empty body)
      if (error.response?.status === 413) {
        // Try to get message from response data, otherwise use default
        errorMessage = error.response?.data?.message || '文件大小超过服务器限制'
      } else if (error.response?.data?.message) {
        errorMessage = error.response.data.message
      } else if (error.code === 'ERR_NETWORK') {
        errorMessage = '网络连接失败，请检查网络后重试'
      }

      setFiles((prev) =>
        prev.map((item) => (item.id === fileWrapper.id ? { ...item, status: 'error', errorMessage } : item))
      )
    }
  }

  const cancelFile = (id) => {
    // Abort upload if in progress
    if (abortControllersRef.current[id]) {
      abortControllersRef.current[id].abort()
      delete abortControllersRef.current[id]
    }
    setFiles((prev) => prev.filter((file) => file.id !== id))
  }

  const close = () => {
    // Abort all uploads in progress
    Object.values(abortControllersRef.current).forEach((controller) => controller.abort())
    abortControllersRef.current = {}
    setFiles([])
    setPanelShow(false)
  }

  // Handle file selection and reset input for re-upload
  const handleFileChange = (event) => {
    addFiles(event.target.files)
    // Reset input value to allow selecting the same file again
    event.target.value = ''
  }

  useEffect(() => {
    const openUploader = async ({ params = {}, options: opts = {}, others = {} }) => {
      // Fetch config before opening file picker (only once)
      await fetchConfig()
      setCustomParams(params)
      setCustomOptions(opts)
      if (others.isFolder) {
        folderInputRef.current?.click()
      } else {
        fileInputRef.current?.click()
      }
    }

    const closePanel = (logout) => {
      if (logout) {
        setFiles([])
      }
      setPanelShow(false)
    }

    const openPanel = () => setPanelShow(true)

    bus.on('openUploader', openUploader)
    bus.on('closeUploadPanel', closePanel)
    bus.on('openUploadPanel', openPanel)

    return () => {
      bus.off('openUploader', openUploader)
      bus.off('closeUploadPanel', closePanel)
      bus.off('openUploadPanel', openPanel)
    }
  }, [])

  return (
    <div id="global-uploader" className={cn(!global && 'global-uploader-single')}>
      <input
        ref={fileInputRef}
        type="file"
        multiple
        className="hidden"
        onChange={handleFileChange}
      />
      <input
        ref={folderInputRef}
        type="file"
        webkitdirectory="true"
        directory=""
        multiple
        className="hidden"
        onChange={handleFileChange}
      />

      {panelShow && (
        <div className={cn('file-panel', collapse && 'collapse')}>
          <div className="file-title">
            <div className="title">文件列表</div>
            <div className="operate">
              <Button variant="ghost" size="icon" onClick={() => setCollapse((prev) => !prev)}>
                <ChevronDown className={cn('h-4 w-4', collapse && 'rotate-180')} />
              </Button>
              <Button variant="ghost" size="icon" onClick={close}>
                <X className="h-4 w-4" />
              </Button>
            </div>
          </div>
          <ScrollArea className="file-list">
            {files.length === 0 ? (
              <div className="no-file">暂无待上传文件</div>
            ) : (
              <ul className="space-y-2 p-3">
                {files.map((file) => (
                  <li key={file.id} className="file-item">
                    <div className="flex items-center justify-between text-sm">
                      <div className="flex-1 truncate">{file.name}</div>
                      <div className="flex items-center gap-2">
                        <span className={cn(
                          "text-xs",
                          (file.status === 'error' || file.status === 'rejected') ? 'text-red-500' : 'text-slate-500'
                        )}>
                          {(file.status === 'error' || file.status === 'rejected') && file.errorMessage
                            ? file.errorMessage
                            : file.status === 'uploading' && file.speed > 0
                              ? `${file.progress}% · ${formatSpeed(file.speed)}`
                              : statusText[file.status]}
                        </span>
                        {(file.status === 'uploading' || file.status === 'waiting') && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-5 w-5"
                            onClick={() => cancelFile(file.id)}
                          >
                            <X className="h-3 w-3" />
                          </Button>
                        )}
                      </div>
                    </div>
                    <Progress className="mt-2" value={file.progress} />
                  </li>
                ))}
              </ul>
            )}
          </ScrollArea>
        </div>
      )}
    </div>
  )
}

export default GlobalUploader
