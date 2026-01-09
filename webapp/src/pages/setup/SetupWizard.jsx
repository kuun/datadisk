import React, { useEffect, useState } from 'react'
import http from '../../lib/http'
import { alertError, alertSuccess } from '../../lib/utils'
import { Button } from '../../components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card'
import { Input } from '../../components/ui/input'
import './SetupWizard.css'

const steps = [
  { title: '数据库配置', description: '配置数据库连接信息' },
  { title: '创建管理员', description: '注册第一个用户作为管理员' },
  { title: '完成配置', description: '系统初始化完成' }
]

const SetupWizard = () => {
  const [activeStep, setActiveStep] = useState(0)
  const [loading, setLoading] = useState(false)
  const [dbForm, setDbForm] = useState({
    type: 'postgres',
    host: 'localhost',
    port: '5432',
    database: '',
    username: '',
    password: ''
  })
  const [adminForm, setAdminForm] = useState({
    username: '',
    password: '',
    confirmPassword: '',
    email: ''
  })

  useEffect(() => {
    const checkStatus = async () => {
      try {
        const res = await http.get('/api/setup/status')
        if (res.data.initialized) {
          window.location.href = '/ui/login'
        }
      } catch (error) {
        console.error('检查系统状态失败:', error)
      }
    }
    checkStatus()
  }, [])

  const testConnection = async () => {
    if (!dbForm.host || !dbForm.port || !dbForm.database || !dbForm.username || !dbForm.password) {
      alertError('请填写完整的数据库配置信息')
      return
    }
    try {
      setLoading(true)
      const res = await http.post('/api/setup/test-db', dbForm)
      if (res.data.code === 0) {
        alertSuccess('数据库连接测试成功')
      } else {
        alertError(res.data.message || '数据库连接测试失败')
      }
    } catch (error) {
      alertError(`数据库连接测试失败: ${error.response?.data?.message || error.message}`)
    } finally {
      setLoading(false)
    }
  }

  const handleNext = async () => {
    if (activeStep === 0) {
      if (!dbForm.host || !dbForm.port || !dbForm.database || !dbForm.username || !dbForm.password) {
        alertError('请填写完整的数据库配置信息')
        return
      }
      try {
        setLoading(true)
        const res = await http.post('/api/setup/init/db', dbForm)
        if (res.data.code === 0) {
          setActiveStep(1)
        } else {
          alertError(res.data.message || '保存数据库配置失败')
        }
      } catch (error) {
        alertError(`保存数据库配置失败: ${error.response?.data?.message || error.message}`)
      } finally {
        setLoading(false)
      }
    } else if (activeStep === 1) {
      if (!adminForm.username || adminForm.username.length < 3) {
        alertError('用户名至少需要3个字符')
        return
      }
      if (!adminForm.password || adminForm.password.length < 6) {
        alertError('密码至少需要6个字符')
        return
      }
      if (adminForm.password !== adminForm.confirmPassword) {
        alertError('两次输入的密码不一致')
        return
      }
      try {
        setLoading(true)
        const res = await http.post('/api/setup/init/user', adminForm)
        if (res.data.code === 0) {
          setActiveStep(2)
          alertSuccess('系统初始化成功')
        } else {
          alertError(res.data.message)
        }
      } catch (error) {
        alertError('系统初始化失败')
      } finally {
        setLoading(false)
      }
    }
  }

  return (
    <div className="setup-container">
      <Card className="setup-card">
        <CardHeader>
          <div className="card-header">
            <img src="/assets/img/datadisk-logo.png" className="logo" alt="logo" />
            <CardTitle>系统初始化配置</CardTitle>
          </div>
        </CardHeader>
        <CardContent>
          <div className="steps">
            {steps.map((step, index) => (
              <div key={step.title} className={`step ${index === activeStep ? 'active' : ''}`}>
                <div className="step-title">{step.title}</div>
                <div className="step-desc">{step.description}</div>
              </div>
            ))}
          </div>

          {activeStep === 0 && (
            <div className="step-content">
              <label>数据库类型</label>
              <select
                value={dbForm.type}
                onChange={(event) => setDbForm((prev) => ({ ...prev, type: event.target.value }))}
              >
                <option value="postgres">PostgreSQL</option>
              </select>
              <label>主机地址</label>
              <Input value={dbForm.host} onChange={(event) => setDbForm((prev) => ({ ...prev, host: event.target.value }))} />
              <label>端口</label>
              <Input value={dbForm.port} onChange={(event) => setDbForm((prev) => ({ ...prev, port: event.target.value }))} />
              <label>数据库名</label>
              <Input value={dbForm.database} onChange={(event) => setDbForm((prev) => ({ ...prev, database: event.target.value }))} />
              <label>用户名</label>
              <Input value={dbForm.username} onChange={(event) => setDbForm((prev) => ({ ...prev, username: event.target.value }))} />
              <label>密码</label>
              <Input type="password" value={dbForm.password} onChange={(event) => setDbForm((prev) => ({ ...prev, password: event.target.value }))} />
              <div className="step-actions">
                <Button variant="secondary" onClick={testConnection} disabled={loading}>
                  测试连接
                </Button>
                <Button onClick={handleNext} disabled={loading}>
                  下一步
                </Button>
              </div>
            </div>
          )}

          {activeStep === 1 && (
            <div className="step-content">
              <p className="form-hint">第一个注册的用户将成为系统管理员</p>
              <label>用户名</label>
              <Input
                value={adminForm.username}
                placeholder="请输入用户名（至少3个字符）"
                onChange={(event) => setAdminForm((prev) => ({ ...prev, username: event.target.value }))}
              />
              <label>密码</label>
              <Input
                type="password"
                value={adminForm.password}
                placeholder="请输入密码（至少6个字符）"
                onChange={(event) => setAdminForm((prev) => ({ ...prev, password: event.target.value }))}
              />
              <label>确认密码</label>
              <Input
                type="password"
                value={adminForm.confirmPassword}
                placeholder="请再次输入密码"
                onChange={(event) => setAdminForm((prev) => ({ ...prev, confirmPassword: event.target.value }))}
              />
              <label>邮箱</label>
              <Input
                type="email"
                value={adminForm.email}
                placeholder="可选"
                onChange={(event) => setAdminForm((prev) => ({ ...prev, email: event.target.value }))}
              />
              <div className="step-actions">
                <Button variant="secondary" onClick={() => setActiveStep(0)}>
                  上一步
                </Button>
                <Button onClick={handleNext} disabled={loading}>
                  创建管理员
                </Button>
              </div>
            </div>
          )}

          {activeStep === 2 && (
            <div className="step-content">
              <div className="success-message">
                <h3>系统初始化完成</h3>
                <p>现在您可以使用配置的管理员账号登录系统了</p>
              </div>
              <div className="step-actions">
                <Button onClick={() => (window.location.href = '/ui/login')}>前往登录</Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}

export default SetupWizard
