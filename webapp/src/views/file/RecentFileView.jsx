import React, { useEffect, useMemo, useRef, useState } from 'react'
import http from '../../lib/http'
import { alertError, alertSuccess, formatFileSize } from '../../lib/utils'
import { Button } from '../../components/ui/button'
import { Input } from '../../components/ui/input'
import { Card } from '../../components/ui/card'
import { RefreshCw, X } from 'lucide-react'
import ImageViewer from '../../components/viewer/ImageViewer'
import PdfViewer from '../../components/viewer/PdfViewer'
import TextViewer from '../../components/viewer/TextViewer'
import ZipViewer from '../../components/viewer/ZipViewer'
import './RecentFileView.css'

const RecentFileView = () => {
  const [recentFiles, setRecentFiles] = useState([])
  const [loading, setLoading] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [filterType, setFilterType] = useState('all')
  const imagePreviewRef = useRef(null)
  const textPreviewRef = useRef(null)
  const pdfPreviewRef = useRef(null)
  const zipPreviewRef = useRef(null)
  const [previewImageUrl, setPreviewImageUrl] = useState('')

  const loadRecentFiles = async () => {
    setLoading(true)
    try {
      const response = await http.get('/api/file/recent?limit=50')
      setRecentFiles(response.data.files || [])
    } catch (error) {
      console.error('Failed to load recent files:', error)
      alertError('加载最近使用的文件失败')
      setRecentFiles([])
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadRecentFiles()
  }, [])

  const filteredFiles = useMemo(() => {
    const files = recentFiles.filter((file) => {
      if (!searchQuery) return true
      const query = searchQuery.toLowerCase()
      return file.fileName.toLowerCase().includes(query) || file.filePath.toLowerCase().includes(query)
    })
    const filtered = filterType === 'all' ? files : files.filter((file) => file.accessType === filterType)
    return filtered.sort((a, b) => b.accessTime - a.accessTime)
  }, [recentFiles, searchQuery, filterType])

  const getFileIcon = (file) => {
    if (file.isDir) return 'folder.png'
    const fileName = file.fileName.toLowerCase()
    if (/\.(jpg|jpeg|png|gif|bmp|webp)$/.test(fileName)) return 'image.png'
    if (/\.(doc|docx)$/.test(fileName)) return 'word.png'
    if (/\.(xls|xlsx)$/.test(fileName)) return 'excel.png'
    if (/\.(ppt|pptx)$/.test(fileName)) return 'ppt.png'
    if (/\.(zip|rar|7z|tar|gz|xz)$/.test(fileName)) return 'zip-new.png'
    if (/\.(txt|json|xml|csv)$/.test(fileName)) return 'text.png'
    if (/\.pdf$/.test(fileName)) return 'pdf.png'
    return 'unkown.png'
  }

  const getRelativeTime = (timestamp) => {
    const now = Date.now()
    const diff = now - timestamp * 1000
    const minutes = Math.floor(diff / (1000 * 60))
    const hours = Math.floor(diff / (1000 * 60 * 60))
    const days = Math.floor(diff / (1000 * 60 * 60 * 24))
    if (minutes < 1) return '刚刚'
    if (minutes < 60) return `${minutes}分钟前`
    if (hours < 24) return `${hours}小时前`
    if (days < 7) return `${days}天前`
    return '一周前'
  }

  const buildDownloadLink = (filePath) => `/api/file/download/single?path=${encodeURIComponent(filePath)}`
  const buildPreviewLink = (filePath) => `/api/file/preview/single?path=${encodeURIComponent(filePath)}`
  const accessLabels = {
    browse: '浏览',
    download: '下载',
    edit: '编辑',
    upload: '上传',
    preview: '预览'
  }
  const getAccessLabel = (accessType) => accessLabels[accessType] || accessType

  const isDocumentFile = (fileName) => {
    const ext = fileName.toLowerCase().split('.').pop()
    const docExts = ['doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'wps', 'et', 'dps']
    return docExts.includes(ext)
  }

  const onClickFile = async (file) => {
    const fileInfo = file.fileInfo
    if (fileInfo.isDirectory) {
      window.open(`/ui/file/mydocs?path=${encodeURIComponent(file.filePath)}`, '_blank')
      return
    }
    const mimeType = fileInfo.type || ''
    if (mimeType.includes('image')) {
      setPreviewImageUrl(buildPreviewLink(file.filePath))
      imagePreviewRef.current?.open()
    } else if (mimeType.includes('text') || mimeType.includes('application/json')) {
      try {
        const response = await http.get('/api/file/content', {
          params: { path: file.filePath },
          responseType: 'text'
        })
        textPreviewRef.current?.open(response.data)
      } catch (error) {
        alertError('无法预览该文件')
      }
    } else if (mimeType.includes('pdf')) {
      pdfPreviewRef.current?.open(buildPreviewLink(file.filePath))
    } else if (
      mimeType.includes('zip') ||
      mimeType.includes('gzip') ||
      mimeType.includes('x-tar') ||
      mimeType.includes('x-xz') ||
      mimeType.includes('x-7z') ||
      mimeType.includes('vnd.rar') ||
      file.fileName.endsWith('.xz') ||
      file.fileName.endsWith('.tar')
    ) {
      zipPreviewRef.current?.open(file.filePath)
    } else if (isDocumentFile(file.fileName)) {
      try {
        const response = await http.post('/api/editing/create', { filePath: file.filePath })
        window.open(`/editor.html?session=${response.data.sessionId}`)
      } catch (error) {
        alertError('无法打开文档')
      }
    } else {
      window.open(buildDownloadLink(file.filePath))
    }
  }

  const deleteFromRecent = async (file) => {
    const confirmed = window.confirm(`确定要从最近访问中删除 "${file.fileName}" 吗？`)
    if (!confirmed) return
    try {
      await http.delete(`/api/file/recent/${file.id}`)
      alertSuccess('已从最近访问中删除')
      setRecentFiles((prev) => prev.filter((item) => item.id !== file.id))
    } catch (error) {
      console.error('Failed to delete recent file:', error)
      alertError('删除失败')
    }
  }

  return (
    <div className="recent-files">
      <div className="recent-header">
        <h2>最近访问的文件</h2>
        <div className="recent-actions">
          <div className="recent-toolbar">
            <div className="recent-search-wrap">
              <Input
                className="recent-search"
                placeholder="搜索最近文件..."
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
              />
              {searchQuery ? (
                <button
                  type="button"
                  className="recent-search-clear"
                  onClick={() => setSearchQuery('')}
                  aria-label="清除搜索"
                  title="清除"
                >
                  <X className="h-3 w-3" />
                </button>
              ) : null}
            </div>
            <div className="recent-filters">
              {[
                { key: 'all', label: '全部' },
                { key: 'preview', label: '预览' },
                { key: 'download', label: '下载' },
                { key: 'edit', label: '编辑' },
                { key: 'upload', label: '上传' }
              ].map((item) => (
                <button
                  key={item.key}
                  type="button"
                  className={`filter-pill ${filterType === item.key ? 'active' : ''}`}
                  onClick={() => setFilterType(item.key)}
                >
                  {item.label}
                </button>
              ))}
            </div>
          </div>
          <Button
            variant="secondary"
            size="icon"
            className="recent-refresh"
            onClick={loadRecentFiles}
            disabled={loading}
            aria-label="刷新"
            title="刷新"
          >
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
      </div>
      <div className="recent-list">
        {filteredFiles.length === 0 ? (
          <div className="recent-empty">
            <div className="recent-empty-illustration" aria-hidden="true">
              <div className="recent-empty-ring" />
              <div className="recent-empty-dot" />
              <div className="recent-empty-bar" />
            </div>
            <div className="recent-empty-title">最近访问为空</div>
            <div className="recent-empty-subtitle">浏览、下载或编辑文件后，这里会自动记录。</div>
          </div>
        ) : (
          filteredFiles.map((row) => (
            <Card key={row.id} className="recent-card">
              <div className="recent-card-body">
                <div className="recent-card-left">
                  <img
                    className="recent-icon"
                    src={`/assets/img/${getFileIcon(row)}`}
                    width="28"
                    height="28"
                    alt=""
                  />
                  <div className="recent-meta">
                    <button type="button" onClick={() => onClickFile(row)} className="recent-link">
                      {row.fileName}
                    </button>
                    <div className="recent-path" title={row.filePath}>
                      {row.filePath}
                    </div>
                  </div>
                </div>
                <div className="recent-card-right">
                  <div className="recent-card-top">
                    <span className="recent-tag">{getAccessLabel(row.accessType)}</span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="recent-delete h-6 w-6 rounded-full p-0"
                      onClick={() => deleteFromRecent(row)}
                      aria-label="删除"
                      title="删除"
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                  <div className="recent-card-bottom">
                    <span className="recent-size">
                      {row.fileInfo.size === 0 ? '-' : formatFileSize(row.fileInfo.size)}
                    </span>
                    <span className="recent-time">{getRelativeTime(row.accessTime)}</span>
                  </div>
                </div>
              </div>
            </Card>
          ))
        )}
      </div>
      <ImageViewer ref={imagePreviewRef} imageUrl={previewImageUrl} />
      <TextViewer ref={textPreviewRef} />
      <PdfViewer ref={pdfPreviewRef} />
      <ZipViewer ref={zipPreviewRef} />
    </div>
  )
}

export default RecentFileView
