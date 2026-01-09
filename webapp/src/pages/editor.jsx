import React from 'react'
import { createRoot } from 'react-dom/client'
import DocEditor from './editor/DocEditor'
import '../assets/main.css'

createRoot(document.getElementById('app')).render(
  <React.StrictMode>
    <DocEditor />
  </React.StrictMode>
)
