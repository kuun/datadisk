import React, { forwardRef, useImperativeHandle, useState } from 'react'
import http from '../../lib/http'
import { alertError, alertSuccess } from '../../lib/utils'
import { Button } from '../ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '../ui/dialog'
import { Input } from '../ui/input'

const RenameDialog = forwardRef((_, ref) => {
  const [visible, setVisible] = useState(false)
  const [renameForm, setRenameForm] = useState({ newName: '', oldName: '', path: '' })
  const [onSuccess, setOnSuccess] = useState(() => () => {})

  useImperativeHandle(ref, () => ({
    open: (oldName, path, handleSuccess) => {
      setRenameForm({ oldName, newName: oldName, path })
      setVisible(true)
      setOnSuccess(() => handleSuccess)
    }
  }))

  const renameFile = async () => {
    try {
      const path = renameForm.path
      const oldPath = path === '/' ? `${path}${renameForm.oldName}` : `${path}/${renameForm.oldName}`
      await http.post('/api/file/rename', {
        oldPath,
        newName: renameForm.newName
      })
      alertSuccess('文件重命名成功')
      setVisible(false)
      onSuccess?.()
    } catch (error) {
      if (error.response?.status === 409) {
        alertError('操作失败：文件已存在!')
      } else if (error.response?.status === 403) {
        alertError('操作失败：没有权限!')
      } else if (error.response?.status === 404) {
        alertError('操作失败：文件不存在!')
      } else {
        alertError('文件重命名失败')
      }
    }
  }

  return (
    <Dialog open={visible} onOpenChange={setVisible}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>重命名</DialogTitle>
        </DialogHeader>
        <div className="space-y-2">
          <label htmlFor="newName" className="text-sm text-slate-600">
            新名称
          </label>
          <Input
            id="newName"
            value={renameForm.newName}
            onChange={(event) => setRenameForm((prev) => ({ ...prev, newName: event.target.value }))}
          />
        </div>
        <DialogFooter>
          <Button variant="secondary" onClick={() => setVisible(false)}>
            取消
          </Button>
          <Button onClick={renameFile}>保存</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
})

RenameDialog.displayName = 'RenameDialog'

export default RenameDialog
