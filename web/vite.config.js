import { defineConfig } from 'vite'
import { sveltekit } from '@sveltejs/kit/vite'
import { execSync } from 'child_process'

const cmd = (c) => { try { return execSync(c, { encoding: 'utf8' }).trim() } catch { return '' } }
const hash = cmd('git rev-parse --short HEAD')
const dirty = cmd('git status --porcelain') !== ''
const gitHash = hash ? (dirty ? `${hash}-c` : hash) : 'unknown'

export default defineConfig({
  plugins: [sveltekit()],
  define: {
    __BUILD_GIT_HASH__: JSON.stringify(gitHash)
  },
  test: {
    environment: 'node',
    include: ['src/lib/stores/**/*.test.js']
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3000',
        configure: (proxy) => {
          proxy.on('proxyReq', (proxyReq, req) => {
            proxyReq.setHeader('X-Mazu-IP', req.socket.remoteAddress)
          })
        }
      },
      '/ws': {
        target: 'ws://127.0.0.1:3000',
        ws: true,
        configure: (proxy) => {
          proxy.on('proxyReqWs', (proxyReq, req) => {
            proxyReq.setHeader('X-Mazu-IP', req.socket.remoteAddress)
          })
        }
      }
    }
  }
})
