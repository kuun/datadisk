import React from 'react'
import { createRoot } from 'react-dom/client'
import { Toaster } from 'sonner'
import SetupWizard from './setup/SetupWizard'
import '../assets/main.css'

createRoot(document.getElementById('app')).render(
  <React.StrictMode>
    <SetupWizard />
    <Toaster richColors />
  </React.StrictMode>
)
