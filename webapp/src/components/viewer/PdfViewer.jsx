import React, { forwardRef, useImperativeHandle, useState } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog'
import './PdfViewer.css'

const PdfViewer = forwardRef((_, ref) => {
  const [visible, setVisible] = useState(false)
  const [url, setUrl] = useState('')

  const handleOpenChange = (nextOpen) => {
    if (!nextOpen) {
      setVisible(false)
      setUrl('')
      return
    }
    setVisible(true)
  }

  useImperativeHandle(ref, () => ({
    open: (pdfUrl) => {
      setUrl(pdfUrl)
      setVisible(true)
    }
  }))

  return (
    <Dialog open={visible} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-6xl">
        <DialogHeader>
          <DialogTitle>PDF 预览</DialogTitle>
        </DialogHeader>
        <div className="pdf-container">
          <iframe title="PDF" src={url} className="pdf-frame" />
        </div>

      </DialogContent>
    </Dialog>
  )
})

PdfViewer.displayName = 'PdfViewer'

export default PdfViewer
