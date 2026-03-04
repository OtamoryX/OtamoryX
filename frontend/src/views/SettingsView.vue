<template>
  <BasePageView theme="settings">
    <!-- Settings特有的单栏布局 -->
    <div class="settings-view w-full h-full p-6">
      <GlassCard size="sm" radius="lg" class="mb-6">
        <div class="flex items-center">
          <svg class="w-8 h-8 mr-3 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          <h1 class="text-2xl font-bold text-[var(--text-primary)]">{{ pageTitle }}</h1>
        </div>
      </GlassCard>

      <div class="w-full max-w-6xl mx-auto">
        <!-- 标签导航 -->
        <GlassCard v-if="tabs.length > 1" size="sm" radius="lg" class="mb-6">
          <nav class="flex space-x-1">
            <GlassButton v-for="tab in tabs" :key="tab.id" :variant="activeTab === tab.id ? 'primary' : 'ghost'"
              size="sm" class="py-2! px-4!" @click="setActiveTab(tab.id)">
              {{ tab.name }}
            </GlassButton>
          </nav>
        </GlassCard>

        <!-- 外观设置 -->
        <div v-if="isUserSettingsRoute && activeTab === 'appearance'" class="space-y-6 pb-20">
          <GlassCard size="md" radius="lg">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4 flex items-center">
              <svg class="w-6 h-6 mr-2 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" />
              </svg>
              外观设置
            </h2>
            <div class="space-y-6">
              <!-- 主题选择 -->
              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-3">
                  主题模式
                </label>
                <div class="grid grid-cols-3 gap-3">
                  <button
                    v-for="themeOption in themeOptions"
                    :key="themeOption.value"
                    :class="[
                      'flex flex-col items-center justify-center p-4 rounded-lg border-2 transition-all',
                      theme === themeOption.value
                        ? 'border-[var(--accent)] bg-[var(--accent)]/20'
                        : 'border-[var(--border)] bg-[var(--bg-tertiary)] hover:bg-[var(--bg-tertiary)]'
                    ]"
                    @click="setTheme(themeOption.value as 'light' | 'dark' | 'system')"
                  >
                    <component :is="themeOption.icon" class="w-8 h-8 mb-2 text-[var(--text-primary)]" />
                    <span class="text-sm text-[var(--text-primary)] font-medium">{{ themeOption.label }}</span>
                  </button>
                </div>
                <p class="mt-2 text-sm text-[var(--text-secondary)]">
                  选择应用的主题模式，系统模式将跟随操作系统设置
                </p>
              </div>

              <!-- 随机精选开关 -->
              <div class="flex items-center justify-between">
                <div>
                  <label class="block text-sm font-medium text-[var(--text-primary)] mb-1">
                    显示随机精选
                  </label>
                  <p class="text-sm text-[var(--text-secondary)]">
                    在库页面顶部显示随机精选横向滚动区域
                  </p>
                </div>
                <button
                  :class="[
                    'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
                    libraryStore.showCarousel ? 'bg-[var(--accent)]' : 'bg-[var(--border)]'
                  ]"
                  @click="libraryStore.setShowCarousel(!libraryStore.showCarousel)"
                >
                  <span
                    :class="[
                      'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                      libraryStore.showCarousel ? 'translate-x-6' : 'translate-x-1'
                    ]"
                  />
                </button>
              </div>

              <!-- 每页行数设置 -->
              <div class="flex items-center justify-between">
                <div>
                  <label class="block text-sm font-medium text-[var(--text-primary)] mb-1">
                    每页显示行数
                  </label>
                  <p class="text-sm text-[var(--text-secondary)]">
                    书库列表每页显示的行数，每页数量会根据屏幕宽度自动适配列数
                  </p>
                </div>
                <select
                  :value="libraryStore.rowsPerPage"
                  class="w-20 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                  @change="libraryStore.setRowsPerPage(Number(($event.target as HTMLSelectElement).value))"
                >
                  <option v-for="n in 8" :key="n + 2" :value="n + 2">{{ n + 2 }}</option>
                </select>
              </div>
            </div>
          </GlassCard>
        </div>

        <!-- 系统配置 -->
        <div v-if="isAdminSettingsRoute && activeTab === 'system'" class="space-y-6 pb-20">
          <!-- 漫画库路径设置 -->
          <GlassCard size="md" radius="lg" class="mb-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4 flex items-center">
              <svg class="w-6 h-6 mr-2 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 011-1h6a1 1 0 011 1v2M7 7v2m10-2v2" />
              </svg>
              漫画库配置
            </h2>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  漫画库路径
                </label>
                <div class="flex space-x-2">
                  <GlassInput v-model="systemSettings.comicsPath" placeholder="/path/to/comics" class="flex-1" />
                  <GlassButton variant="secondary" @click="selectComicsPath">
                    浏览
                  </GlassButton>
                </div>
                <p class="mt-1 text-sm text-[var(--text-secondary)]">
                  指定存储漫画文件的目录路径
                </p>
              </div>

              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  支持的文件格式
                </label>
                <div class="flex flex-wrap gap-2">
                  <span v-for="format in systemSettings.supportedFormats" :key="format"
                    class="px-2 py-1 bg-[var(--accent)]/20 text-[var(--accent)] text-xs rounded-full border border-[var(--accent)]/30">
                    .{{ format }}
                  </span>
                </div>
              </div>

              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  最大文件大小 (MB)
                </label>
                <input v-model.number="systemSettings.maxFileSize" type="number" min="1" max="1000"
                  class="w-32 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]" />
                <p class="mt-1 text-sm text-[var(--text-secondary)]">
                  单个漫画文件的最大大小限制
                </p>
              </div>
            </div>
          </GlassCard>

          <!-- 缓存策略设置 -->
          <GlassCard size="md" radius="lg" class="mb-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4 flex items-center">
              <svg class="w-6 h-6 mr-2 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M4 7v10c0 2.21 3.57 4 8 4s8-1.79 8-4V7c0 2.21-3.57 4-8 4s-8-1.79-8-4z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M4 7c0 2.21 3.57 4 8 4s8-1.79 8-4-3.57-4-8-4-8 1.79-8 4z" />
              </svg>
              缓存策略配置
            </h2>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  缓存策略
                </label>
                <select v-model="cacheSettings.strategy"
                  class="w-48 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]"
                  @change="handleCacheStrategyChange">
                  <option value="conservative" class="text-[var(--text-primary)]">
                    保守策略
                  </option>
                  <option value="balanced" class="text-[var(--text-primary)]">
                    平衡策略
                  </option>
                  <option value="aggressive" class="text-[var(--text-primary)]">
                    激进策略
                  </option>
                  <option value="custom" class="text-[var(--text-primary)]">自定义</option>
                </select>
                <p class="mt-1 text-sm text-[var(--text-secondary)]">
                  {{ getCacheStrategyDescription() }}
                </p>
              </div>

              <!-- 自定义缓存配置 -->
              <div v-if="cacheSettings.strategy === 'custom'"
                class="pl-4 border-l-4 border-[var(--accent)] bg-[var(--bg-tertiary)] rounded-lg p-4 space-y-4">
                <div>
                  <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                    最大内存使用 (MB)
                  </label>
                  <input v-model.number="cacheSettings.customConfig.maxMemoryMb" type="number" min="128" max="4096"
                    class="w-32 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                </div>

                <div>
                  <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                    最大缓存档案数
                  </label>
                  <input v-model.number="cacheSettings.customConfig.maxCachedArchives
                    " type="number" min="5" max="100"
                    class="w-32 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                </div>

                <div>
                  <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                    缓存过期时间 (小时)
                  </label>
                  <input v-model.number="cacheSettings.customConfig.cacheTtlHours" type="number" min="1" max="168"
                    class="w-32 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                </div>

                <div>
                  <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                    预加载前后页数
                  </label>
                  <div class="flex items-center space-x-4">
                    <div>
                      <label class="text-xs text-[var(--text-secondary)]">前</label>
                      <input v-model.number="cacheSettings.customConfig.preloadPrevPages
                        " type="number" min="0" max="10"
                        class="w-20 px-2 py-1 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-md text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                    </div>
                    <div>
                      <label class="text-xs text-[var(--text-secondary)]">后</label>
                      <input v-model.number="cacheSettings.customConfig.preloadNextPages
                        " type="number" min="0" max="10"
                        class="w-20 px-2 py-1 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-md text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                    </div>
                  </div>
                </div>
              </div>

              <!-- 缓存状态 -->
              <div class="mt-4 p-4 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg">
                <h3 class="text-sm font-medium text-[var(--text-primary)] mb-2">缓存状态</h3>
                <div v-if="cacheStatus" class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <span class="text-[var(--text-secondary)]">当前策略:</span>
                    <span class="ml-2 font-medium text-[var(--text-primary)]">{{
                      cacheStatus.current_strategy
                    }}</span>
                  </div>
                  <div>
                    <span class="text-[var(--text-secondary)]">缓存命中率:</span>
                    <span class="ml-2 font-medium text-[var(--text-primary)]">{{
                      formatHitRate(cacheStatus.stats.hit_rate)
                    }}</span>
                  </div>
                  <div>
                    <span class="text-[var(--text-secondary)]">内存使用:</span>
                    <span class="ml-2 font-medium text-[var(--text-primary)]">{{
                      cacheStatus.stats.memory_usage_mb.toFixed(1)
                      }}
                      MB</span>
                  </div>
                  <div>
                    <span class="text-[var(--text-secondary)]">缓存数量:</span>
                    <span class="ml-2 font-medium text-[var(--text-primary)]">{{
                      cacheStatus.stats.cached_archives
                    }}</span>
                  </div>
                </div>
                <div class="mt-3 space-y-2">
                  <div class="flex flex-wrap gap-2">
                    <GlassButton :disabled="isClearingCache" variant="secondary" size="sm" @click="loadCacheStatus">
                      刷新状态
                    </GlassButton>
                    <GlassButton :disabled="isClearingCache" variant="secondary" size="sm" @click="clearCache('pages')">
                      {{ clearingCacheScope === "pages" ? "清理中..." : "清理阅读缓存(推荐)" }}
                    </GlassButton>
                    <GlassButton :disabled="isClearingCache" variant="secondary" size="sm" @click="clearCache('covers')">
                      {{ clearingCacheScope === "covers" ? "清理中..." : "清理封面缓存" }}
                    </GlassButton>
                    <GlassButton :disabled="isClearingCache" variant="danger" size="sm" @click="clearCache('all')">
                      {{ clearingCacheScope === "all" ? "清理中..." : "清空全部缓存" }}
                    </GlassButton>
                  </div>
                  <p class="text-xs text-[var(--text-secondary)]">
                    封面缓存会影响列表封面显示，建议优先清理“阅读缓存”。
                  </p>
                </div>
              </div>
            </div>
          </GlassCard>

          <!-- 图像缓存设置 -->
          <GlassCard size="md" radius="lg" class="mb-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4 flex items-center">
              <svg class="w-6 h-6 mr-2 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
              </svg>
              图像缓存配置
            </h2>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  缓存路径
                </label>
                <div class="flex space-x-2">
                  <GlassInput v-model="cacheSettings.cachePath" placeholder="/path/to/cache" class="flex-1" />
                  <GlassButton variant="secondary" @click="selectCachePath">
                    浏览
                  </GlassButton>
                </div>
                <p class="mt-1 text-sm text-[var(--text-secondary)]">
                  指定图像缓存文件的存储目录路径
                </p>
              </div>

              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  缓存大小 (GB)
                </label>
                <input v-model.number="cacheSettings.maxSize" type="number" min="0.1" max="10" step="0.1"
                  class="w-32 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                <p class="mt-1 text-sm text-[var(--text-secondary)]">图像缓存的最大存储空间</p>
              </div>

              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  封面生成质量
                </label>
                <div class="flex items-center space-x-4">
                  <input v-model.number="cacheSettings.quality" type="range" min="1" max="100" class="flex-1">
                  <span class="text-sm text-[var(--text-primary)] w-12">{{ cacheSettings.quality }}%</span>
                </div>
                <p class="mt-1 text-sm text-[var(--text-secondary)]">控制漫画封面 JPEG 生成时的压缩质量（1-100）</p>
              </div>

              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  输出格式
                </label>
                <select v-model="cacheSettings.format"
                  class="w-32 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                  <option value="JPEG" class="text-[var(--text-primary)]">JPEG</option>
                  <option value="PNG" class="text-[var(--text-primary)]">PNG</option>
                  <option value="WebP" class="text-[var(--text-primary)]">WebP</option>
                </select>
              </div>
            </div>
          </GlassCard>

          <!-- 自动扫描设置 -->
          <GlassCard size="md" radius="lg" class="mb-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4 flex items-center">
              <svg class="w-6 h-6 mr-2 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              自动扫描配置
            </h2>
            <div class="space-y-4">
              <div class="flex items-center">
                <input id="auto-scan" v-model="scanSettings.enabled" type="checkbox"
                  class="h-4 w-4 text-[var(--accent)] bg-[var(--bg-tertiary)] border-[var(--border)] focus:ring-[var(--accent)] focus:ring-offset-0 rounded">
                <label for="auto-scan" class="ml-2 block text-sm text-[var(--text-primary)]">
                  启用自动扫描
                </label>
              </div>

              <div class="flex items-center">
                <input id="recursive-scan" v-model="scanSettings.recursive" type="checkbox"
                  class="h-4 w-4 text-[var(--accent)] bg-[var(--bg-tertiary)] border-[var(--border)] focus:ring-[var(--accent)] focus:ring-offset-0 rounded" />
                <label for="recursive-scan" class="ml-2 block text-sm text-[var(--text-primary)]">
                  递归扫描子目录
                </label>
              </div>

              <div class="flex items-center">
                <input id="ignore-hidden" v-model="scanSettings.ignoreHidden" type="checkbox"
                  class="h-4 w-4 text-[var(--accent)] bg-[var(--bg-tertiary)] border-[var(--border)] focus:ring-[var(--accent)] focus:ring-offset-0 rounded">
                <label for="ignore-hidden" class="ml-2 block text-sm text-[var(--text-primary)]">
                  忽略隐藏文件
                </label>
              </div>

              <div class="flex items-center">
                <input id="realtime-monitoring" v-model="scanSettings.realtimeMonitoring" type="checkbox"
                  class="h-4 w-4 text-[var(--accent)] bg-[var(--bg-tertiary)] border-[var(--border)] focus:ring-[var(--accent)] focus:ring-offset-0 rounded">
                <label for="realtime-monitoring" class="ml-2 block text-sm text-[var(--text-primary)]">
                  实时文件监控
                </label>
              </div>
            </div>
            <!-- 保存扫描设置按钮 -->
            <div class="flex justify-end pt-4 border-t border-[var(--border)]">
              <GlassButton :disabled="systemLoading" variant="primary" @click="saveScanSettings">
                {{ systemLoading ? "保存中..." : "保存扫描设置" }}
              </GlassButton>
            </div>
          </GlassCard>

          <!-- 扫描操作 -->
          <GlassCard size="md" radius="lg" class="mb-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4 flex items-center">
              <svg class="w-6 h-6 mr-2 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              漫画库扫描
            </h2>
            <div class="space-y-4">
              <div>
                <p class="text-sm text-[var(--text-secondary)] mb-4">
                  手动触发漫画库扫描，系统会自动检测新添加的漫画文件并添加到数据库中。
                </p>
                <GlassButton :disabled="scanLoading" variant="success" @click="handleManualScan">
                  {{ scanLoading ? "扫描中..." : "开始扫描" }}
                </GlassButton>
              </div>
              <div v-if="scanResult" class="p-4 rounded-lg backdrop-blur-md border" :class="scanResult.success
                ? 'bg-green-500/20 text-green-400 border-green-400/30'
                : 'bg-red-500/20 text-red-400 border-red-400/30'
                ">
                {{ scanResult.message }}
              </div>
            </div>
          </GlassCard>

          <div class="flex justify-end">
            <GlassButton :disabled="systemLoading" variant="primary" class="px-8 py-3" @click="saveSystemSettings">
              {{ systemLoading ? "保存中..." : "保存设置" }}
            </GlassButton>
          </div>
        </div>

        <!-- 用户管理 -->
        <div v-if="isAdminSettingsRoute && activeTab === 'users'" class="space-y-6 pb-20">
          <div class="flex justify-between items-center">
            <h2 class="text-lg font-medium text-[var(--text-primary)]">用户管理</h2>
            <button class="px-4 py-2 bg-[var(--accent)] text-white rounded-lg hover:bg-[var(--accent-hover)]"
              @click="showCreateUserModal = true">
              创建用户
            </button>
          </div>

          <div v-if="usersLoading" class="text-center py-8">
            <div class="text-[var(--text-secondary)]">加载中...</div>
          </div>

          <div v-else class="bg-[var(--bg-card)] shadow-sm overflow-hidden sm:rounded-md">
            <ul class="divide-y divide-gray-200">
              <li v-for="user in users" :key="user.id" class="px-6 py-4">
                <div class="flex items-center justify-between">
                  <div class="flex items-center space-x-4">
                    <div class="shrink-0">
                      <div class="h-10 w-10 rounded-full bg-[var(--bg-tertiary)] flex items-center justify-center">
                        <span class="text-sm font-medium text-[var(--text-primary)]">
                          {{ user.username.charAt(0).toUpperCase() }}
                        </span>
                      </div>
                    </div>
                    <div>
                      <p class="text-sm font-medium text-[var(--text-primary)]">
                        {{ user.username }}
                      </p>
                      <p class="text-sm text-[var(--text-secondary)]">
                        {{ user.email || "未设置邮箱" }}
                      </p>
                      <p class="text-xs text-[var(--text-tertiary)]">
                        创建于 {{ formatDate(user.createdAt) }}
                      </p>
                    </div>
                  </div>
                  <div class="flex items-center space-x-2">
                    <button
                      class="px-3 py-1 text-sm text-[var(--accent)] hover:text-[var(--accent-hover)] border border-[var(--accent)] rounded hover:bg-[var(--bg-secondary)]"
                      @click="editUser(user)">
                      编辑
                    </button>
                    <button
                      class="px-3 py-1 text-sm text-red-600 hover:text-red-800 border border-red-300 rounded hover:bg-red-50"
                      @click="confirmDeleteUser(user)">
                      删除
                    </button>
                  </div>
                </div>
              </li>
            </ul>
          </div>
        </div>

        <!-- 插件管理 -->
        <div v-if="isAdminSettingsRoute && activeTab === 'plugins'" class="space-y-6 pb-20">
          <div class="flex justify-between items-center">
            <h2 class="text-lg font-medium text-[var(--text-primary)]">插件管理</h2>
            <button class="px-4 py-2 bg-[var(--accent)] text-white rounded-lg hover:bg-[var(--accent-hover)]"
              @click="showInstallPluginModal = true">
              安装插件
            </button>
          </div>

          <div v-if="pluginsLoading" class="text-center py-8">
            <div class="text-[var(--text-secondary)]">加载中...</div>
          </div>

          <div v-else-if="plugins?.length === 0" class="text-center py-8">
            <div class="text-[var(--text-secondary)]">暂无已安装的插件</div>
          </div>

          <div v-else class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            <GlassCard v-for="plugin in plugins" :key="plugin.id" class="bg-[var(--bg-card)] rounded-lg shadow border p-6">
              <div class="flex items-start justify-between mb-4">
                <div class="flex-1">
                  <h3 class="text-lg font-semibold text-[var(--text-primary)]">
                    {{ plugin.name }}
                  </h3>
                  <p class="text-sm text-[var(--text-secondary)]">v{{ plugin.version }}</p>
                </div>
                <span :class="[
                  'px-2 py-1 text-xs rounded-full',
                  plugin.enabled
                    ? 'bg-green-100 text-green-800'
                    : 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]',
                ]">
                  {{ plugin.enabled ? "已启用" : "已禁用" }}
                </span>
              </div>

              <p v-if="plugin.description" class="text-sm text-[var(--text-secondary)] mb-4">
                {{ plugin.description }}
              </p>

              <div class="text-xs text-[var(--text-tertiary)] mb-4">
                安装于 {{ formatDate(plugin.installedAt) }}
              </div>

              <div class="flex justify-between items-center">
                <GlassButton :variant="plugin.enabled ? 'danger' : 'success'" size="sm"
                  @click="handleTogglePlugin(plugin)">
                  {{ plugin.enabled ? "禁用" : "启用" }}
                </GlassButton>

                <div class="flex items-center space-x-2">
                  <GlassButton v-if="plugin.config" variant="secondary" size="sm" @click="configurePlugin(plugin)">
                    配置
                  </GlassButton>
                </div>
              </div>
            </GlassCard>
          </div>
        </div>

        <!-- 批量操作 -->
        <div v-if="isAdminSettingsRoute && activeTab === 'batch'" class="space-y-6 pb-20">
          <!-- 批量删除操作 -->
          <div class="bg-[var(--bg-card)] shadow-sm rounded-lg p-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4">批量删除操作</h2>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
              <!-- 批量删除选中漫画 -->
              <div class="border border-[var(--border)] rounded-lg p-4">
                <h3 class="text-md font-medium text-[var(--text-primary)] mb-3">
                  选中的漫画
                </h3>
                <p class="text-sm text-[var(--text-secondary)] mb-4">
                  删除当前选中的漫画文件和数据库记录
                </p>
                <div class="mb-4">
                  <input v-model="batchDeleteForm.archiveIds" type="text" placeholder="输入漫画ID列表，用逗号分隔"
                    class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                  <p class="mt-1 text-xs text-[var(--text-secondary)]">
                    例如: 1,2,3 或留空删除所有
                  </p>
                </div>
                <GlassButton :disabled="batchOperationLoading" variant="danger" class="w-full"
                  @click="handleBatchDeleteArchives">
                  {{ batchOperationLoading ? "删除中..." : "批量删除漫画" }}
                </GlassButton>
              </div>

              <!-- 按分类批量删除 -->
              <div class="border border-[var(--border)] rounded-lg p-4">
                <h3 class="text-md font-medium text-[var(--text-primary)] mb-3">
                  按分类删除
                </h3>
                <p class="text-sm text-[var(--text-secondary)] mb-4">
                  删除指定分类下的所有漫画
                </p>
                <div class="mb-4">
                  <select v-model="batchDeleteForm.categoryId"
                    class="w-full px-3 py-2 border border-[var(--border)] rounded-md focus:outline-none focus:ring-2 focus:ring-[var(--accent)]">
                    <option value="">选择分类</option>
                    <option v-for="category in categories" :key="category.id" :value="category.id">
                      {{ category.name }}
                    </option>
                  </select>
                </div>
                <button :disabled="batchOperationLoading || !batchDeleteForm.categoryId
                  " class="w-full px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
                  @click="handleBatchDeleteCategoryArchives">
                  {{ batchOperationLoading ? "删除中..." : "删除分类漫画" }}
                </button>
              </div>

              <!-- 按标签批量删除 -->
              <div class="border border-[var(--border)] bg-[var(--bg-tertiary)] rounded-lg p-4">
                <h3 class="text-md font-medium text-[var(--text-primary)] mb-3">按标签删除</h3>
                <p class="text-sm text-[var(--text-secondary)] mb-4">
                  删除指定标签下的所有漫画
                </p>
                <div class="mb-4">
                  <select v-model="batchDeleteForm.tagId"
                    class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                    <option value="" class="text-[var(--text-primary)]">选择标签</option>
                    <option v-for="tag in tags" :key="tag.id" :value="tag.id" class="text-[var(--text-primary)]">
                      {{ tag.namespace }}:{{ tag.name }}
                    </option>
                  </select>
                </div>
                <GlassButton :disabled="batchOperationLoading || !batchDeleteForm.tagId" variant="danger" class="w-full"
                  @click="handleBatchDeleteTagArchives">
                  {{ batchOperationLoading ? "删除中..." : "删除标签漫画" }}
                </GlassButton>
              </div>
            </div>
          </div>

          <!-- 清理操作 -->
          <GlassCard class="bg-[var(--bg-card)] shadow-sm rounded-lg p-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4">数据清理操作</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
              <!-- 清理无用标签 -->
              <div class="border border-[var(--border)] rounded-lg p-4">
                <h3 class="text-md font-medium text-[var(--text-primary)] mb-3">
                  清理无用标签
                </h3>
                <p class="text-sm text-[var(--text-secondary)] mb-4">
                  删除没有关联任何漫画的标签
                </p>
                <div class="mb-4">
                  <div class="text-sm text-[var(--text-secondary)]">
                    将会删除所有未被任何漫画使用的标签，系统标签除外
                  </div>
                </div>
                <button :disabled="batchOperationLoading"
                  class="w-full px-4 py-2 bg-yellow-600 text-white rounded-lg hover:bg-yellow-700 disabled:opacity-50"
                  @click="handlePruneTags">
                  {{ batchOperationLoading ? "清理中..." : "清理无用标签" }}
                </button>
              </div>

              <!-- 清理空分类 -->
              <div class="border border-[var(--border)] rounded-lg p-4">
                <h3 class="text-md font-medium text-[var(--text-primary)] mb-3">
                  清理空分类
                </h3>
                <p class="text-sm text-[var(--text-secondary)] mb-4">
                  删除没有包含任何漫画的分类
                </p>
                <div class="mb-4">
                  <div class="text-sm text-[var(--text-secondary)]">
                    将会删除所有不包含漫画的静态分类和无效的动态分类
                  </div>
                </div>
                <button :disabled="batchOperationLoading"
                  class="w-full px-4 py-2 bg-yellow-600 text-white rounded-lg hover:bg-yellow-700 disabled:opacity-50"
                  @click="handlePruneCategories">
                  {{ batchOperationLoading ? "清理中..." : "清理空分类" }}
                </button>
              </div>
            </div>
          </GlassCard>

          <!-- 操作历史 -->
          <GlassCard v-if="batchOperationHistory.length > 0" class="bg-[var(--bg-card)] shadow-sm rounded-lg p-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4">操作历史</h2>
            <div class="space-y-3">
              <div v-for="(record, index) in batchOperationHistory" :key="index"
                class="flex items-center justify-between p-3 bg-[var(--bg-secondary)] rounded-lg">
                <div>
                  <div class="text-sm font-medium text-[var(--text-primary)]">
                    {{ record.operation }}
                  </div>
                  <div class="text-xs text-[var(--text-secondary)]">
                    {{ record.timestamp }}
                  </div>
                </div>
                <div class="text-right">
                  <div :class="[
                    'text-sm font-medium',
                    record.success ? 'text-green-600' : 'text-red-600',
                  ]">
                    {{ record.success ? "成功" : "失败" }}
                  </div>
                  <div class="text-xs text-[var(--text-secondary)]">
                    {{ record.result }}
                  </div>
                </div>
              </div>
            </div>
            <div class="mt-4 flex justify-end">
              <button class="px-4 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)]" @click="batchOperationHistory = []">
                清空历史
              </button>
            </div>
          </GlassCard>
        </div>

        <!-- AI自动标签 -->
        <div v-if="isAdminSettingsRoute && activeTab === 'ai'" class="space-y-6 pb-20">
          <GlassCard size="md" radius="lg" class="mb-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4 flex items-center">
              <svg class="w-6 h-6 mr-2 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
              </svg>
              AI自动标签配置
            </h2>
            <div class="space-y-4">
              <div class="flex items-center">
                <input id="ai-enabled" v-model="aiSettings.enabled" type="checkbox"
                  class="h-4 w-4 text-[var(--accent)] bg-[var(--bg-tertiary)] border-[var(--border)] focus:ring-[var(--accent)] focus:ring-offset-0 rounded">
                <label for="ai-enabled" class="ml-2 block text-sm text-[var(--text-primary)]">
                  启用AI自动标签
                </label>
              </div>

              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  自动应用阈值
                </label>
                <div class="flex items-center space-x-4">
                  <input v-model.number="aiSettings.autoApplyThreshold" type="range" min="0.1" max="1.0" step="0.1"
                    class="flex-1">
                  <span class="text-sm text-[var(--text-primary)] w-12">{{
                    (aiSettings.autoApplyThreshold * 100).toFixed(0)
                    }}%</span>
                </div>
                <p class="mt-1 text-sm text-[var(--text-secondary)]">
                  置信度达到此阈值的AI标签将自动应用
                </p>
              </div>

              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  处理调度
                </label>
                <select v-model="aiSettings.processingSchedule"
                  class="w-48 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]">
                  <option value="immediate" class="text-[var(--text-primary)]">
                    立即处理
                  </option>
                  <option value="batch" class="text-[var(--text-primary)]">批量处理</option>
                  <option value="off-peak" class="text-[var(--text-primary)]">
                    非高峰时段
                  </option>
                </select>
              </div>

              <div>
                <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                  最大并发任务数
                </label>
                <input v-model.number="aiSettings.maxConcurrentTasks" type="number" min="1" max="10"
                  class="w-32 px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)]" />
              </div>
            </div>
          </GlassCard>

          <!-- AI状态监控 -->
          <GlassCard v-if="aiStatus" size="md" radius="lg" class="mb-6">
            <h2 class="text-lg font-medium text-[var(--text-primary)] mb-4 flex items-center">
              <svg class="w-6 h-6 mr-2 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
              AI处理状态
            </h2>
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div class="text-center p-4 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg backdrop-blur-md">
                <div class="text-2xl font-bold text-[var(--accent)]">
                  {{ aiStatus.queueSize }}
                </div>
                <div class="text-sm text-[var(--text-secondary)]">队列中</div>
              </div>
              <div class="text-center p-4 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg backdrop-blur-md">
                <div class="text-2xl font-bold text-green-400">
                  {{ aiStatus.processingCount }}
                </div>
                <div class="text-sm text-[var(--text-secondary)]">处理中</div>
              </div>
              <div class="text-center p-4 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg backdrop-blur-md">
                <div class="text-2xl font-bold text-purple-400">
                  {{ aiStatus.completedToday }}
                </div>
                <div class="text-sm text-[var(--text-secondary)]">今日完成</div>
              </div>
              <div class="text-center p-4 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg backdrop-blur-md">
                <div class="text-2xl font-bold text-red-400">
                  {{ aiStatus.failedToday }}
                </div>
                <div class="text-sm text-[var(--text-secondary)]">今日失败</div>
              </div>
            </div>
          </GlassCard>

          <div class="flex justify-end">
            <GlassButton :disabled="aiLoading" variant="primary" class="px-8 py-3" @click="saveAISettings">
              {{ aiLoading ? "保存中..." : "保存AI设置" }}
            </GlassButton>
          </div>
        </div>
      </div>

      <!-- 创建用户模态框 -->
      <div v-if="showCreateUserModal"
        class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-9999"
        @click="showCreateUserModal = false">
        <div
          class="bg-black/20 backdrop-blur-xl border border-[var(--border)] rounded-lg shadow-2xl p-6 max-w-md w-full mx-4 max-h-[90vh] overflow-y-auto"
          @click.stop>
          <h3 class="text-lg font-bold mb-4 text-[var(--text-primary)] flex items-center">
            <svg class="w-6 h-6 mr-2 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
            </svg>
            创建用户
          </h3>
          <form class="space-y-4" @submit.prevent="handleCreateUser">
            <div>
              <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">用户名</label>
              <input v-model="createUserForm.username" type="text" required
                class="w-full px-4 py-3 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)] transition-all"
                placeholder="输入用户名">
            </div>
            <div>
              <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">邮箱</label>
              <input v-model="createUserForm.email" type="email"
                class="w-full px-4 py-3 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)] transition-all"
                placeholder="输入邮箱（可选）">
            </div>
            <div>
              <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">密码</label>
              <input v-model="createUserForm.password" type="password" required
                class="w-full px-4 py-3 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:bg-[var(--bg-tertiary)] transition-all"
                placeholder="输入密码">
            </div>
            <div class="flex justify-end space-x-3 mt-6">
              <button type="button"
                class="px-6 py-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] bg-[var(--bg-tertiary)] hover:bg-[var(--border)] rounded-lg transition-all duration-200"
                @click="showCreateUserModal = false">
                取消
              </button>
              <button type="submit" :disabled="createUserLoading"
                class="px-6 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-[var(--text-primary)] rounded-lg transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center">
                <svg v-if="createUserLoading" class="animate-spin -ml-1 mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                  <path class="opacity-75" fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                </svg>
                {{ createUserLoading ? "创建中..." : "创建" }}
              </button>
            </div>
          </form>
        </div>
      </div>

      <!-- 安装插件模态框 -->
      <div v-if="showInstallPluginModal"
        class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-9999"
        @click="showInstallPluginModal = false">
        <div
          class="bg-black/20 backdrop-blur-xl border border-[var(--border)] rounded-lg shadow-2xl p-6 max-w-md w-full mx-4 max-h-[90vh] overflow-y-auto"
          @click.stop>
          <h3 class="text-lg font-bold mb-4 text-[var(--text-primary)] flex items-center">
            <svg class="w-6 h-6 mr-2 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
            </svg>
            安装插件
          </h3>
          <form class="space-y-4" @submit.prevent="handleInstallPlugin">
            <div>
              <label class="block text-sm font-medium text-[var(--text-primary)] mb-2">
                选择插件文件 (.zip)
              </label>
              <div class="relative">
                <input ref="pluginFileInput" type="file" accept=".zip" required
                  class="block w-full px-4 py-3 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded-lg text-[var(--text-primary)] file:mr-3 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-[var(--accent)] file:text-white hover:file:bg-[var(--accent-hover)] transition-all">
              </div>
              <p class="mt-2 text-xs text-[var(--text-secondary)]">支持 .zip 格式的插件文件</p>
            </div>
            <div class="flex justify-end space-x-3">
              <button type="button"
                class="px-6 py-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] bg-[var(--bg-tertiary)] hover:bg-[var(--border)] rounded-lg transition-all duration-200"
                @click="showInstallPluginModal = false">
                取消
              </button>
              <button type="submit" :disabled="installPluginLoading"
                class="px-6 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-[var(--text-primary)] rounded-lg transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center">
                <svg v-if="installPluginLoading" class="animate-spin -ml-1 mr-2 h-4 w-4" fill="none"
                  viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                  <path class="opacity-75" fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                </svg>
                {{ installPluginLoading ? "安装中..." : "安装" }}
              </button>
            </div>
          </form>
        </div>
      </div>

      <ConfirmModal
        :show="confirmDialog.show"
        :title="confirmDialog.title"
        :message="confirmDialog.message"
        :type="confirmDialog.type"
        :confirm-text="confirmDialog.confirmText"
        :cancel-text="confirmDialog.cancelText"
        :show-cancel="confirmDialog.showCancel"
        @close="handleConfirmDialogClose"
        @confirm="handleConfirmDialogConfirm"
      />

      <!-- 目录浏览器 -->
      <DirectoryBrowser :is-open="showDirectoryBrowser" :initial-path="directoryBrowserType === 'comics'
        ? systemSettings.comicsPath
        : cacheSettings.cachePath
        " @close="closeDirectoryBrowser" @select="handleDirectorySelected" />
    </div>
  </BasePageView>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, watch, h } from "vue";
import { useQuery, useQueryClient } from "@tanstack/vue-query";
import { useRoute, useRouter } from "vue-router";
import { useTheme } from "@/composables/useTheme";
import { useLibraryStore } from "@/stores/library";
import BasePageView from "@/components/layout/BasePageView.vue";
import DirectoryBrowser from "@/components/DirectoryBrowser.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassInput from "@/components/base/GlassInput.vue";
import ConfirmModal from "@/components/common/ConfirmModal.vue";
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
  clearCache as apiClearCache,
} from "@/utils/api";
import type { CacheClearScope } from "@/utils/api";
import type {
  SystemSettings,
  ScanSettings,
  User,
  Plugin,
  AISettings,
} from "@/types/api";

const queryClient = useQueryClient();
const route = useRoute();
const router = useRouter();
const { theme, setTheme } = useTheme();
const libraryStore = useLibraryStore();

// 主题选项
const themeOptions = [
  {
    value: 'light',
    label: '浅色',
    icon: () => h('svg', { class: 'w-8 h-8', fill: 'none', stroke: 'currentColor', viewBox: '0 0 24 24' }, [
      h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '2', d: 'M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z' })
    ])
  },
  {
    value: 'dark',
    label: '深色',
    icon: () => h('svg', { class: 'w-8 h-8', fill: 'none', stroke: 'currentColor', viewBox: '0 0 24 24' }, [
      h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '2', d: 'M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z' })
    ])
  },
  {
    value: 'system',
    label: '跟随系统',
    icon: () => h('svg', { class: 'w-8 h-8', fill: 'none', stroke: 'currentColor', viewBox: '0 0 24 24' }, [
      h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '2', d: 'M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z' })
    ])
  }
];

// 标签页管理
const ADMIN_TAB_IDS = ["system", "users", "plugins", "batch", "ai"] as const;
const USER_TAB_IDS = ["appearance"] as const;
const activeTab = ref<string>("appearance");
const isAdminSettingsRoute = computed(() => route.name === "admin-settings");
const isUserSettingsRoute = computed(() => route.name === "settings");
const pageTitle = computed(() =>
  isAdminSettingsRoute.value ? "管理" : "个人设置",
);
const tabs = computed(() =>
  isAdminSettingsRoute.value
    ? [
      { id: "system", name: "系统配置" },
      { id: "users", name: "用户管理" },
      { id: "plugins", name: "插件管理" },
      { id: "batch", name: "批量操作" },
      { id: "ai", name: "AI自动标签" },
    ]
    : [{ id: "appearance", name: "外观" }],
);

const resolveTabFromQuery = (queryTab: unknown, isAdmin: boolean): string => {
  const rawTab = Array.isArray(queryTab) ? queryTab[0] : queryTab;
  const candidate = typeof rawTab === "string" ? rawTab : "";
  const allowedTabs: readonly string[] = isAdmin ? ADMIN_TAB_IDS : USER_TAB_IDS;
  const fallbackTab = isAdmin ? "system" : "appearance";

  return allowedTabs.includes(candidate) ? candidate : fallbackTab;
};

const setActiveTab = (tabId: string) => {
  activeTab.value = resolveTabFromQuery(tabId, isAdminSettingsRoute.value);
};

watch(
  () => [route.name, route.query.tab],
  ([routeName, queryTab]) => {
    const isAdmin = routeName === "admin-settings";
    activeTab.value = resolveTabFromQuery(queryTab, isAdmin);
  },
  { immediate: true },
);

watch(
  () => activeTab.value,
  (tab) => {
    const currentTabQuery = Array.isArray(route.query.tab)
      ? route.query.tab[0]
      : route.query.tab;

    if (isAdminSettingsRoute.value) {
      if (currentTabQuery !== tab) {
        void router.replace({
          name: "admin-settings",
          query: { ...route.query, tab },
        });
      }
      return;
    }

    if (currentTabQuery !== undefined) {
      const { tab: _unusedTab, ...restQuery } = route.query;
      void router.replace({
        name: "settings",
        query: restQuery,
      });
    }
  },
);

// 系统设置
const systemSettings = ref<SystemSettings>({
  comicsPath: "./comics",
  supportedFormats: ["cbz", "cbr", "cb7", "zip", "rar"],
  maxFileSize: 100,
  imageCacheSize: 1024,
  imageCachePath: "./data/cache",
  scanOnStartup: true,
  scanSettings: {
    enabled: true,
    recursive: true,
    ignoreHidden: true,
    realtimeMonitoring: false,
  },
});

const cacheSettings = ref({
  cachePath: "./data/cache",
  maxSize: 1.0,
  quality: 85,
  format: "WebP",
  strategy: "balanced",
  customConfig: {
    maxMemoryMb: 512,
    maxCachedArchives: 30,
    cacheTtlHours: 24,
    preloadPrevPages: 2,
    preloadNextPages: 3,
  },
});

const scanSettings = ref<ScanSettings>({
  enabled: true,
  recursive: true,
  ignoreHidden: true,
  realtimeMonitoring: false,
});

const systemLoading = ref(false);
const scanLoading = ref(false);
const scanResult = ref<{ success: boolean; message: string } | null>(null);

// AI设置
const aiSettings = ref<AISettings>({
  enabled: false,
  autoApplyThreshold: 0.8,
  processingSchedule: "batch",
  maxConcurrentTasks: 2,
  enabledAnalyzers: [],
});

const aiLoading = ref(false);

// 缓存相关
interface CacheStatus {
  current_strategy: string;
  stats: {
    hit_rate: number;
    memory_usage_mb: number;
    cached_archives: number;
  };
}

const cacheStatus = ref<CacheStatus | null>(null);
const clearingCacheScope = ref<CacheClearScope | null>(null);
const isClearingCache = computed(() => clearingCacheScope.value !== null);

// 用户管理
const showCreateUserModal = ref(false);
const createUserForm = ref({
  username: "",
  email: "",
  password: "",
});
const createUserLoading = ref(false);

// 插件管理
const showInstallPluginModal = ref(false);
const pluginFileInput = ref<HTMLInputElement>();
const installPluginLoading = ref(false);

// 批量操作
const batchOperationLoading = ref(false);
const batchDeleteForm = ref({
  archiveIds: "",
  categoryId: "",
  tagId: "",
});

interface BatchOperationRecord {
  operation: string;
  timestamp: string;
  success: boolean;
  result: string;
}

const batchOperationHistory = ref<BatchOperationRecord[]>([]);

const getErrorMessage = (error: unknown, fallback: string): string => {
  if (typeof error !== "object" || error === null) return fallback;
  const withResponse = error as { response?: { data?: { message?: string } } };
  const responseMessage = withResponse.response?.data?.message;
  if (typeof responseMessage === "string" && responseMessage) return responseMessage;

  const withMessage = error as { message?: string };
  if (typeof withMessage.message === "string" && withMessage.message) {
    return withMessage.message;
  }

  return fallback;
};

type ConfirmDialogType = "default" | "danger" | "warning" | "info";

interface ConfirmDialogOptions {
  title?: string;
  message: string;
  type?: ConfirmDialogType;
  confirmText?: string;
  cancelText?: string;
  showCancel?: boolean;
}

const confirmDialog = ref({
  show: false,
  title: "确认操作",
  message: "",
  type: "default" as ConfirmDialogType,
  confirmText: "确认",
  cancelText: "取消",
  showCancel: true,
});

let confirmDialogResolver: ((result: boolean) => void) | null = null;

const askForConfirmation = (
  options: ConfirmDialogOptions,
): Promise<boolean> => {
  if (confirmDialogResolver) {
    confirmDialogResolver(false);
    confirmDialogResolver = null;
  }

  confirmDialog.value = {
    show: true,
    title: options.title ?? "确认操作",
    message: options.message,
    type: options.type ?? "default",
    confirmText: options.confirmText ?? "确认",
    cancelText: options.cancelText ?? "取消",
    showCancel: options.showCancel ?? true,
  };

  return new Promise((resolve) => {
    confirmDialogResolver = resolve;
  });
};

const resolveConfirmDialog = (result: boolean) => {
  confirmDialog.value.show = false;
  if (confirmDialogResolver) {
    confirmDialogResolver(result);
    confirmDialogResolver = null;
  }
};

const handleConfirmDialogClose = () => {
  resolveConfirmDialog(false);
};

const handleConfirmDialogConfirm = () => {
  resolveConfirmDialog(true);
};

const showInfoDialog = async (title: string, message: string) => {
  await askForConfirmation({
    title,
    message,
    type: "info",
    confirmText: "知道了",
    showCancel: false,
  });
};

// 查询数据
const { data: users, isLoading: usersLoading } = useQuery({
  queryKey: ["users"],
  queryFn: getUsers,
  enabled: () => isAdminSettingsRoute.value && activeTab.value === "users",
});

const { data: plugins, isLoading: pluginsLoading } = useQuery({
  queryKey: ["plugins"],
  queryFn: getPlugins,
  enabled: () => isAdminSettingsRoute.value && activeTab.value === "plugins",
});

const { data: aiStatus } = useQuery({
  queryKey: ["ai-status"],
  queryFn: getAIStatus,
  enabled: () => isAdminSettingsRoute.value && activeTab.value === "ai",
  refetchInterval: 5000,
});

const { data: categories } = useQuery({
  queryKey: ["categories"],
  queryFn: getCategories,
  enabled: () => isAdminSettingsRoute.value && activeTab.value === "batch",
});

const { data: tags } = useQuery({
  queryKey: ["tags"],
  queryFn: getTags,
  enabled: () => isAdminSettingsRoute.value && activeTab.value === "batch",
});

// 目录浏览相关
const showDirectoryBrowser = ref(false);
const directoryBrowserType = ref<"comics" | "cache">("comics");

// 系统设置相关方法
const selectComicsPath = () => {
  directoryBrowserType.value = "comics";
  showDirectoryBrowser.value = true;
};

const selectCachePath = () => {
  directoryBrowserType.value = "cache";
  showDirectoryBrowser.value = true;
};

const handleDirectorySelected = (path: string) => {
  if (directoryBrowserType.value === "comics") {
    systemSettings.value.comicsPath = path;
  } else if (directoryBrowserType.value === "cache") {
    cacheSettings.value.cachePath = path;
  }
  showDirectoryBrowser.value = false;
};

const closeDirectoryBrowser = () => {
  showDirectoryBrowser.value = false;
};

const saveSystemSettings = async () => {
  systemLoading.value = true;
  try {
    await updateSettings({
      ...systemSettings.value,
      maxFileSize: systemSettings.value.maxFileSize * 1024 * 1024, // 转换为字节
      imageCacheSize: cacheSettings.value.maxSize * 1024 * 1024 * 1024, // 转换为字节
      imageCachePath: cacheSettings.value.cachePath,
      scanSettings: scanSettings.value,
      imageCacheQuality: cacheSettings.value.quality,
      imageCacheFormat: cacheSettings.value.format,
    });

    // 保存缓存策略配置
    if (cacheSettings.value.strategy) {
      await configureCache({
        strategy:
          cacheSettings.value.strategy === "custom"
            ? undefined
            : cacheSettings.value.strategy,
        custom_config:
          cacheSettings.value.strategy === "custom"
            ? {
              max_memory_mb: cacheSettings.value.customConfig.maxMemoryMb,
              max_cached_archives:
                cacheSettings.value.customConfig.maxCachedArchives,
              cache_ttl_hours: cacheSettings.value.customConfig.cacheTtlHours,
              preload_prev_pages:
                cacheSettings.value.customConfig.preloadPrevPages,
              preload_next_pages:
                cacheSettings.value.customConfig.preloadNextPages,
            }
            : undefined,
      });
    }

    await showInfoDialog("操作成功", "系统设置已保存");
  } catch (error) {
    console.error("保存设置失败:", error);
    await showInfoDialog("操作失败", "保存失败");
  } finally {
    systemLoading.value = false;
  }
};

// 缓存相关方法
const getCacheStrategyDescription = () => {
  switch (cacheSettings.value.strategy) {
    case "conservative":
      return "保守策略：低内存使用，较短缓存时间，适合资源有限的系统";
    case "balanced":
      return "平衡策略：中等配置，适合大多数使用场景";
    case "aggressive":
      return "激进策略：高内存使用，长缓存时间，适合高性能系统";
    case "custom":
      return "自定义策略：根据您的需求自由配置缓存参数";
    default:
      return "";
  }
};

const handleCacheStrategyChange = () => {
  // 切换到预设策略时，更新默认值
  switch (cacheSettings.value.strategy) {
    case "conservative":
      cacheSettings.value.customConfig = {
        maxMemoryMb: 256,
        maxCachedArchives: 10,
        cacheTtlHours: 6,
        preloadPrevPages: 1,
        preloadNextPages: 2,
      };
      break;
    case "balanced":
      cacheSettings.value.customConfig = {
        maxMemoryMb: 512,
        maxCachedArchives: 30,
        cacheTtlHours: 24,
        preloadPrevPages: 2,
        preloadNextPages: 3,
      };
      break;
    case "aggressive":
      cacheSettings.value.customConfig = {
        maxMemoryMb: 1024,
        maxCachedArchives: 50,
        cacheTtlHours: 168,
        preloadPrevPages: 3,
        preloadNextPages: 5,
      };
      break;
  }
};

const clearCache = async (scope: CacheClearScope) => {
  const scopeNameMap: Record<CacheClearScope, string> = {
    pages: "阅读缓存",
    covers: "封面缓存",
    all: "全部缓存",
  };

  const confirmMessageMap: Record<CacheClearScope, string> = {
    pages: "确定要清理阅读缓存吗？这不会影响列表封面。",
    covers: "确定要清理封面缓存吗？清理后列表封面会重新生成。",
    all: "确定要清空全部缓存吗？包括阅读缓存和封面缓存。",
  };

  const confirmed = await askForConfirmation({
    title: "确认清理缓存",
    message: confirmMessageMap[scope],
    type: scope === "all" ? "danger" : "warning",
    confirmText: "确认清理",
  });

  if (!confirmed) {
    return;
  }

  clearingCacheScope.value = scope;
  try {
    const result = await apiClearCache(scope);
    // Immediately refresh cache status
    await loadCacheStatus();

    if (result.success) {
      await showInfoDialog("操作成功", `${scopeNameMap[scope]}已成功清理`);
    } else {
      await showInfoDialog(
        "操作完成",
        result.message || `${scopeNameMap[scope]}已清理`,
      );
    }
  } catch (error) {
    console.error("清空缓存失败:", error);
    await showInfoDialog("操作失败", `清理${scopeNameMap[scope]}失败`);
  } finally {
    clearingCacheScope.value = null;
  }
};

const loadCacheStatus = async () => {
  try {
    cacheStatus.value = await getCacheStatus();
  } catch (error) {
    console.error("加载缓存状态失败:", error);
  }
};

// AI设置相关方法
const saveAISettings = async () => {
  aiLoading.value = true;
  try {
    await updateAISettings(aiSettings.value);
    await showInfoDialog("操作成功", "AI设置已保存");
  } catch (error) {
    console.error("保存AI设置失败:", error);
    await showInfoDialog("操作失败", "保存失败");
  } finally {
    aiLoading.value = false;
  }
};

// 用户管理相关方法
const handleCreateUser = async () => {
  const username = createUserForm.value.username.trim();
  const email = createUserForm.value.email.trim();
  if (!username || !createUserForm.value.password) return;

  createUserLoading.value = true;
  try {
    await createUser({
      username,
      password: createUserForm.value.password,
      email: email || undefined,
    });
    queryClient.invalidateQueries({ queryKey: ["users"] });
    showCreateUserModal.value = false;
    createUserForm.value = { username: "", email: "", password: "" };
  } catch (error: unknown) {
    console.error("创建用户失败:", error);
    await showInfoDialog("操作失败", getErrorMessage(error, "创建用户失败"));
  } finally {
    createUserLoading.value = false;
  }
};

const editUser = (user: User) => {
  console.log("Edit user:", user);
};

const confirmDeleteUser = async (user: User) => {
  const confirmed = await askForConfirmation({
    title: "确认删除用户",
    message: `确定要删除用户 "${user.username}" 吗？`,
    type: "danger",
    confirmText: "删除",
  });

  if (!confirmed) return;

  try {
    await deleteUser(user.id);
    queryClient.invalidateQueries({ queryKey: ["users"] });
  } catch (error) {
    console.error("删除用户失败:", error);
    await showInfoDialog("操作失败", getErrorMessage(error, "删除用户失败"));
  }
};

// 插件管理相关方法
const handleInstallPlugin = async () => {
  if (!pluginFileInput.value?.files?.[0]) return;

  installPluginLoading.value = true;
  try {
    const formData = new FormData();
    formData.append("plugin", pluginFileInput.value.files[0]);

    await installPlugin(formData);
    queryClient.invalidateQueries({ queryKey: ["plugins"] });
    showInstallPluginModal.value = false;
    if (pluginFileInput.value) {
      pluginFileInput.value.value = "";
    }
  } catch (error) {
    console.error("安装插件失败:", error);
  } finally {
    installPluginLoading.value = false;
  }
};

const handleTogglePlugin = async (plugin: Plugin) => {
  try {
    await togglePlugin(plugin.id);
    queryClient.invalidateQueries({ queryKey: ["plugins"] });
  } catch (error) {
    console.error("切换插件状态失败:", error);
  }
};

const configurePlugin = (plugin: Plugin) => {
  console.log("Configure plugin:", plugin);
};

// 批量操作相关方法
const addOperationRecord = (
  operation: string,
  success: boolean,
  result: string,
) => {
  batchOperationHistory.value.unshift({
    operation,
    timestamp: new Date().toLocaleString("zh-CN"),
    success,
    result,
  });
};

const handleBatchDeleteArchives = async () => {
  const confirmed = await askForConfirmation({
    title: "确认批量删除",
    message: "确定要执行批量删除漫画操作吗？此操作不可撤销！",
    type: "danger",
    confirmText: "确认删除",
  });

  if (!confirmed) {
    return;
  }

  batchOperationLoading.value = true;
  try {
    const archiveIds = batchDeleteForm.value.archiveIds
      ? batchDeleteForm.value.archiveIds.split(",").map((id) => id.trim())
      : [];

    await batchDeleteArchives(archiveIds);
    addOperationRecord(
      "批量删除漫画",
      true,
      `删除了 ${archiveIds.length || "所有"} 个漫画`,
    );
    batchDeleteForm.value.archiveIds = "";

    // 刷新相关数据
    queryClient.invalidateQueries({ queryKey: ["archives"] });
  } catch (error) {
    console.error("批量删除漫画失败:", error);
    addOperationRecord("批量删除漫画", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handleBatchDeleteCategoryArchives = async () => {
  const confirmed = await askForConfirmation({
    title: "确认按分类删除",
    message: "确定要删除该分类下的所有漫画吗？此操作不可撤销！",
    type: "danger",
    confirmText: "确认删除",
  });

  if (!confirmed) {
    return;
  }

  batchOperationLoading.value = true;
  try {
    const categoryId = batchDeleteForm.value.categoryId;
    await batchDeleteCategoryArchives(categoryId);

    const categoryName =
      categories.value?.find((c) => c.id === categoryId)?.name || categoryId;
    addOperationRecord(
      "按分类删除漫画",
      true,
      `删除了分类 "${categoryName}" 下的所有漫画`,
    );
    batchDeleteForm.value.categoryId = "";

    // 刷新相关数据
    queryClient.invalidateQueries({ queryKey: ["archives"] });
    queryClient.invalidateQueries({ queryKey: ["categories"] });
  } catch (error) {
    console.error("按分类删除漫画失败:", error);
    addOperationRecord("按分类删除漫画", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handleBatchDeleteTagArchives = async () => {
  const confirmed = await askForConfirmation({
    title: "确认按标签删除",
    message: "确定要删除该标签下的所有漫画吗？此操作不可撤销！",
    type: "danger",
    confirmText: "确认删除",
  });

  if (!confirmed) {
    return;
  }

  batchOperationLoading.value = true;
  try {
    const tagId = batchDeleteForm.value.tagId;
    await batchDeleteTagArchives(tagId);

    const tag = tags.value?.find((t) => t.id === tagId);
    const tagName = tag ? `${tag.namespace}:${tag.name}` : tagId;
    addOperationRecord(
      "按标签删除漫画",
      true,
      `删除了标签 "${tagName}" 下的所有漫画`,
    );
    batchDeleteForm.value.tagId = "";

    // 刷新相关数据
    queryClient.invalidateQueries({ queryKey: ["archives"] });
    queryClient.invalidateQueries({ queryKey: ["tags"] });
  } catch (error) {
    console.error("按标签删除漫画失败:", error);
    addOperationRecord("按标签删除漫画", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handlePruneTags = async () => {
  const confirmed = await askForConfirmation({
    title: "确认清理标签",
    message: "确定要清理所有无用的标签吗？",
    type: "warning",
    confirmText: "确认清理",
  });

  if (!confirmed) {
    return;
  }

  batchOperationLoading.value = true;
  try {
    await pruneTags();
    addOperationRecord("清理无用标签", true, "成功清理了所有未使用的标签");

    // 刷新标签数据
    queryClient.invalidateQueries({ queryKey: ["tags"] });
  } catch (error) {
    console.error("清理标签失败:", error);
    addOperationRecord("清理无用标签", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handlePruneCategories = async () => {
  const confirmed = await askForConfirmation({
    title: "确认清理分类",
    message: "确定要清理所有空分类吗？",
    type: "warning",
    confirmText: "确认清理",
  });

  if (!confirmed) {
    return;
  }

  batchOperationLoading.value = true;
  try {
    await pruneCategories();
    addOperationRecord("清理空分类", true, "成功清理了所有空分类");

    // 刷新分类数据
    queryClient.invalidateQueries({ queryKey: ["categories"] });
  } catch (error) {
    console.error("清理分类失败:", error);
    addOperationRecord("清理空分类", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

// 工具方法
const formatDate = (dateString: string) => {
  const date = new Date(dateString);
  return date.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
};

const formatHitRate = (hitRate: number | undefined) => {
  if (hitRate === undefined || hitRate === null || isNaN(hitRate)) {
    return "0.0%";
  }
  return (hitRate * 100).toFixed(1) + "%";
};

const loadAdminData = async () => {
  try {
    const settings = await getSettings();
    systemSettings.value = {
      ...settings,
      maxFileSize: Math.round(settings.maxFileSize / (1024 * 1024)), // 转换为MB
    };
    cacheSettings.value = {
      cachePath: settings.imageCachePath || "",
      maxSize: settings.imageCacheSize / (1024 * 1024 * 1024), // 转换为GB
      quality: settings.imageCacheQuality || 85,
      format: settings.imageCacheFormat || "WebP",
      strategy: "balanced", // 默认策略
      customConfig: {
        maxMemoryMb: 512,
        maxCachedArchives: 30,
        cacheTtlHours: 24,
        preloadPrevPages: 2,
        preloadNextPages: 3,
      },
    };
  } catch (error) {
    console.error("加载设置失败:", error);
  }

  try {
    const aiConfig = await getAISettings();
    aiSettings.value = aiConfig;
  } catch (error) {
    console.error("加载AI设置失败:", error);
  }

  // 加载扫描设置
  try {
    const scanConfig = await getScanSettings();
    if (scanConfig && scanConfig.scanSettings) {
      scanSettings.value = scanConfig.scanSettings;
    }
  } catch (error) {
    console.error("加载扫描设置失败:", error);
  }

  // 加载缓存状态
  await loadCacheStatus();
};

// 初始化
onMounted(async () => {
  if (!isAdminSettingsRoute.value) {
    return;
  }

  await loadAdminData();
});

watch(isAdminSettingsRoute, (isAdmin, wasAdmin) => {
  if (isAdmin && !wasAdmin) {
    void loadAdminData();
  }
});

// 手动扫描相关方法
const handleManualScan = async () => {
  scanLoading.value = true;
  scanResult.value = null;

  try {
    const result = await triggerScan();
    scanResult.value = {
      success: true,
      message: result.message,
    };
    // 刷新漫画列表数据
    queryClient.invalidateQueries({ queryKey: ["archives"] });
  } catch (error) {
    console.error("手动扫描失败:", error);
    scanResult.value = {
      success: false,
      message: "扫描失败，请检查漫画库路径是否正确",
    };
  } finally {
    scanLoading.value = false;
  }
};

// 保存扫描设置
const saveScanSettings = async () => {
  systemLoading.value = true;

  try {
    const result = await updateScanSettings(scanSettings.value);
    console.log("扫描设置保存成功:", result);

    // 显示成功消息
    scanResult.value = {
      success: true,
      message: `扫描设置已更新，实时监控状态: ${result.monitoring_status ? "已启用" : "已禁用"}`,
    };
  } catch (error) {
    console.error("保存扫描设置失败:", error);
    scanResult.value = {
      success: false,
      message: "保存扫描设置失败，请稍后重试",
    };
  } finally {
    systemLoading.value = false;
  }
};
</script>

<style scoped>
/* 只保留Settings页面特有的样式 */
.settings-view {
  width: 100%;
  min-height: calc(100vh - 4rem);
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

.space-y-6>* {
  width: 100%;
  box-sizing: border-box;
}

/* 网格布局确保正确宽度 */
.grid {
  width: 100%;
}

/* 输入元素确保正确宽度 */
input,
select,
textarea {
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
