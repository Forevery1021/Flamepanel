import axios from 'axios'

/** 后端统一错误响应（见 flame-kernel/src/core/error.rs） */
export interface ApiError {
  code: string
  message: string
  status: number
}

const api = axios.create({
  baseURL: '/api',
  timeout: 15000,
})

api.interceptors.request.use((config) => {
  const token = localStorage.getItem('token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

api.interceptors.response.use(
  (res) => res,
  (err) => {
    if (err.response?.status === 401) {
      localStorage.removeItem('token')
      localStorage.removeItem('username')
      localStorage.removeItem('role')
      if (window.location.pathname !== '/login') {
        window.location.href = '/login'
      }
    }

    // 规范化后端统一错误格式 {code, error, message}
    const data = err.response?.data
    if (data && typeof data === 'object' && 'error' in data) {
      const normalized = new Error(data.message || err.message) as Error & ApiError
      normalized.code = data.error
      normalized.message = data.message || err.message
      normalized.status = err.response.status
      return Promise.reject(normalized)
    }

    return Promise.reject(err)
  },
)

export default api
