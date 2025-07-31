<template>
  <div class="settings-view w-full h-full p-6">
    <h1 class="text-2xl font-bold text-gray-900 mb-6">系统设置</h1>
    
    <div class="w-full max-w-6xl mx-auto">
      <!-- 标签导航 -->
      <div class="border-b border-gray-200 mb-6">
        <nav class="-mb-px flex space-x-8">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            @click="activeTab = tab.id"
            :class="[
              'py-2 px-1 border-b-2 font-medium text-sm',
              activeTab === tab.id
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            ]"
          >
            {{ tab.name }}
          </button>
        </nav>
      </div>

      <!-- 系统配置 -->
      <div v-if="activeTab === 'system'" class="space-y-6">
        <!-- 漫画库路径设置 -->
        <div class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">漫画库配置</h2>
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                漫画库路径
              </label>
              <div class="flex space-x-2">
                <input
                  v-model="systemSettings.comicsPath"
                  type="text"
                  placeholder="/path/to/comics"
                  class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <button
                  @click="selectComicsPath"
                  class="px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700"
                >
                  浏览
                </button>
              </div>
              <p class="mt-1 text-sm text-gray-500">指定存储漫画文件的目录路径</p>
            </div>

            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                支持的文件格式
              </label>
              <div class="flex flex-wrap gap-2">
                <span
                  v-for="format in systemSettings.supportedFormats"
                  :key="format"
                  class="px-2 py-1 bg-blue-100 text-blue-800 text-xs rounded-full"
                >
                  .{{ format }}
                </span>
              </div>
            </div>

            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                最大文件大小 (MB)
              </label>
              <input
                v-model.number="systemSettings.maxFileSize"
                type="number"
                min="1"
                max="1000"
                class="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <p class="mt-1 text-sm text-gray-500">单个漫画文件的最大大小限制</p>
            </div>
          </div>
        </div>

        <!-- 图像缓存设置 -->
        <div class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">图像缓存配置</h2>
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                缓存路径
              </label>
              <div class="flex space-x-2">
                <input
                  v-model="cacheSettings.cachePath"
                  type="text"
                  placeholder="/path/to/cache"
                  class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <button
                  @click="selectCachePath"
                  class="px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700"
                >
                  浏览
                </button>
              </div>
              <p class="mt-1 text-sm text-gray-500">指定图像缓存文件的存储目录路径</p>
            </div>

            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                缓存大小 (GB)
              </label>
              <input
                v-model.number="cacheSettings.maxSize"
                type="number"
                min="0.1"
                max="10"
                step="0.1"
                class="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <p class="mt-1 text-sm text-gray-500">图像缓存的最大存储空间</p>
            </div>

            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                图像质量
              </label>
              <div class="flex items-center space-x-4">
                <input
                  v-model.number="cacheSettings.quality"
                  type="range"
                  min="1"
                  max="100"
                  class="flex-1"
                />
                <span class="text-sm text-gray-600 w-12">{{ cacheSettings.quality }}%</span>
              </div>
              <p class="mt-1 text-sm text-gray-500">缓存图像的压缩质量</p>
            </div>

            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                输出格式
              </label>
              <select
                v-model="cacheSettings.format"
                class="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="JPEG">JPEG</option>
                <option value="PNG">PNG</option>
                <option value="WebP">WebP</option>
              </select>
            </div>
          </div>
        </div>

        <!-- 自动扫描设置 -->
        <div class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">自动扫描配置</h2>
          <div class="space-y-4">
            <div class="flex items-center">
              <input
                id="auto-scan"
                v-model="scanSettings.enabled"
                type="checkbox"
                class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              />
              <label for="auto-scan" class="ml-2 block text-sm text-gray-900">
                启用自动扫描
              </label>
            </div>

            <div class="flex items-center">
              <input
                id="recursive-scan"
                v-model="scanSettings.recursive"
                type="checkbox"
                class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              />
              <label for="recursive-scan" class="ml-2 block text-sm text-gray-900">
                递归扫描子目录
              </label>
            </div>

            <div class="flex items-center">
              <input
                id="ignore-hidden"
                v-model="scanSettings.ignoreHidden"
                type="checkbox"
                class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              />
              <label for="ignore-hidden" class="ml-2 block text-sm text-gray-900">
                忽略隐藏文件
              </label>
            </div>

            <div class="flex items-center">
              <input
                id="realtime-monitoring"
                v-model="scanSettings.realtimeMonitoring"
                type="checkbox"
                class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              />
              <label for="realtime-monitoring" class="ml-2 block text-sm text-gray-900">
                实时文件监控
              </label>
            </div>
          </div>
        </div>

        <div class="flex justify-end">
          <button
            @click="saveSystemSettings"
            :disabled="systemLoading"
            class="px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {{ systemLoading ? '保存中...' : '保存设置' }}
          </button>
        </div>
      </div>

      <!-- 用户管理 -->
      <div v-if="activeTab === 'users'" class="space-y-6">
        <div class="flex justify-between items-center">
          <h2 class="text-lg font-medium text-gray-900">用户管理</h2>
          <button
            @click="showCreateUserModal = true"
            class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            创建用户
          </button>
        </div>

        <div v-if="usersLoading" class="text-center py-8">
          <div class="text-gray-500">加载中...</div>
        </div>

        <div v-else class="bg-white shadow overflow-hidden sm:rounded-md">
          <ul class="divide-y divide-gray-200">
            <li v-for="user in users" :key="user.id" class="px-6 py-4">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-4">
                  <div class="flex-shrink-0">
                    <div class="h-10 w-10 rounded-full bg-gray-300 flex items-center justify-center">
                      <span class="text-sm font-medium text-gray-700">
                        {{ user.username.charAt(0).toUpperCase() }}
                      </span>
                    </div>
                  </div>
                  <div>
                    <p class="text-sm font-medium text-gray-900">{{ user.username }}</p>
                    <p class="text-sm text-gray-500">{{ user.email || '未设置邮箱' }}</p>
                    <p class="text-xs text-gray-400">创建于 {{ formatDate(user.createdAt) }}</p>
                  </div>
                </div>
                <div class="flex items-center space-x-2">
                  <button
                    @click="editUser(user)"
                    class="px-3 py-1 text-sm text-blue-600 hover:text-blue-800 border border-blue-300 rounded hover:bg-blue-50"
                  >
                    编辑
                  </button>
                  <button
                    @click="confirmDeleteUser(user)"
                    class="px-3 py-1 text-sm text-red-600 hover:text-red-800 border border-red-300 rounded hover:bg-red-50"
                  >
                    删除
                  </button>
                </div>
              </div>
            </li>
          </ul>
        </div>
      </div>

      <!-- 插件管理 -->
      <div v-if="activeTab === 'plugins'" class="space-y-6">
        <div class="flex justify-between items-center">
          <h2 class="text-lg font-medium text-gray-900">插件管理</h2>
          <button
            @click="showInstallPluginModal = true"
            class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            安装插件
          </button>
        </div>

        <div v-if="pluginsLoading" class="text-center py-8">
          <div class="text-gray-500">加载中...</div>
        </div>

        <div v-else-if="plugins?.length === 0" class="text-center py-8">
          <div class="text-gray-500">暂无已安装的插件</div>
        </div>

        <div v-else class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          <div
            v-for="plugin in plugins"
            :key="plugin.id"
            class="bg-white rounded-lg shadow border p-6"
          >
            <div class="flex items-start justify-between mb-4">
              <div class="flex-1">
                <h3 class="text-lg font-semibold text-gray-900">{{ plugin.name }}</h3>
                <p class="text-sm text-gray-500">v{{ plugin.version }}</p>
              </div>
              <span
                :class="[
                  'px-2 py-1 text-xs rounded-full',
                  plugin.enabled 
                    ? 'bg-green-100 text-green-800' 
                    : 'bg-gray-100 text-gray-800'
                ]"
              >
                {{ plugin.enabled ? '已启用' : '已禁用' }}
              </span>
            </div>

            <p v-if="plugin.description" class="text-sm text-gray-600 mb-4">
              {{ plugin.description }}
            </p>

            <div class="text-xs text-gray-400 mb-4">
              安装于 {{ formatDate(plugin.installedAt) }}
            </div>

            <div class="flex justify-between items-center">
              <button
                @click="handleTogglePlugin(plugin)"
                :class="[
                  'px-3 py-1 text-sm rounded border',
                  plugin.enabled
                    ? 'text-red-600 border-red-300 hover:bg-red-50'
                    : 'text-green-600 border-green-300 hover:bg-green-50'
                ]"
              >
                {{ plugin.enabled ? '禁用' : '启用' }}
              </button>
              
              <div class="flex items-center space-x-2">
                <button
                  v-if="plugin.config"
                  @click="configurePlugin(plugin)"
                  class="px-3 py-1 text-sm text-blue-600 border border-blue-300 rounded hover:bg-blue-50"
                >
                  配置
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- AI自动标签 -->
      <div v-if="activeTab === 'ai'" class="space-y-6">
        <div class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">AI自动标签配置</h2>
          <div class="space-y-4">
            <div class="flex items-center">
              <input
                id="ai-enabled"
                v-model="aiSettings.enabled"
                type="checkbox"
                class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              />
              <label for="ai-enabled" class="ml-2 block text-sm text-gray-900">
                启用AI自动标签
              </label>
            </div>

            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                自动应用阈值
              </label>
              <div class="flex items-center space-x-4">
                <input
                  v-model.number="aiSettings.autoApplyThreshold"
                  type="range"
                  min="0.1"
                  max="1.0"
                  step="0.1"
                  class="flex-1"
                />
                <span class="text-sm text-gray-600 w-12">{{ (aiSettings.autoApplyThreshold * 100).toFixed(0) }}%</span>
              </div>
              <p class="mt-1 text-sm text-gray-500">置信度达到此阈值的AI标签将自动应用</p>
            </div>

            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                处理调度
              </label>
              <select
                v-model="aiSettings.processingSchedule"
                class="w-48 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="immediate">立即处理</option>
                <option value="batch">批量处理</option>
                <option value="off-peak">非高峰时段</option>
              </select>
            </div>

            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                最大并发任务数
              </label>
              <input
                v-model.number="aiSettings.maxConcurrentTasks"
                type="number"
                min="1"
                max="10"
                class="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        </div>

        <!-- AI状态监控 -->
        <div v-if="aiStatus" class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">AI处理状态</h2>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div class="text-center p-4 bg-gray-50 rounded-lg">
              <div class="text-2xl font-bold text-blue-600">{{ aiStatus.queueSize }}</div>
              <div class="text-sm text-gray-600">队列中</div>
            </div>
            <div class="text-center p-4 bg-gray-50 rounded-lg">
              <div class="text-2xl font-bold text-green-600">{{ aiStatus.processingCount }}</div>
              <div class="text-sm text-gray-600">处理中</div>
            </div>
            <div class="text-center p-4 bg-gray-50 rounded-lg">
              <div class="text-2xl font-bold text-purple-600">{{ aiStatus.completedToday }}</div>
              <div class="text-sm text-gray-600">今日完成</div>
            </div>
            <div class="text-center p-4 bg-gray-50 rounded-lg">
              <div class="text-2xl font-bold text-red-600">{{ aiStatus.failedToday }}</div>
              <div class="text-sm text-gray-600">今日失败</div>
            </div>
          </div>
        </div>

        <div class="flex justify-end">
          <button
            @click="saveAISettings"
            :disabled="aiLoading"
            class="px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {{ aiLoading ? '保存中...' : '保存AI设置' }}
          </button>
        </div>
      </div>
    </div>

    <!-- 创建用户模态框 -->
    <div
      v-if="showCreateUserModal"
      class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
      @click="showCreateUserModal = false"
    >
      <div
        class="bg-white rounded-lg p-6 max-w-md w-full mx-4"
        @click.stop
      >
        <h3 class="text-lg font-bold mb-4 text-gray-900">创建用户</h3>
        <form @submit.prevent="handleCreateUser" class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">用户名</label>
            <input
              v-model="createUserForm.username"
              type="text"
              required
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">邮箱</label>
            <input
              v-model="createUserForm.email"
              type="email"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">密码</label>
            <input
              v-model="createUserForm.password"
              type="password"
              required
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div class="flex justify-end space-x-3 mt-6">
            <button
              type="button"
              @click="showCreateUserModal = false"
              class="px-4 py-2 text-gray-600 hover:text-gray-800"
            >
              取消
            </button>
            <button
              type="submit"
              :disabled="createUserLoading"
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              {{ createUserLoading ? '创建中...' : '创建' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 安装插件模态框 -->
    <div
      v-if="showInstallPluginModal"
      class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
      @click="showInstallPluginModal = false"
    >
      <div
        class="bg-white rounded-lg p-6 max-w-md w-full mx-4"
        @click.stop
      >
        <h3 class="text-lg font-bold mb-4 text-gray-900">安装插件</h3>
        <form @submit.prevent="handleInstallPlugin" class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              选择插件文件 (.zip)
            </label>
            <input
              ref="pluginFileInput"
              type="file"
              accept=".zip"
              required
              class="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-full file:border-0 file:text-sm file:font-semibold file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
            />
          </div>
          <div class="flex justify-end space-x-3">
            <button
              type="button"
              @click="showInstallPluginModal = false"
              class="px-4 py-2 text-gray-600 hover:text-gray-800"
            >
              取消
            </button>
            <button
              type="submit"
              :disabled="installPluginLoading"
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              {{ installPluginLoading ? '安装中...' : '安装' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import { 
  getSettings, 
  updateSettings, 
  getUsers, 
  createUser, 
  deleteUser,
  getPlugins,
  installPlugin,
  togglePlugin,
  getAISettings,
  updateAISettings,
  getAIStatus
} from '@/utils/api'
import type { SystemSettings, User, CreateUserRequest, Plugin, AISettings, AIStatus } from '@/types/api'

const queryClient = useQueryClient()

// 标签页管理
const activeTab = ref('system')
const tabs = [
  { id: 'system', name: '系统配置' },
  { id: 'users', name: '用户管理' },
  { id: 'plugins', name: '插件管理' },
  { id: 'ai', name: 'AI自动标签' }
]

// 系统设置
const systemSettings = ref<SystemSettings>({
  comicsPath: '',
  supportedFormats: ['cbz', 'cbr', 'cb7', 'zip', 'rar'],
  maxFileSize: 100,
  imageCacheSize: 1024,
  scanOnStartup: true
})

const cacheSettings = ref({
  cachePath: '',
  maxSize: 1.0,
  quality: 85,
  format: 'WebP'
})

const scanSettings = ref({
  enabled: true,
  recursive: true,
  ignoreHidden: true,
  realtimeMonitoring: false
})

const systemLoading = ref(false)

// AI设置
const aiSettings = ref<AISettings>({
  enabled: false,
  autoApplyThreshold: 0.8,
  processingSchedule: 'batch',
  maxConcurrentTasks: 2,
  enabledAnalyzers: []
})

const aiLoading = ref(false)

// 用户管理
const showCreateUserModal = ref(false)
const createUserForm = ref({
  username: '',
  email: '',
  password: ''
})
const createUserLoading = ref(false)

// 插件管理
const showInstallPluginModal = ref(false)
const pluginFileInput = ref<HTMLInputElement>()
const installPluginLoading = ref(false)

// 查询数据
const { data: users, isLoading: usersLoading } = useQuery({
  queryKey: ['users'],
  queryFn: getUsers,
  enabled: () => activeTab.value === 'users'
})

const { data: plugins, isLoading: pluginsLoading } = useQuery({
  queryKey: ['plugins'],
  queryFn: getPlugins,
  enabled: () => activeTab.value === 'plugins'
})

const { data: aiStatus } = useQuery({
  queryKey: ['ai-status'],
  queryFn: getAIStatus,
  enabled: () => activeTab.value === 'ai',
  refetchInterval: 5000
})

// 系统设置相关方法
const selectComicsPath = () => {
  // TODO: 实现文件夹选择
  console.log('Select comics path')
}

const selectCachePath = () => {
  // TODO: 实现文件夹选择
  console.log('Select cache path')
}

const saveSystemSettings = async () => {
  systemLoading.value = true
  try {
    await updateSettings({
      ...systemSettings.value,
      maxFileSize: systemSettings.value.maxFileSize * 1024 * 1024, // 转换为字节
      imageCacheSize: cacheSettings.value.maxSize * 1024 * 1024 * 1024, // 转换为字节
      imageCachePath: cacheSettings.value.cachePath,
      imageCacheQuality: cacheSettings.value.quality,
      imageCacheFormat: cacheSettings.value.format
    })
    alert('系统设置已保存')
  } catch (error) {
    console.error('保存设置失败:', error)
    alert('保存失败')
  } finally {
    systemLoading.value = false
  }
}

// AI设置相关方法
const saveAISettings = async () => {
  aiLoading.value = true
  try {
    await updateAISettings(aiSettings.value)
    alert('AI设置已保存')
  } catch (error) {
    console.error('保存AI设置失败:', error)
    alert('保存失败')
  } finally {
    aiLoading.value = false
  }
}

// 用户管理相关方法
const handleCreateUser = async () => {
  if (!createUserForm.value.username || !createUserForm.value.password) return
  
  createUserLoading.value = true
  try {
    await createUser(createUserForm.value)
    queryClient.invalidateQueries({ queryKey: ['users'] })
    showCreateUserModal.value = false
    createUserForm.value = { username: '', email: '', password: '' }
  } catch (error) {
    console.error('创建用户失败:', error)
  } finally {
    createUserLoading.value = false
  }
}

const editUser = (user: User) => {
  console.log('Edit user:', user)
}

const confirmDeleteUser = (user: User) => {
  if (confirm(`确定要删除用户 "${user.username}" 吗？`)) {
    deleteUser(user.id).then(() => {
      queryClient.invalidateQueries({ queryKey: ['users'] })
    })
  }
}

// 插件管理相关方法
const handleInstallPlugin = async () => {
  if (!pluginFileInput.value?.files?.[0]) return
  
  installPluginLoading.value = true
  try {
    const formData = new FormData()
    formData.append('plugin', pluginFileInput.value.files[0])
    
    await installPlugin(formData)
    queryClient.invalidateQueries({ queryKey: ['plugins'] })
    showInstallPluginModal.value = false
    if (pluginFileInput.value) {
      pluginFileInput.value.value = ''
    }
  } catch (error) {
    console.error('安装插件失败:', error)
  } finally {
    installPluginLoading.value = false
  }
}

const handleTogglePlugin = async (plugin: Plugin) => {
  try {
    await togglePlugin(plugin.id)
    queryClient.invalidateQueries({ queryKey: ['plugins'] })
  } catch (error) {
    console.error('切换插件状态失败:', error)
  }
}

const configurePlugin = (plugin: Plugin) => {
  console.log('Configure plugin:', plugin)
}

// 工具方法
const formatDate = (dateString: string) => {
  const date = new Date(dateString)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric'
  })
}

// 初始化
onMounted(async () => {
  try {
    const settings = await getSettings()
    systemSettings.value = {
      ...settings,
      maxFileSize: Math.round(settings.maxFileSize / (1024 * 1024)) // 转换为MB
    }
    cacheSettings.value = {
      cachePath: settings.imageCachePath || '',
      maxSize: settings.imageCacheSize / (1024 * 1024 * 1024), // 转换为GB
      quality: settings.imageCacheQuality || 85,
      format: settings.imageCacheFormat || 'WebP'
    }
  } catch (error) {
    console.error('加载设置失败:', error)
  }

  try {
    const aiConfig = await getAISettings()
    aiSettings.value = aiConfig
  } catch (error) {
    console.error('加载AI设置失败:', error)
  }
})
</script>

<style scoped>
.settings-view {
  width: 100%;
  min-height: calc(100vh - 4rem); /* 减去导航栏高度 */
  display: block;
  position: relative;
  overflow-x: auto;
}

/* 确保所有容器都占满宽度 */
.max-w-6xl {
  width: 100%;
  max-width: 72rem;
  margin: 0 auto;
}

/* 标签页内容区域 */
.space-y-6 {
  width: 100%;
}

.space-y-6 > * {
  width: 100%;
  box-sizing: border-box;
}

/* 表单元素确保正确宽度 */
.bg-white {
  width: 100%;
  box-sizing: border-box;
}

/* 网格布局确保正确宽度 */
.grid {
  width: 100%;
}

/* 输入元素确保正确宽度 */
input, select, textarea {
  box-sizing: border-box;
}

/* 标签导航 */
nav {
  width: 100%;
}

/* 强制重置可能的flex影响 */
* {
  flex-shrink: 0;
}
</style>