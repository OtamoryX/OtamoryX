<template>
  <div class="min-h-screen w-full flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
    <div class="max-w-md w-full space-y-8">
      <div>
        <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
          欢迎使用 OtamoryX
        </h2>
        <p class="mt-2 text-center text-sm text-gray-600">
          {{ isInitializing ? '系统初始化' : '登录到您的漫画图书馆' }}
        </p>
      </div>

      <!-- 系统状态检查 -->
      <div v-if="systemStatusLoading" class="text-center">
        <div class="text-gray-500">检查系统状态...</div>
      </div>

      <!-- 初始化表单 -->
      <form v-else-if="isInitializing" @submit.prevent="handleInitialize" class="mt-8 space-y-6">
        <div class="rounded-md shadow-sm -space-y-px">
          <div>
            <label for="username" class="sr-only">用户名</label>
            <input
              id="username"
              v-model="initForm.username"
              name="username"
              type="text"
              required
              class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
              placeholder="管理员用户名"
            />
          </div>
          <div>
            <label for="email" class="sr-only">邮箱</label>
            <input
              id="email"
              v-model="initForm.email"
              name="email"
              type="email"
              class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
              placeholder="邮箱地址（可选）"
            />
          </div>
          <div>
            <label for="password" class="sr-only">密码</label>
            <input
              id="password"
              v-model="initForm.password"
              name="password"
              type="password"
              required
              class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
              placeholder="密码"
            />
          </div>
        </div>

        <div>
          <button
            type="submit"
            :disabled="initLoading"
            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {{ initLoading ? '初始化中...' : '初始化系统' }}
          </button>
        </div>
      </form>

      <!-- 登录表单 -->
      <form v-else @submit.prevent="handleLogin" class="mt-8 space-y-6">
        <div class="rounded-md shadow-sm -space-y-px">
          <div>
            <label for="login-username" class="sr-only">用户名</label>
            <input
              id="login-username"
              v-model="loginForm.username"
              name="username"
              type="text"
              required
              class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
              placeholder="用户名"
            />
          </div>
          <div>
            <label for="login-password" class="sr-only">密码</label>
            <input
              id="login-password"
              v-model="loginForm.password"
              name="password"
              type="password"
              required
              class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
              placeholder="密码"
            />
          </div>
        </div>

        <div>
          <button
            type="submit"
            :disabled="loginLoading"
            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {{ loginLoading ? '登录中...' : '登录' }}
          </button>
        </div>
      </form>

      <!-- 错误提示 -->
      <div v-if="error" class="bg-red-50 border border-red-200 rounded-lg p-4">
        <div class="text-red-800 text-sm">{{ error }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { getSystemStatus, initializeSystem, login } from '@/utils/api'

const router = useRouter()
const authStore = useAuthStore()

const systemStatusLoading = ref(true)
const isInitializing = ref(false)
const initLoading = ref(false)
const loginLoading = ref(false)
const error = ref('')

const initForm = ref({
  username: '',
  email: '',
  password: ''
})

const loginForm = ref({
  username: '',
  password: ''
})

const checkSystemStatus = async () => {
  try {
    const status = await getSystemStatus()
    isInitializing.value = !status.initialized
  } catch (err) {
    console.error('Failed to check system status:', err)
    error.value = '无法连接到服务器'
  } finally {
    systemStatusLoading.value = false
  }
}

const handleInitialize = async () => {
  if (!initForm.value.username || !initForm.value.password) {
    error.value = '请填写用户名和密码'
    return
  }

  initLoading.value = true
  error.value = ''

  try {
    const response = await initializeSystem(initForm.value)
    await authStore.login(response.token)
    router.push('/library')
  } catch (err: any) {
    error.value = err.response?.data?.message || '初始化失败'
  } finally {
    initLoading.value = false
  }
}

const handleLogin = async () => {
  if (!loginForm.value.username || !loginForm.value.password) {
    error.value = '请填写用户名和密码'
    return
  }

  loginLoading.value = true
  error.value = ''

  try {
    const response = await login(loginForm.value)
    await authStore.login(response.token)
    router.push('/library')
  } catch (err: any) {
    error.value = err.response?.data?.message || '登录失败'
  } finally {
    loginLoading.value = false
  }
}

onMounted(() => {
  // 如果已经登录，直接跳转
  if (authStore.isAuthenticated) {
    router.push('/library')
    return
  }
  
  checkSystemStatus()
})
</script>

<style scoped>
/* 确保登录页面占满全屏 */
.min-h-screen {
  min-height: 100vh;
  width: 100vw;
}
</style>