import axios from 'axios'

const http = axios.create()

http.interceptors.response.use(
  (response) => {
    // Check business logic error (code: false means error)
    if (response.data && response.data.code === false) {
      const error = new Error(response.data.message || '请求失败')
      error.response = response
      error.isBusinessError = true
      return Promise.reject(error)
    }
    return response
  },
  (error) => {
    if (error?.response?.status === 401) {
      window.location.href = '/ui/login'
    }
    return Promise.reject(error)
  }
)

export default http
