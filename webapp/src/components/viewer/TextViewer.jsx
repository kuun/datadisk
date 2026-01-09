import React, { forwardRef, useImperativeHandle, useState } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog'

const TextViewer = forwardRef((_, ref) => {
  const [visible, setVisible] = useState(false)
  const [content, setContent] = useState('')

  const handleOpenChange = (nextOpen) => {
    if (!nextOpen) {
      setVisible(false)
      setContent('')
      return
    }
    setVisible(true)
  }

  useImperativeHandle(ref, () => ({
    open: (text) => {
      setContent(text)
      setVisible(true)
    }
  }))

  return (
    <Dialog open={visible} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-5xl">
        <DialogHeader>
          <DialogTitle>文本预览</DialogTitle>
        </DialogHeader>
        <div className="max-h-[70vh] overflow-auto">
          <pre className="whitespace-pre-wrap break-words">{content}</pre>
        </div>
      </DialogContent>
    </Dialog>
  )
})

TextViewer.displayName = 'TextViewer'

export default TextViewer
