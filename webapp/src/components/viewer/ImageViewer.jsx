import React, { forwardRef, useImperativeHandle, useState } from 'react'
import { Button } from '../ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '../ui/dialog'
import './ImageViewer.css'

const ImageViewer = forwardRef(({ imageUrl }, ref) => {
  const [visible, setVisible] = useState(false)
  const [scale, setScale] = useState(1)

  const handleOpenChange = (nextOpen) => {
    if (!nextOpen) {
      setVisible(false)
      setScale(1)
      return
    }
    setVisible(true)
  }

  useImperativeHandle(ref, () => ({
    open: () => {
      setScale(1)
      setVisible(true)
    }
  }))

  return (
    <Dialog open={visible} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-5xl">
        <DialogHeader>
          <DialogTitle>图片预览</DialogTitle>
        </DialogHeader>
        <div className="preview-container">
          <img
            src={imageUrl}
            alt="预览图片"
            className="preview-image"
            style={{ transform: `scale(${scale})` }}
          />
        </div>
        <DialogFooter className="justify-between">
          <div className="flex gap-2">
            <Button variant="secondary" size="icon" onClick={() => setScale((s) => s + 0.1)}>
              +
            </Button>
            <Button
              variant="secondary"
              size="icon"
              onClick={() => setScale((s) => (s > 0.1 ? s - 0.1 : s))}
            >
              -
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
})

ImageViewer.displayName = 'ImageViewer'

export default ImageViewer
