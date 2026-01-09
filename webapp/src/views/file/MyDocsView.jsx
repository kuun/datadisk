import React, { useEffect, useRef, useState } from 'react'
import moment from 'moment'
import { useNavigate, useSearchParams } from 'react-router-dom'
import bus from '../../components/uploader/bus'
import http from '../../lib/http'
import { alertError, alertSuccess, formatFileSize } from '../../lib/utils'
import RenameDialog from '../../components/file/RenameDialog'
import ImageViewer from '../../components/viewer/ImageViewer'
import PdfViewer from '../../components/viewer/PdfViewer'
import TextViewer from '../../components/viewer/TextViewer'
import ZipViewer from '../../components/viewer/ZipViewer'
import { Button } from '../../components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger
} from '../../components/ui/dropdown-menu'
import { Folder, MoreHorizontal, RefreshCw, Upload, Download, Trash2, Copy, Scissors, ClipboardPaste } from 'lucide-react'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '../../components/ui/dialog'
import { Input } from '../../components/ui/input'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../components/ui/table'
import './MyDocsView.css'

const MyDocsView = () => {
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const [fileList, setFileList] = useState([])
  const [parentPath, setParentPath] = useState('/')
  const [breadcrumbs, setBreadcrumbs] = useState([])
  const [selectedRows, setSelectedRows] = useState([])
  const [mkdirDialog, setMkdirDialog] = useState(false)
  const [folderName, setFolderName] = useState('')
  const renameDialogRef = useRef(null)
  const imagePreviewRef = useRef(null)
  const textPreviewRef = useRef(null)
  const pdfPreviewRef = useRef(null)
  const zipPreviewRef = useRef(null)
  const [previewImageUrl, setPreviewImageUrl] = useState('')
  const [contextMenu, setContextMenu] = useState({ visible: false, x: 0, y: 0, row: null })
  const breadcrumbRef = useRef(null)

  const clipboardRef = useRef({ isCopy: false, files: [], sourceDir: '' })
  const [pasteDisabled, setPasteDisabled] = useState(true)

  const getAbsPath = (path) => {
    if (parentPath === '/') return `${parentPath}${path}`
    return `${parentPath}/${path}`
  }

  const updateBreadcrumbs = (path) => {
    const parts = path.split('/')
    const next = [{ value: '我的文档', id: '/' }]
    let currentPath = ''
    parts.forEach((part) => {
      if (part) {
        currentPath += `/${part}`
        next.push({ value: part, id: currentPath })
      }
    })
    setBreadcrumbs(next)
  }

  const getFiles = async (path) => {
    try {
      const response = await http.get('/api/file/list', { params: { path } })
      const list = response.data || []
      list.sort((a, b) => {
        if (a.type === 'directory' && b.type !== 'directory') return -1
        if (a.type !== 'directory' && b.type === 'directory') return 1
        return new Date(b.lastmod) - new Date(a.lastmod)
      })
      setFileList(list)
    } catch (error) {
      console.error('Failed to get files:', error)
      alertError('获取文件列表失败')
    }
  }

  const refresh = () => getFiles(parentPath)

  useEffect(() => {
    const path = searchParams.get('path') || '/'
    setParentPath(path)
    updateBreadcrumbs(path)
    getFiles(path)
  }, [searchParams])

  useEffect(() => {
    const onFileAdded = (file) => file.resume?.()
    const onFileSuccess = () => getFiles(parentPath)

    bus.on('fileAdded', onFileAdded)
    bus.on('fileSuccess', onFileSuccess)
    document.addEventListener('click', handleDocumentClick)
    return () => {
      bus.off('fileAdded', onFileAdded)
      bus.off('fileSuccess', onFileSuccess)
      document.removeEventListener('click', handleDocumentClick)
    }
  }, [parentPath])

  const uploadFile = (isFolder) => {
    bus.emit('openUploader', {
      params: {
        parentId: -1,
        parentPath
      },
      options: {
        target: '/api/file/upload'
      },
      others: {
        page: 'file',
        isFolder
      }
    })
  }

  const onClickFileName = async (row) => {
    if (row.type === 'directory') {
      const newPath = getAbsPath(row.basename)
      navigate(`/ui/file/mydocs?path=${encodeURIComponent(newPath)}`)
      return
    }
    if (row.mime && row.mime.includes('image')) {
      setPreviewImageUrl(`/api/file/preview/single?path=${getAbsPath(row.basename)}`)
      imagePreviewRef.current?.open()
      return
    }
    if (row.mime && (row.mime.includes('text') || row.mime.includes('json') || row.mime.includes('javascript'))) {
      try {
        const response = await http.get('/api/file/content', { params: { path: getAbsPath(row.basename) } })
        textPreviewRef.current?.open(response.data)
      } catch (error) {
        console.error('Failed to get file content:', error)
        alertError('获取文件内容失败')
      }
      return
    }
    if (row.mime && row.mime.includes('pdf')) {
      const pdfUrl = `/api/file/preview/single?path=${encodeURIComponent(getAbsPath(row.basename))}`
      pdfPreviewRef.current?.open(pdfUrl)
      return
    }
    if (
      row.mime &&
      (row.mime.includes('zip') ||
        row.mime.includes('gzip') ||
        row.mime.includes('x-tar') ||
        row.mime.includes('x-7z') ||
        row.mime.includes('x-xz') ||
        row.mime.includes('vnd.rar') ||
        row.basename.endsWith('.gz') ||
        row.basename.endsWith('.tgz') ||
        row.basename.endsWith('.xz') ||
        row.basename.endsWith('.tar'))
    ) {
      zipPreviewRef.current?.open(getAbsPath(row.basename))
      return
    }
    if (
      row.mime &&
      (row.mime.includes('msword') ||
        row.mime.includes('wordprocessingml') ||
        row.mime.includes('wps-office.doc') ||
        row.mime.includes('excel') ||
        row.mime.includes('spreadsheetml') ||
        row.mime.includes('wps-office.xls') ||
        row.mime.includes('powerpoint') ||
        row.mime.includes('presentationml') ||
        row.mime.includes('wps-office.ppt'))
    ) {
      try {
        const response = await http.post('/api/editing/create', { filePath: getAbsPath(row.basename) })
        window.open(`/editor.html?session=${response.data.sessionId}`)
      } catch (error) {
        console.error('Failed to create editing session:', error)
        alertError('无法打开文档')
      }
      return
    }
    alert('暂不支持预览该文件类型')
  }

  const handleClickBreadcrumb = (index, item) => {
    const next = breadcrumbs.slice(0, index + 1)
    setBreadcrumbs(next)
    const newPath = item.id
    navigate(`/ui/file/mydocs?path=${encodeURIComponent(newPath)}`)
  }

  const mkdir = async () => {
    try {
      await http.post('/api/file/mkdir', { path: parentPath, name: folderName })
      setMkdirDialog(false)
      setFolderName('')
      getFiles(parentPath)
      alertSuccess('文件夹创建成功')
    } catch (error) {
      console.error('Failed to create directory:', error)
      alertError(error.message || '文件夹创建失败')
    }
  }

  const deleteFile = async () => {
    if (selectedRows.length === 0) {
      alert('请选择要删除的文件')
      return
    }
    try {
      const files = selectedRows.map((row) => row.basename)
      const response = await http.post('/api/file/delete', { files, parentDir: parentPath })
      alertSuccess(response.data.message)
      setSelectedRows([])
      getFiles(parentPath)
    } catch (error) {
      console.error('Failed to delete files:', error)
      alertError('删除文件失败')
    }
  }

  const copyFile = () => {
    if (selectedRows.length === 0) {
      alertError('请选择要复制的文件')
      return
    }
    clipboardRef.current = {
      isCopy: true,
      files: selectedRows.map((row) => row.basename),
      sourceDir: parentPath
    }
    alertSuccess(`已复制${selectedRows.length}个文件`)
    setPasteDisabled(false)
  }

  const moveFile = () => {
    if (selectedRows.length === 0) {
      alertError('请选择要剪切的文件')
      return
    }
    clipboardRef.current = {
      isCopy: false,
      files: selectedRows.map((row) => row.basename),
      sourceDir: parentPath
    }
    alertSuccess(`已剪切${selectedRows.length}个文件`)
    setPasteDisabled(false)
  }

  const pasteFile = async () => {
    try {
      await http.post('/api/file/copy', {
        files: clipboardRef.current.files,
        source: clipboardRef.current.sourceDir,
        target: parentPath,
        isCopy: clipboardRef.current.isCopy
      })
      alertSuccess('文件复制/移动任务已添加，可在任务列表中查看进度')
      clipboardRef.current = { isCopy: false, files: [], sourceDir: '' }
      setPasteDisabled(true)
      setSelectedRows([])
    } catch (error) {
      console.log('Error copying files:', error)
      alertError('复制/移动文件失败')
    }
  }

  const downloadFile = () => {
    if (selectedRows.length <= 0) {
      alert('请选择要下载的文件')
      return
    }
    if (selectedRows.length === 1 && selectedRows[0].type !== 'directory') {
      const url = getAbsPath(selectedRows[0].basename)
      const link = `/api/file/download/single?path=${encodeURIComponent(url)}`
      window.open(link)
      return
    }
    const downloadFiles = selectedRows.map((row) => row.basename)
    http
      .post('/api/file/download/pre', { files: downloadFiles, parentDir: parentPath })
      .then((res) => {
        window.location.href = `/api/file/download?guid=${res.data.guid}`
      })
      .catch((error) => {
        console.error('Failed to prepare download:', error)
        alertError('准备下载失败')
      })
  }

  const handleRowContextMenu = (event, row) => {
    event.preventDefault()
    setContextMenu({ visible: true, x: event.clientX, y: event.clientY, row })
    if (!selectedRows.find((item) => item.basename === row.basename)) {
      setSelectedRows([row])
    }
  }

  const handleContextMenuAction = (action) => {
    const currentRow = contextMenu.row
    setContextMenu((prev) => ({ ...prev, visible: false }))
    switch (action) {
      case 'copy':
        copyFile()
        break
      case 'cut':
        moveFile()
        break
      case 'paste':
        if (!pasteDisabled) pasteFile()
        break
      case 'delete':
        deleteFile()
        break
      case 'rename':
        if (currentRow) {
          renameDialogRef.current?.open(currentRow.basename, parentPath, () => getFiles(parentPath))
        }
        break
      case 'download':
        downloadFile()
        break
      default:
        break
    }
  }

  const handleDocumentClick = (event) => {
    if (!event.target.closest('.context-menu')) {
      setContextMenu((prev) => ({ ...prev, visible: false }))
    }
  }

  const handleBreadcrumbWheel = (event) => {
    if (!breadcrumbRef.current) return
    event.preventDefault()
    breadcrumbRef.current.scrollLeft += event.deltaY
  }

  const toggleSelection = (row) => {
    setSelectedRows((prev) => {
      const exists = prev.find((item) => item.basename === row.basename)
      if (exists) return prev.filter((item) => item.basename !== row.basename)
      return [...prev, row]
    })
  }

  const isSelected = (row) => selectedRows.some((item) => item.basename === row.basename)

  const isAllSelected = fileList.length > 0 && selectedRows.length === fileList.length

  const toggleSelectAll = () => {
    if (isAllSelected) {
      setSelectedRows([])
    } else {
      setSelectedRows([...fileList])
    }
  }

  const fileIcon = (row) => {
    if (row.type === 'directory') return '/assets/img/folder.png'
    if (row.mime?.includes('image')) return '/assets/img/image.png'
    if (row.mime?.includes('msword') || row.mime?.includes('wordprocessingml')) return '/assets/img/word.png'
    if (row.mime?.includes('excel') || row.mime?.includes('spreadsheetml')) return '/assets/img/excel.png'
    if (row.mime?.includes('powerpoint') || row.mime?.includes('presentationml')) return '/assets/img/ppt.png'
    if (
      row.mime?.includes('zip') ||
      row.mime?.includes('x-tar') ||
      row.mime?.includes('x-7z') ||
      row.mime?.includes('vnd.rar') ||
      row.mime?.includes('gzip')
    ) {
      return '/assets/img/zip-new.png'
    }
    if (row.mime?.includes('text') || row.mime?.includes('json') || row.mime?.includes('javascript')) {
      return '/assets/img/text.png'
    }
    if (row.mime?.includes('pdf')) return '/assets/img/pdf.png'
    return '/assets/img/unkown.png'
  }

  return (
    <div className="mydocs">
      <div className="ship-file-btn">
        <div className="toolbar-segment">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button type="button" className="segment-btn">
                <Upload className="mr-1 h-3.5 w-3.5" />
                上传
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuItem onClick={() => uploadFile(false)}>
                <Upload className="mr-2 h-4 w-4" />
                上传文件
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => uploadFile(true)}>
                <Folder className="mr-2 h-4 w-4" />
                上传文件夹
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <button type="button" className="segment-btn" onClick={() => setMkdirDialog(true)}>
            <Folder className="mr-1 h-3.5 w-3.5" />
            新建文件夹
          </button>
        </div>
        <div className="toolbar-segment">
          <button type="button" className="segment-btn" onClick={copyFile}>
            <Copy className="mr-1 h-3.5 w-3.5" />
            复制
          </button>
          <button type="button" className="segment-btn" onClick={moveFile}>
            <Scissors className="mr-1 h-3.5 w-3.5" />
            剪切
          </button>
          <button
            type="button"
            className={`segment-btn ${pasteDisabled ? 'disabled' : ''}`}
            onClick={pasteFile}
            disabled={pasteDisabled}
            title={pasteDisabled ? '暂无可粘贴内容' : ''}
          >
            <ClipboardPaste className="mr-1 h-3.5 w-3.5" />
            粘贴
          </button>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="sm" variant="ghost">
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={downloadFile}>
              <Download className="mr-2 h-4 w-4" />
              下载
            </DropdownMenuItem>
            <DropdownMenuItem onClick={deleteFile}>
              <Trash2 className="mr-2 h-4 w-4" />
              删除
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <Button size="sm" variant="ghost" className="toolbar-refresh" onClick={refresh}>
          <RefreshCw className="mr-2 h-4 w-4" />
        </Button>
      </div>

      <div
        className="ship-bc ship-bc-scroll"
        ref={breadcrumbRef}
        onWheel={handleBreadcrumbWheel}
      >
        {breadcrumbs.map((item, index) => (
          <span key={`${item.id}-${index}`} className="breadcrumb-wrap">
            <button
              type="button"
              className="breadcrumb-item"
              onClick={() => handleClickBreadcrumb(index, item)}
              title={item.value}
            >
              {item.value}
            </button>
            {index < breadcrumbs.length - 1 && <span className="breadcrumb-sep">/</span>}
          </span>
        ))}
      </div>

      <div className="file-table-wrapper">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[40px]">
                <input
                  type="checkbox"
                  checked={isAllSelected}
                  onChange={toggleSelectAll}
                />
              </TableHead>
              <TableHead>文件名称</TableHead>
              <TableHead className="w-[150px]">大小</TableHead>
              <TableHead className="w-[180px]">上传日期</TableHead>
              <TableHead className="w-[120px]">状态</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {fileList.map((row) => (
              <TableRow
                key={row.basename}
                onContextMenu={(event) => handleRowContextMenu(event, row)}
                className={isSelected(row) ? 'selected' : ''}
              >
                <TableCell>
                  <input
                    type="checkbox"
                    checked={isSelected(row)}
                    onChange={() => toggleSelection(row)}
                  />
                </TableCell>
                <TableCell onClick={() => onClickFileName(row)}>
                  <div className="file-name">
                    <img src={fileIcon(row)} width="24" height="24" alt="" />
                    <span>{row.basename}</span>
                  </div>
                </TableCell>
                <TableCell>{row.size === 0 ? '-' : formatFileSize(row.size)}</TableCell>
                <TableCell>{moment(row.lastmod).format('YYYY-MM-DD HH:mm:ss')}</TableCell>
                <TableCell>{row.status || '-'}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <Dialog open={mkdirDialog} onOpenChange={setMkdirDialog}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>新建文件夹</DialogTitle>
          </DialogHeader>
          <Input value={folderName} onChange={(event) => setFolderName(event.target.value)} />
          <DialogFooter>
            <Button variant="secondary" onClick={() => setMkdirDialog(false)}>
              取消
            </Button>
            <Button onClick={mkdir}>保存</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {contextMenu.visible && (
        <div className="context-menu" style={{ left: contextMenu.x, top: contextMenu.y }}>
          <button type="button" className="context-menu-item" onClick={() => handleContextMenuAction('copy')}>
            复制
          </button>
          <button type="button" className="context-menu-item" onClick={() => handleContextMenuAction('cut')}>
            剪切
          </button>
          <button
            type="button"
            className={`context-menu-item ${pasteDisabled ? 'disabled' : ''}`}
            onClick={() => !pasteDisabled && handleContextMenuAction('paste')}
          >
            粘贴
          </button>
          <div className="context-menu-divider" />
          <button type="button" className="context-menu-item" onClick={() => handleContextMenuAction('rename')}>
            重命名
          </button>
          <button type="button" className="context-menu-item" onClick={() => handleContextMenuAction('download')}>
            下载
          </button>
          <div className="context-menu-divider" />
          <button type="button" className="context-menu-item danger" onClick={() => handleContextMenuAction('delete')}>
            删除
          </button>
        </div>
      )}

      <RenameDialog ref={renameDialogRef} />
      <ImageViewer ref={imagePreviewRef} imageUrl={previewImageUrl} />
      <TextViewer ref={textPreviewRef} />
      <PdfViewer ref={pdfPreviewRef} />
      <ZipViewer ref={zipPreviewRef} />
    </div>
  )
}

export default MyDocsView
