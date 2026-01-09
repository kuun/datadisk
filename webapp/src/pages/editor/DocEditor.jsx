import React, { useEffect, useState } from 'react'
import { DocumentEditor } from '@onlyoffice/document-editor-react'
import http from '../../lib/http'
import './DocEditor.css'

const DocEditor = () => {
  const [docServerUrl, setDocServerUrl] = useState('')
  const [ready, setReady] = useState(false)
  const [loading, setLoading] = useState(true)
  const [errorMessage, setErrorMessage] = useState('无法加载编辑器，请稍后重试。')
  const [config, setConfig] = useState({})

  const getDocumentType = (contentType) => {
    if (contentType.includes('word')) return 'word'
    if (contentType.includes('excel')) return 'cell'
    if (contentType.includes('powerpoint')) return 'slide'
    return 'word'
  }

  const initEditor = async (sessionId) => {
    try {
      const response = await http.get(`/api/editing/query?session=${sessionId}`)
      const session = response.data

      if (!session.docServerUrl) {
        throw new Error('docServerUrl is empty from /api/editing/query response')
      }
      setDocServerUrl(session.docServerUrl)

      const fileName = session.filePath.split('/').pop() || ''
      const fileType = fileName.split('.').pop() || ''
      const folder = session.filePath.split('/').slice(0, -1).join('/')

      setConfig({
        document: {
          fileType,
          key: session.sessionId,
          title: fileName,
          url: `${session.datadiskUrl}/api/editing/download/${sessionId}`,
          info: {
            owner: session.userName,
            folder
          }
        },
        documentType: getDocumentType(session.contentType),
        editorConfig: {
          mode: 'edit',
          lang: 'zh',
          callbackUrl: `${session.datadiskUrl}/api/editing/save/${sessionId}`,
          user: {
            id: session.userName,
            name: session.displayName || session.fullName || session.userName
          },
          region: 'zh_CN',
          customization: {
            forcesave: true
          }
        },
        token: session.token,
        type: 'desktop'
      })
      setReady(true)
    } catch (error) {
      console.error('Failed to initialize editor:', error)
      setErrorMessage('编辑器初始化失败，请检查服务配置。')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search)
    const sessionId = urlParams.get('session')
    if (sessionId) {
      initEditor(sessionId)
    } else {
      setLoading(false)
      setErrorMessage('缺少编辑会话参数。')
    }
  }, [])

  const onDocumentReady = () => {
    console.log('Document is ready')
  }

  const onLoadComponentError = (errorCode, errorDescription) => {
    console.log(errorCode, errorDescription)
  }

  if (!ready) {
    return (
      <div className="editor-status">
        {loading ? <div>编辑器初始化中...</div> : <div className="editor-error">{errorMessage}</div>}
      </div>
    )
  }

  return (
    <DocumentEditor
      id="docEditor"
      documentServerUrl={docServerUrl}
      config={config}
      events_onDocumentReady={onDocumentReady}
      onLoadComponentError={onLoadComponentError}
    />
  )
}

export default DocEditor
