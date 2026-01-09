import { RefreshCw, Trash2 } from 'lucide-react'
import React, { useEffect, useState } from 'react'
import http from '../lib/http'
import { alertError } from '../lib/utils'
import { Button } from '../components/ui/button'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../components/ui/table'
import './AuditView.css'

const AuditView = () => {
  const [logs, setLogs] = useState([])
  const [selectedRows, setSelectedRows] = useState([])
  const [pageInfo, setPageInfo] = useState({ currentPage: 1, pageSize: 50, total: 0 })

  const fetchLogs = async (page = 1, pageSize = pageInfo.pageSize) => {
    const response = await http.get('/api/oplog/query', { params: { page, pageSize } })
    setLogs(response.data.logs || [])
    setPageInfo((prev) => ({ ...prev, total: response.data.total || 0, currentPage: page }))
  }

  useEffect(() => {
    fetchLogs()
  }, [])

  const refreshLogs = () => fetchLogs(pageInfo.currentPage, pageInfo.pageSize)

  const deleteLogs = async () => {
    if (selectedRows.length === 0) {
      alertError('请选择要删除的日志.')
      return
    }
    const selectedIds = selectedRows.map((row) => row.id)
    await http.post('/api/oplog/delete', selectedIds)
    setSelectedRows([])
    fetchLogs(pageInfo.currentPage, pageInfo.pageSize)
  }

  const toggleSelection = (row) => {
    setSelectedRows((prev) => {
      const exists = prev.find((item) => item.id === row.id)
      if (exists) return prev.filter((item) => item.id !== row.id)
      return [...prev, row]
    })
  }

  const isSelected = (row) => selectedRows.some((item) => item.id === row.id)

  const isAllSelected = logs.length > 0 && selectedRows.length === logs.length

  const toggleSelectAll = () => {
    if (isAllSelected) {
      setSelectedRows([])
    } else {
      setSelectedRows([...logs])
    }
  }

  return (
    <div className="audit-container">
      <div className="toolbar-btn">
        <div className="toolbar-segment">
          <button type="button" className="segment-btn" onClick={refreshLogs}>
            <RefreshCw className="mr-1 h-3.5 w-3.5" />
            刷新
          </button>
          <button type="button" className="segment-btn danger" onClick={deleteLogs}>
            <Trash2 className="mr-1 h-3.5 w-3.5" />
            删除
          </button>
        </div>
      </div>
      <div className="table-container">
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
              <TableHead className="w-[180px]">时间</TableHead>
              <TableHead className="w-[180px]">用户名</TableHead>
              <TableHead className="w-[180px]">操作类型</TableHead>
              <TableHead>描述</TableHead>
              <TableHead className="w-[120px]">结果</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {logs.map((row) => (
              <TableRow key={row.id} className={isSelected(row) ? 'selected' : ''}>
                <TableCell>
                  <input type="checkbox" checked={isSelected(row)} onChange={() => toggleSelection(row)} />
                </TableCell>
                <TableCell>{new Date(row.opTime * 1000).toLocaleString()}</TableCell>
                <TableCell>{row.username}</TableCell>
                <TableCell>{row.opType}</TableCell>
                <TableCell className="whitespace-pre-wrap">{row.opDesc}</TableCell>
                <TableCell>{row.result}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
      <div className="audit-pager">
        <div className="pager-left">
          共 {pageInfo.total} 条
          <select
            value={pageInfo.pageSize}
            onChange={(event) => {
              const size = Number(event.target.value)
              setPageInfo((prev) => ({ ...prev, pageSize: size }))
              fetchLogs(1, size)
            }}
          >
            {[15, 25, 50, 100].map((size) => (
              <option key={size} value={size}>
                {size}/页
              </option>
            ))}
          </select>
        </div>
        <div className="pager-right">
          <Button
            variant="secondary"
            onClick={() => fetchLogs(Math.max(1, pageInfo.currentPage - 1), pageInfo.pageSize)}
          >
            上一页
          </Button>
          <span>{pageInfo.currentPage}</span>
          <Button
            variant="secondary"
            onClick={() =>
              fetchLogs(
                Math.min(Math.ceil(pageInfo.total / pageInfo.pageSize), pageInfo.currentPage + 1),
                pageInfo.pageSize
              )
            }
          >
            下一页
          </Button>
        </div>
      </div>
    </div>
  )
}

export default AuditView
