import React, { useEffect, useState } from 'react'
import http from '../lib/http'
import { alertError } from '../lib/utils'
import { t } from '../lib/i18n'
import './LoginView.css'

const LoginView = () => {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')

  const login = () => {
    if (!username) {
      alertError(t('Username cannot be left blank'))
      return
    }
    if (!password) {
      alertError(t('Password cannot be left blank'))
      return
    }
    http
      .post('/api/login', {
        username,
        password
      })
      .then(() => {
        window.location.href = '/ui/file'
      })
      .catch((error) => {
        alertError(t(error.response?.data?.error || '登录失败'))
      })
  }

  useEffect(() => {
    const checkSetup = async () => {
      try {
        const res = await http.get('/api/setup/status')
        if (!res.data.initialized) {
          window.location.href = '/setup.html'
        }
      } catch (error) {
        console.error('检查系统状态失败:', error)
      }
    }
    checkSetup()
  }, [])

  return (
    <div className="LoginPageContainer">
      <div className="LoginPageInnerContainer">
        <div className="ImageContainer">
          <img src="/assets/img/login-background.jpg" className="background-image" alt="背景" />
        </div>
        <div className="LoginFormContainer">
          <div className="LoginFormInnerContainer">
            <div className="LogoContainer">
              <img src="/assets/img/datadisk-logo.png" className="logo" alt="Logo" />
            </div>
            <header className="header">{t('login')}</header>
            <header className="subHeader">{t('Welcome')}</header>

            <form
              onSubmit={(event) => {
                event.preventDefault()
                login()
              }}
            >
              <div className="inputContainer">
                <label className="label" htmlFor="userName">
                  <img src="/assets/img/email.png" className="labelIcon" alt="" />
                  <span>{t('username')}*</span>
                </label>
                <input
                  type="text"
                  className="input"
                  id="userName"
                  placeholder={t('placeholder.username')}
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                />
              </div>
              <div className="inputContainer">
                <label className="label" htmlFor="password">
                  <img src="/assets/img/password.png" className="labelIcon" alt="" />
                  <span>{t('password')}*</span>
                </label>
                <input
                  type="password"
                  className="input"
                  id="password"
                  placeholder={t('placeholder.password')}
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                />
              </div>
              <div className="OptionsContainer">
                <div className="checkboxContainer">
                  <input type="checkbox" id="RememberMe" className="checkbox" />
                  <label htmlFor="RememberMe">{t('remember password')}</label>
                </div>
                <a href="#" className="ForgotPasswordLink">
                  {t('forgot password')}?
                </a>
              </div>
              <button className="LoginButton" type="submit">
                {t('login')}
              </button>
            </form>
            <div className="copyright">CopyRight © Datadisk Team</div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default LoginView
