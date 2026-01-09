import React from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { Toaster } from 'sonner'
import App from './App'
import './assets/main.css'
import {
  ContactsProvider,
  FileProvider,
  GroupsProvider,
  LoginProvider,
  TaskProvider
} from './store/providers'

createRoot(document.getElementById('app')).render(
  <React.StrictMode>
    <BrowserRouter>
      <LoginProvider>
        <TaskProvider>
          <ContactsProvider>
            <GroupsProvider>
              <FileProvider>
                <App />
                <Toaster richColors />
              </FileProvider>
            </GroupsProvider>
          </ContactsProvider>
        </TaskProvider>
      </LoginProvider>
    </BrowserRouter>
  </React.StrictMode>
)
