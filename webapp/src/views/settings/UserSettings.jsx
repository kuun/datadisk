import React, { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import http from '../../lib/http'
import { alertError, alertSuccess } from '../../lib/utils'
import { useLogin } from '../../store/providers'
import { Button } from '../../components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card'
import { Input } from '../../components/ui/input'
import './UserSettings.css'

const UserSettings = () => {
  const navigate = useNavigate()
  const { loginUser, updateAvatar, userAvatar } = useLogin()
  const fileInputRef = useRef(null)
  const [userForm, setUserForm] = useState({
    username: '',
    fullName: '',
    email: '',
    phone: '',
    avatar: ''
  })
  const [pwdForm, setPwdForm] = useState({
    oldPassword: '',
    newPassword: '',
    confirmPassword: ''
  })

  const getUserInfo = async (username) => {
    if (!username) return
    try {
      const res = await http.get(`/api/user/info?username=${username}`)
      setUserForm({
        ...res.data.data,
        avatar: `/api/user/avatar/${username}`
      })
    } catch (error) {
      alertError('获取用户信息失败')
    }
  }

  useEffect(() => {
    setUserForm((prev) => ({ ...prev, username: loginUser }))
    getUserInfo(loginUser)
  }, [loginUser])

  const updateUserInfo = async () => {
    try {
      const res = await http.post('/api/user/update', userForm)
      if (res.data.code) {
        alertSuccess('基本信息更新成功')
      } else {
        alertError(res.data.message)
      }
    } catch (error) {
      alertError(error.response?.data?.message || '更新失败')
    }
  }

  const updatePassword = async () => {
    if (!pwdForm.oldPassword) {
      alertError('请输入旧密码')
      return
    }
    if (!pwdForm.newPassword) {
      alertError('请输入新密码')
      return
    }
    if (pwdForm.newPassword !== pwdForm.confirmPassword) {
      alertError('两次输入的密码不一致')
      return
    }

    try {
      const res = await http.post('/api/user/change-password', {
        oldPassword: pwdForm.oldPassword,
        newPassword: pwdForm.newPassword
      })
      if (res.data.code === true) {
        alertSuccess('密码修改成功，请重新登录')
        setPwdForm({ oldPassword: '', newPassword: '', confirmPassword: '' })
        // Logout and redirect to login page
        http.post('/api/logout').finally(() => navigate('/ui/login'))
      } else {
        alertError(res.data.message || '密码修改失败')
      }
    } catch (error) {
      alertError(error.response?.data?.message || '密码修改失败')
    }
  }

  const handleFileChange = async (event) => {
    const file = event.target.files?.[0]
    if (!file) return
    if (!file.type.startsWith('image/')) {
      alertError('头像必须是图片格式!')
      return
    }
    if (file.size / 1024 / 1024 >= 2) {
      alertError('头像大小不能超过 2MB!')
      return
    }
    const formData = new FormData()
    formData.append('avatar', file)
    formData.append('username', userForm.username)
    try {
      const res = await http.post('/api/user/upload/avatar', formData, {
        headers: { 'Content-Type': 'multipart/form-data' }
      })
      if (res.data.code === 0) {
        const timestamp = Date.now()
        const newAvatarUrl = `/api/user/avatar/${userForm.username}?t=${timestamp}`
        updateAvatar(timestamp)
        setUserForm((prev) => ({ ...prev, avatar: newAvatarUrl }))
        alertSuccess('头像更新成功')
      } else {
        alertError(res.data.message || '头像上传失败')
      }
    } catch (error) {
      alertError('头像上传失败')
    }
  }

  const handleDeleteAvatar = async () => {
    try {
      const res = await http.delete(`/api/user/avatar/${userForm.username}`)
      if (res.data.code === 0) {
        updateAvatar(null)
        setUserForm((prev) => ({ ...prev, avatar: '' }))
        alertSuccess('头像删除成功')
      } else {
        alertError(res.data.message || '删除失败')
      }
    } catch (error) {
      alertError('删除头像失败')
    }
  }

  return (
    <div className="settings-container">
      <div className="settings-form">
        <Card className="settings-card">
          <CardHeader>
            <CardTitle>基本信息</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="avatar-row">
              <div className="avatar-container">
                {userForm.avatar ? (
                  <img src={userForm.avatar} className="avatar" alt="avatar" />
                ) : (
                  <div className="avatar-placeholder">+</div>
                )}
                <input ref={fileInputRef} type="file" onChange={handleFileChange} />
              </div>
              <Button variant="secondary" className="btn-pill" onClick={() => fileInputRef.current?.click()}>
                上传头像
              </Button>
              <Button variant="destructive" className="btn-pill" onClick={handleDeleteAvatar}>
                删除头像
              </Button>
            </div>
            <div className="settings-grid">
              <label>用户名</label>
              <Input value={userForm.username} disabled />
              <label>姓名</label>
              <Input
                value={userForm.fullName || ''}
                onChange={(event) => setUserForm((prev) => ({ ...prev, fullName: event.target.value }))}
              />
              <label>邮箱</label>
              <Input
                value={userForm.email || ''}
                onChange={(event) => setUserForm((prev) => ({ ...prev, email: event.target.value }))}
              />
              <label>手机号码</label>
              <Input
                value={userForm.phone || ''}
                onChange={(event) => setUserForm((prev) => ({ ...prev, phone: event.target.value }))}
              />
            </div>
            <div className="settings-actions">
              <Button className="btn-pill" onClick={updateUserInfo}>
                保存修改
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
      <div className="settings-form">
        <Card className="settings-card">
          <CardHeader>
            <CardTitle>修改密码</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="settings-grid">
              <label>当前密码</label>
              <Input
                type="password"
                value={pwdForm.oldPassword}
                onChange={(event) => setPwdForm((prev) => ({ ...prev, oldPassword: event.target.value }))}
              />
              <label>新密码</label>
              <Input
                type="password"
                value={pwdForm.newPassword}
                onChange={(event) => setPwdForm((prev) => ({ ...prev, newPassword: event.target.value }))}
              />
              <label>确认新密码</label>
              <Input
                type="password"
                value={pwdForm.confirmPassword}
                onChange={(event) =>
                  setPwdForm((prev) => ({ ...prev, confirmPassword: event.target.value }))
                }
              />
            </div>
            <div className="settings-actions">
              <Button className="btn-pill" onClick={updatePassword}>
                修改密码
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

export default UserSettings
