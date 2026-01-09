import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useLogin } from '../store/providers'

const DefaultRedirect = () => {
  const navigate = useNavigate()
  const { canFile, canContacts, canAudit, canGroup, permissionsLoaded } = useLogin()

  useEffect(() => {
    if (!permissionsLoaded) return

    // Redirect to first available permission
    if (canFile) {
      navigate('/ui/file', { replace: true })
    } else if (canContacts) {
      navigate('/ui/contacts', { replace: true })
    } else if (canAudit) {
      navigate('/ui/audit', { replace: true })
    } else if (canGroup) {
      navigate('/ui/group', { replace: true })
    } else {
      // No permissions, redirect to settings as fallback
      navigate('/ui/settings', { replace: true })
    }
  }, [permissionsLoaded, canFile, canContacts, canAudit, canGroup, navigate])

  return null
}

export default DefaultRedirect
