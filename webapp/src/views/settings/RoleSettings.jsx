import React from 'react'
import RoleManager from '../../components/RoleManager'
import { useLogin } from '../../store/providers'

const RoleSettings = () => {
  const { canRole } = useLogin()

  if (!canRole) {
    return <div>权限不足</div>
  }

  return <RoleManager />
}

export default RoleSettings
