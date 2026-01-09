import { toast } from 'sonner'
import { t } from './i18n'

export const formatFileSize = (fileSize) => {
  if (fileSize < 1024) {
    return `${fileSize}B`
  }
  if (fileSize < 1024 * 1024) {
    return `${(fileSize / 1024).toFixed(2)}KB`
  }
  if (fileSize < 1024 * 1024 * 1024) {
    return `${(fileSize / (1024 * 1024)).toFixed(2)}MB`
  }
  if (fileSize < 1024 * 1024 * 1024 * 1024) {
    return `${(fileSize / (1024 * 1024 * 1024)).toFixed(2)}GB`
  }
  return `${(fileSize / (1024 * 1024 * 1024 * 1024)).toFixed(2)}PB`
}

export const unitToByte = (size, unit) => {
  const multipliers = {
    KB: 1024,
    MB: 1024 * 1024,
    GB: 1024 * 1024 * 1024,
    TB: 1024 * 1024 * 1024 * 1024,
    PB: 1024 * 1024 * 1024 * 1024 * 1024
  }
  return size * (multipliers[unit] || 1)
}

export const byteToUnit = (size) => {
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(0)} KB`
  if (size < 1024 * 1024 * 1024) return `${(size / (1024 * 1024)).toFixed(0)} MB`
  if (size < 1024 * 1024 * 1024 * 1024) return `${(size / (1024 * 1024 * 1024)).toFixed(0)} GB`
  return `${(size / (1024 * 1024 * 1024 * 1024)).toFixed(0)} TB`
}

export const alertError = (message) => {
  toast.error(message)
}

export const alertSuccess = (message) => {
  toast.success(message)
}

export const translate = t
