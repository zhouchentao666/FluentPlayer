import LogtoClient from '@logto/browser'

const config = {
  appId: '2a22nn23flw9nyrwi6jw9',
  endpoint: 'https://auth.shiqianjiang.cn/',
  redirectUri: 'http://localhost:1420',
  postLogoutRedirectUri: 'http://localhost:1420',
  instance: null as LogtoClient | null,
  resources: ['https://api.ceru.shiqianjiang.cn/api', 'https://api.qz.shiqianjiang.cn/api']
}

export default config
