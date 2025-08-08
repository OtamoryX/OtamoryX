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

        <!-- 缓存策略设置 -->
        <div class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">缓存策略配置</h2>
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                缓存策略
              </label>
              <select
                v-model="cacheSettings.strategy"
                @change="handleCacheStrategyChange"
                class="w-48 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="conservative">保守策略</option>
                <option value="balanced">平衡策略</option>
                <option value="aggressive">激进策略</option>
                <option value="custom">自定义</option>
              </select>
              <p class="mt-1 text-sm text-gray-500">
                {{ getCacheStrategyDescription() }}
              </p>
            </div>

            <!-- 自定义缓存配置 -->
            <div v-if="cacheSettings.strategy === 'custom'" class="pl-4 border-l-4 border-blue-500 space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">
                  最大内存使用 (MB)
                </label>
                <input
                  v-model.number="cacheSettings.customConfig.maxMemoryMb"
                  type="number"
                  min="128"
                  max="4096"
                  class="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">
                  最大缓存档案数
                </label>
                <input
                  v-model.number="cacheSettings.customConfig.maxCachedArchives"
                  type="number"
                  min="5"
                  max="100"
                  class="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">
                  缓存过期时间 (小时)
                </label>
                <input
                  v-model.number="cacheSettings.customConfig.cacheTtlHours"
                  type="number"
                  min="1"
                  max="168"
                  class="w-32 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label class="block text-sm font-medium text-gray-700 mb-2">
                  预加载前后页数
                </label>
                <div class="flex items-center space-x-4">
                  <div>
                    <label class="text-xs text-gray-600">前</label>
                    <input
                      v-model.number="cacheSettings.customConfig.preloadPrevPages"
                      type="number"
                      min="0"
                      max="10"
                      class="w-20 px-2 py-1 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                  <div>
                    <label class="text-xs text-gray-600">后</label>
                    <input
                      v-model.number="cacheSettings.customConfig.preloadNextPages"
                      type="number"
                      min="0"
                      max="10"
                      class="w-20 px-2 py-1 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                </div>
              </div>
            </div>

            <!-- 缓存状态 -->
            <div class="mt-4 p-4 bg-gray-50 rounded-lg">
              <h3 class="text-sm font-medium text-gray-700 mb-2">缓存状态</h3>
              <div v-if="cacheStatus" class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                <div>
                  <span class="text-gray-500">当前策略:</span>
                  <span class="ml-2 font-medium">{{ cacheStatus.current_strategy }}</span>
                </div>
                <div>
                  <span class="text-gray-500">缓存命中率:</span>
                  <span class="ml-2 font-medium">{{ (cacheStatus.stats.hit_rate * 100).toFixed(1) }}%</span>
                </div>
                <div>
                  <span class="text-gray-500">内存使用:</span>
                  <span class="ml-2 font-medium">{{ (cacheStatus.stats.memory_usage_mb).toFixed(1) }} MB</span>
                </div>
                <div>
                  <span class="text-gray-500">缓存数量:</span>
                  <span class="ml-2 font-medium">{{ cacheStatus.stats.cached_archives }}</span>
                </div>
              </div>
              <div class="mt-3 flex justify-end">
                <button
                  @click="clearCache"
                  :disabled="clearingCache"
                  class="px-4 py-2 text-sm text-red-600 border border-red-300 rounded hover:bg-red-50 disabled:opacity-50"
                >
                  {{ clearingCache ? '清理中...' : '清空缓存' }}
                </button>
              </div>
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
          
          <!-- 保存扫描设置按钮 -->
          <div class="flex justify-end pt-4 border-t border-gray-200">
            <button
              @click="saveScanSettings"
              :disabled="systemLoading"
              class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {{ systemLoading ? '保存中...' : '保存扫描设置' }}
            </button>
          </div>
        </div>

        <!-- 扫描操作 -->
        <div class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">漫画库扫描</h2>
          <div class="space-y-4">
            <div>
              <p class="text-sm text-gray-600 mb-4">
                手动触发漫画库扫描，系统会自动检测新添加的漫画文件并添加到数据库中。
              </p>
              <button
                @click="handleManualScan"
                :disabled="scanLoading"
                class="px-6 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50"
              >
                {{ scanLoading ? '扫描中...' : '开始扫描' }}
              </button>
            </div>
            <div v-if="scanResult" class="p-4 rounded-lg" :class="scanResult.success ? 'bg-green-50 text-green-800' : 'bg-red-50 text-red-800'">
              {{ scanResult.message }}
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

      <!-- 批量操作 -->
      <div v-if="activeTab === 'batch'" class="space-y-6">
        <!-- 批量删除操作 -->
        <div class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">批量删除操作</h2>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
            <!-- 批量删除选中漫画 -->
            <div class="border border-gray-200 rounded-lg p-4">
              <h3 class="text-md font-medium text-gray-900 mb-3">选中的漫画</h3>
              <p class="text-sm text-gray-600 mb-4">删除当前选中的漫画文件和数据库记录</p>
              <div class="mb-4">
                <input
                  v-model="batchDeleteForm.archiveIds"
                  type="text"
                  placeholder="输入漫画ID列表，用逗号分隔"
                  class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <p class="mt-1 text-xs text-gray-500">例如: 1,2,3 或留空删除所有</p>
              </div>
              <button
                @click="handleBatchDeleteArchives"
                :disabled="batchOperationLoading"
                class="w-full px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
              >
                {{ batchOperationLoading ? '删除中...' : '批量删除漫画' }}
              </button>
            </div>

            <!-- 按分类批量删除 -->
            <div class="border border-gray-200 rounded-lg p-4">
              <h3 class="text-md font-medium text-gray-900 mb-3">按分类删除</h3>
              <p class="text-sm text-gray-600 mb-4">删除指定分类下的所有漫画</p>
              <div class="mb-4">
                <select
                  v-model="batchDeleteForm.categoryId"
                  class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="">选择分类</option>
                  <option 
                    v-for="category in categories" 
                    :key="category.id" 
                    :value="category.id"
                  >
                    {{ category.name }}
                  </option>
                </select>
              </div>
              <button
                @click="handleBatchDeleteCategoryArchives"
                :disabled="batchOperationLoading || !batchDeleteForm.categoryId"
                class="w-full px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
              >
                {{ batchOperationLoading ? '删除中...' : '删除分类漫画' }}
              </button>
            </div>

            <!-- 按标签批量删除 -->
            <div class="border border-gray-200 rounded-lg p-4">
              <h3 class="text-md font-medium text-gray-900 mb-3">按标签删除</h3>
              <p class="text-sm text-gray-600 mb-4">删除指定标签下的所有漫画</p>
              <div class="mb-4">
                <select
                  v-model="batchDeleteForm.tagId"
                  class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="">选择标签</option>
                  <option 
                    v-for="tag in tags" 
                    :key="tag.id" 
                    :value="tag.id"
                  >
                    {{ tag.namespace }}:{{ tag.name }}
                  </option>
                </select>
              </div>
              <button
                @click="handleBatchDeleteTagArchives"
                :disabled="batchOperationLoading || !batchDeleteForm.tagId"
                class="w-full px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
              >
                {{ batchOperationLoading ? '删除中...' : '删除标签漫画' }}
              </button>
            </div>
          </div>
        </div>

        <!-- 清理操作 -->
        <div class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">数据清理操作</h2>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- 清理无用标签 -->
            <div class="border border-gray-200 rounded-lg p-4">
              <h3 class="text-md font-medium text-gray-900 mb-3">清理无用标签</h3>
              <p class="text-sm text-gray-600 mb-4">删除没有关联任何漫画的标签</p>
              <div class="mb-4">
                <div class="text-sm text-gray-500">
                  将会删除所有未被任何漫画使用的标签，系统标签除外
                </div>
              </div>
              <button
                @click="handlePruneTags"
                :disabled="batchOperationLoading"
                class="w-full px-4 py-2 bg-yellow-600 text-white rounded-lg hover:bg-yellow-700 disabled:opacity-50"
              >
                {{ batchOperationLoading ? '清理中...' : '清理无用标签' }}
              </button>
            </div>

            <!-- 清理空分类 -->
            <div class="border border-gray-200 rounded-lg p-4">
              <h3 class="text-md font-medium text-gray-900 mb-3">清理空分类</h3>
              <p class="text-sm text-gray-600 mb-4">删除没有包含任何漫画的分类</p>
              <div class="mb-4">
                <div class="text-sm text-gray-500">
                  将会删除所有不包含漫画的静态分类和无效的动态分类
                </div>
              </div>
              <button
                @click="handlePruneCategories"
                :disabled="batchOperationLoading"
                class="w-full px-4 py-2 bg-yellow-600 text-white rounded-lg hover:bg-yellow-700 disabled:opacity-50"
              >
                {{ batchOperationLoading ? '清理中...' : '清理空分类' }}
              </button>
            </div>
          </div>
        </div>

        <!-- 操作历史 -->
        <div v-if="batchOperationHistory.length > 0" class="bg-white shadow rounded-lg p-6">
          <h2 class="text-lg font-medium text-gray-900 mb-4">操作历史</h2>
          <div class="space-y-3">
            <div 
              v-for="(record, index) in batchOperationHistory" 
              :key="index"
              class="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
            >
              <div>
                <div class="text-sm font-medium text-gray-900">{{ record.operation }}</div>
                <div class="text-xs text-gray-500">{{ record.timestamp }}</div>
              </div>
              <div class="text-right">
                <div 
                  :class="[
                    'text-sm font-medium',
                    record.success ? 'text-green-600' : 'text-red-600'
                  ]"
                >
                  {{ record.success ? '成功' : '失败' }}
                </div>
                <div class="text-xs text-gray-500">{{ record.result }}</div>
              </div>
            </div>
          </div>
          <div class="mt-4 flex justify-end">
            <button
              @click="batchOperationHistory = []"
              class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800"
            >
              清空历史
            </button>
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

    <!-- 目录浏览器 -->
    <DirectoryBrowser
      :is-open="showDirectoryBrowser"
      :initial-path="directoryBrowserType === 'comics' ? systemSettings.comicsPath : cacheSettings.cachePath"
      @close="closeDirectoryBrowser"
      @select="handleDirectorySelected"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import DirectoryBrowser from '@/components/DirectoryBrowser.vue'
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
  getAIStatus,
  getCategories,
  getTags,
  batchDeleteArchives,
  batchDeleteCategoryArchives,
  batchDeleteTagArchives,
  pruneTags,
  pruneCategories,
  triggerScan,
  getScanSettings,
  updateScanSettings,
  getCacheStatus,
  configureCache,
  clearCache as apiClearCache
} from '@/utils/api'
import type { SystemSettings, User, CreateUserRequest, Plugin, AISettings, AIStatus } from '@/types/api'

const queryClient = useQueryClient()

// 标签页管理
const activeTab = ref('system')
const tabs = [
  { id: 'system', name: '系统配置' },
  { id: 'users', name: '用户管理' },
  { id: 'plugins', name: '插件管理' },
  { id: 'batch', name: '批量操作' },
  { id: 'ai', name: 'AI自动标签' }
]

// 系统设置
const systemSettings = ref<SystemSettings>({
  comicsPath: './comics',
  supportedFormats: ['cbz', 'cbr', 'cb7', 'zip', 'rar'],
  maxFileSize: 100,
  imageCacheSize: 1024,
  scanOnStartup: true
})

const cacheSettings = ref({
  cachePath: './data/cache',
  maxSize: 1.0,
  quality: 85,
  format: 'WebP',
  strategy: 'balanced',
  customConfig: {
    maxMemoryMb: 512,
    maxCachedArchives: 30,
    cacheTtlHours: 24,
    preloadPrevPages: 2,
    preloadNextPages: 3
  }
})

const scanSettings = ref({
  enabled: true,
  recursive: true,
  ignoreHidden: true,
  realtimeMonitoring: false
})

const systemLoading = ref(false)
const scanLoading = ref(false)
const scanResult = ref<{ success: boolean; message: string } | null>(null)

// AI设置
const aiSettings = ref<AISettings>({
  enabled: false,
  autoApplyThreshold: 0.8,
  processingSchedule: 'batch',
  maxConcurrentTasks: 2,
  enabledAnalyzers: []
})

const aiLoading = ref(false)

// 缓存相关
const cacheStatus = ref<any>(null)
const clearingCache = ref(false)

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

// 批量操作
const batchOperationLoading = ref(false)
const batchDeleteForm = ref({
  archiveIds: '',
  categoryId: '',
  tagId: ''
})

interface BatchOperationRecord {
  operation: string
  timestamp: string
  success: boolean
  result: string
}

const batchOperationHistory = ref<BatchOperationRecord[]>([])

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

const { data: categories } = useQuery({
  queryKey: ['categories'],
  queryFn: getCategories,
  enabled: () => activeTab.value === 'batch'
})

const { data: tags } = useQuery({
  queryKey: ['tags'],
  queryFn: getTags,
  enabled: () => activeTab.value === 'batch'
})

// 目录浏览相关
const showDirectoryBrowser = ref(false)
const directoryBrowserType = ref<'comics' | 'cache'>('comics')

// 系统设置相关方法
const selectComicsPath = () => {
  directoryBrowserType.value = 'comics'
  showDirectoryBrowser.value = true
}

const selectCachePath = () => {
  directoryBrowserType.value = 'cache'
  showDirectoryBrowser.value = true
}

const handleDirectorySelected = (path: string) => {
  if (directoryBrowserType.value === 'comics') {
    systemSettings.value.comicsPath = path
  } else if (directoryBrowserType.value === 'cache') {
    cacheSettings.value.cachePath = path
  }
  showDirectoryBrowser.value = false
}

const closeDirectoryBrowser = () => {
  showDirectoryBrowser.value = false
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
    
    // 保存缓存策略配置
    if (cacheSettings.value.strategy) {
      await configureCache({
        strategy: cacheSettings.value.strategy === 'custom' ? undefined : cacheSettings.value.strategy,
        custom_config: cacheSettings.value.strategy === 'custom' ? {
          max_memory_mb: cacheSettings.value.customConfig.maxMemoryMb,
          max_cached_archives: cacheSettings.value.customConfig.maxCachedArchives,
          cache_ttl_hours: cacheSettings.value.customConfig.cacheTtlHours,
          preload_prev_pages: cacheSettings.value.customConfig.preloadPrevPages,
          preload_next_pages: cacheSettings.value.customConfig.preloadNextPages
        } : undefined
      })
    }
    
    alert('系统设置已保存')
  } catch (error) {
    console.error('保存设置失败:', error)
    alert('保存失败')
  } finally {
    systemLoading.value = false
  }
}

// 缓存相关方法
const getCacheStrategyDescription = () => {
  switch (cacheSettings.value.strategy) {
    case 'conservative':
      return '保守策略：低内存使用，较短缓存时间，适合资源有限的系统'
    case 'balanced':
      return '平衡策略：中等配置，适合大多数使用场景'
    case 'aggressive':
      return '激进策略：高内存使用，长缓存时间，适合高性能系统'
    case 'custom':
      return '自定义策略：根据您的需求自由配置缓存参数'
    default:
      return ''
  }
}

const handleCacheStrategyChange = () => {
  // 切换到预设策略时，更新默认值
  switch (cacheSettings.value.strategy) {
    case 'conservative':
      cacheSettings.value.customConfig = {
        maxMemoryMb: 256,
        maxCachedArchives: 10,
        cacheTtlHours: 6,
        preloadPrevPages: 1,
        preloadNextPages: 2
      }
      break
    case 'balanced':
      cacheSettings.value.customConfig = {
        maxMemoryMb: 512,
        maxCachedArchives: 30,
        cacheTtlHours: 24,
        preloadPrevPages: 2,
        preloadNextPages: 3
      }
      break
    case 'aggressive':
      cacheSettings.value.customConfig = {
        maxMemoryMb: 1024,
        maxCachedArchives: 50,
        cacheTtlHours: 168,
        preloadPrevPages: 3,
        preloadNextPages: 5
      }
      break
  }
}

const clearCache = async () => {
  if (!confirm('确定要清空所有缓存吗？')) {
    return
  }
  
  clearingCache.value = true
  try {
    await apiClearCache()
    await loadCacheStatus()
    alert('缓存已清空')
  } catch (error) {
    console.error('清空缓存失败:', error)
    alert('清空缓存失败')
  } finally {
    clearingCache.value = false
  }
}

const loadCacheStatus = async () => {
  try {
    cacheStatus.value = await getCacheStatus()
  } catch (error) {
    console.error('加载缓存状态失败:', error)
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

// 批量操作相关方法
const addOperationRecord = (operation: string, success: boolean, result: string) => {
  batchOperationHistory.value.unshift({
    operation,
    timestamp: new Date().toLocaleString('zh-CN'),
    success,
    result
  })
}

const handleBatchDeleteArchives = async () => {
  if (!confirm('确定要执行批量删除漫画操作吗？此操作不可撤销！')) {
    return
  }

  batchOperationLoading.value = true
  try {
    const archiveIds = batchDeleteForm.value.archiveIds
      ? batchDeleteForm.value.archiveIds.split(',').map(id => id.trim())
      : []
    
    await batchDeleteArchives(archiveIds)
    addOperationRecord('批量删除漫画', true, `删除了 ${archiveIds.length || '所有'} 个漫画`)
    batchDeleteForm.value.archiveIds = ''
    
    // 刷新相关数据
    queryClient.invalidateQueries({ queryKey: ['archives'] })
  } catch (error) {
    console.error('批量删除漫画失败:', error)
    addOperationRecord('批量删除漫画', false, (error as Error).message)
  } finally {
    batchOperationLoading.value = false
  }
}

const handleBatchDeleteCategoryArchives = async () => {
  if (!confirm('确定要删除该分类下的所有漫画吗？此操作不可撤销！')) {
    return
  }

  batchOperationLoading.value = true
  try {
    const categoryId = batchDeleteForm.value.categoryId
    await batchDeleteCategoryArchives(categoryId)
    
    const categoryName = categories.value?.find(c => c.id === categoryId)?.name || categoryId
    addOperationRecord('按分类删除漫画', true, `删除了分类 "${categoryName}" 下的所有漫画`)
    batchDeleteForm.value.categoryId = ''
    
    // 刷新相关数据
    queryClient.invalidateQueries({ queryKey: ['archives'] })
    queryClient.invalidateQueries({ queryKey: ['categories'] })
  } catch (error) {
    console.error('按分类删除漫画失败:', error)
    addOperationRecord('按分类删除漫画', false, (error as Error).message)
  } finally {
    batchOperationLoading.value = false
  }
}

const handleBatchDeleteTagArchives = async () => {
  if (!confirm('确定要删除该标签下的所有漫画吗？此操作不可撤销！')) {
    return
  }

  batchOperationLoading.value = true
  try {
    const tagId = batchDeleteForm.value.tagId
    await batchDeleteTagArchives(tagId)
    
    const tag = tags.value?.find(t => t.id === tagId)
    const tagName = tag ? `${tag.namespace}:${tag.name}` : tagId
    addOperationRecord('按标签删除漫画', true, `删除了标签 "${tagName}" 下的所有漫画`)
    batchDeleteForm.value.tagId = ''
    
    // 刷新相关数据
    queryClient.invalidateQueries({ queryKey: ['archives'] })
    queryClient.invalidateQueries({ queryKey: ['tags'] })
  } catch (error) {
    console.error('按标签删除漫画失败:', error)
    addOperationRecord('按标签删除漫画', false, (error as Error).message)
  } finally {
    batchOperationLoading.value = false
  }
}

const handlePruneTags = async () => {
  if (!confirm('确定要清理所有无用的标签吗？')) {
    return
  }

  batchOperationLoading.value = true
  try {
    await pruneTags()
    addOperationRecord('清理无用标签', true, '成功清理了所有未使用的标签')
    
    // 刷新标签数据
    queryClient.invalidateQueries({ queryKey: ['tags'] })
  } catch (error) {
    console.error('清理标签失败:', error)
    addOperationRecord('清理无用标签', false, (error as Error).message)
  } finally {
    batchOperationLoading.value = false
  }
}

const handlePruneCategories = async () => {
  if (!confirm('确定要清理所有空分类吗？')) {
    return
  }

  batchOperationLoading.value = true
  try {
    await pruneCategories()
    addOperationRecord('清理空分类', true, '成功清理了所有空分类')
    
    // 刷新分类数据
    queryClient.invalidateQueries({ queryKey: ['categories'] })
  } catch (error) {
    console.error('清理分类失败:', error)
    addOperationRecord('清理空分类', false, (error as Error).message)
  } finally {
    batchOperationLoading.value = false
  }
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
      format: settings.imageCacheFormat || 'WebP',
      strategy: 'balanced', // 默认策略
      customConfig: {
        maxMemoryMb: 512,
        maxCachedArchives: 30,
        cacheTtlHours: 24,
        preloadPrevPages: 2,
        preloadNextPages: 3
      }
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

  // 加载扫描设置
  try {
    const scanConfig = await getScanSettings()
    if (scanConfig && scanConfig.scanSettings) {
      scanSettings.value = scanConfig.scanSettings
    }
  } catch (error) {
    console.error('加载扫描设置失败:', error)
  }

  // 加载缓存状态
  await loadCacheStatus()
})

// 手动扫描相关方法
const handleManualScan = async () => {
  scanLoading.value = true
  scanResult.value = null
  
  try {
    const result = await triggerScan()
    scanResult.value = {
      success: true,
      message: result.message
    }
    // 刷新漫画列表数据
    queryClient.invalidateQueries({ queryKey: ['archives'] })
  } catch (error) {
    console.error('手动扫描失败:', error)
    scanResult.value = {
      success: false,
      message: '扫描失败，请检查漫画库路径是否正确'
    }
  } finally {
    scanLoading.value = false
  }
}

// 保存扫描设置
const saveScanSettings = async () => {
  systemLoading.value = true
  
  try {
    const result = await updateScanSettings(scanSettings.value)
    console.log('扫描设置保存成功:', result)
    
    // 显示成功消息
    scanResult.value = {
      success: true,
      message: `扫描设置已更新，实时监控状态: ${result.monitoring_status ? '已启用' : '已禁用'}`
    }
  } catch (error) {
    console.error('保存扫描设置失败:', error)
    scanResult.value = {
      success: false,
      message: '保存扫描设置失败，请稍后重试'
    }
  } finally {
    systemLoading.value = false
  }
}
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