<template>
  <div class="space-y-6">
    <SettingsSaveBar
      v-if="section !== 'overview' && section !== 'review'"
      :dirty="aiDirty"
      :saving="aiLoading"
      :saved-message="savedMessage"
      @save="emit('save')"
      @discard="emit('discard')"
    />

    <GlassCard v-if="section === 'overview'" size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">智能处理工作台</h2>
          <p class="mt-1 max-w-2xl text-sm text-[var(--text-secondary)]">
            从这里确认处理能力是否就绪，再进入模型、规则或任务页面。智能处理不会阻塞漫画入库和阅读。
          </p>
        </div>
        <span
          class="rounded-md border px-2 py-1 text-xs"
          :class="activeProfile?.enabled ? 'border-green-500/40 bg-green-500/10 text-green-700 dark:text-green-300' : 'border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300'"
        >
          {{ activeProfile?.enabled ? "基础能力已就绪" : "需要配置模型" }}
        </span>
      </div>

      <div class="mt-5 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3">
          <div class="text-xs text-[var(--text-secondary)]">首选模型</div>
          <div class="mt-1 truncate text-sm font-medium text-[var(--text-primary)]">
            {{ activeProfile?.name || "未配置" }}
          </div>
          <div class="mt-1 truncate text-xs text-[var(--text-tertiary)]">
            {{ activeProfile?.connection.model || "添加一个可用模型" }}
          </div>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3">
          <div class="text-xs text-[var(--text-secondary)]">视觉能力</div>
          <div class="mt-1 text-sm font-medium text-[var(--text-primary)]">
            {{ activeProfile?.connection.visionCapable ? "可用于页面分析" : "文本模型" }}
          </div>
          <div class="mt-1 text-xs text-[var(--text-tertiary)]">没有视觉模型时可走 OCR 辅助路径</div>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3">
          <div class="text-xs text-[var(--text-secondary)]">待处理任务</div>
          <div class="mt-1 text-sm font-medium text-[var(--text-primary)]">{{ aiStatus?.queueSize ?? "-" }}</div>
          <div class="mt-1 text-xs text-[var(--text-tertiary)]">处理中 {{ aiStatus?.processingCount ?? "-" }}</div>
        </div>
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3">
          <div class="text-xs text-[var(--text-secondary)]">待处理失败</div>
          <div class="mt-1 text-sm font-medium text-[var(--text-primary)]">{{ aiStatus?.unresolvedFailureCount ?? "-" }}</div>
          <div class="mt-1 text-xs text-[var(--text-tertiary)]">需要在审核与任务中处理</div>
        </div>
      </div>

      <div class="mt-6 border-t border-[var(--border)] pt-5">
        <h3 class="text-sm font-medium text-[var(--text-primary)]">处理链路</h3>
        <div class="mt-3 grid grid-cols-1 gap-2 text-sm sm:grid-cols-5">
          <div v-for="(step, index) in [
            { name: '新入库', detail: '扫描完成' },
            { name: '内容理解', detail: '模型 + OCR' },
            { name: '标题翻译', detail: aiSettings.features.titleTranslation.enabled ? '已启用' : '未启用' },
            { name: '自动标签', detail: aiSettings.features.autoTagging.enabled ? '已启用' : '未启用' },
            { name: '推荐特征', detail: '后台使用' },
          ]" :key="step.name" class="relative rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3">
            <div class="font-medium text-[var(--text-primary)]">{{ step.name }}</div>
            <div class="mt-1 text-xs text-[var(--text-secondary)]">{{ step.detail }}</div>
            <span v-if="index < 4" class="pointer-events-none absolute -right-2 top-1/2 hidden text-[var(--text-tertiary)] sm:block">›</span>
          </div>
        </div>
      </div>
    </GlassCard>

    <GlassCard v-if="section === 'models'" size="md" radius="lg">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">AI 连接</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            当前选中的配置优先执行；连接、限流或服务端错误时会依次切换到其他已启用配置。
          </p>
        </div>
        <GlassButton variant="secondary" size="sm" @click="addProfile">
          添加配置
        </GlassButton>
      </div>

      <div class="mb-5 flex flex-wrap gap-2">
        <button
          v-for="profile in aiSettings.profiles"
          :key="profile.id"
          type="button"
          class="rounded-lg border px-3 py-2 text-left text-sm transition-colors"
          :class="
            profile.id === aiSettings.activeProfileId
              ? 'border-[var(--accent)] bg-[var(--accent)]/10 text-[var(--text-primary)]'
              : 'border-[var(--border)] bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
          "
          @click="aiSettings.activeProfileId = profile.id"
        >
          <span class="block font-medium">{{
            profile.name || "未命名配置"
          }}</span>
          <span class="block text-xs opacity-75">{{
            profile.id === aiSettings.activeProfileId
              ? "首选配置"
              : profile.enabled
                ? "回退候选"
                : "已停用"
          }}</span>
        </button>
      </div>

      <div class="mb-5 flex flex-wrap items-center gap-2 text-xs text-[var(--text-secondary)]">
        <span>回退顺序按上方配置顺序执行：</span>
        <GlassButton
          title="上移首选配置"
          size="xs"
          variant="secondary"
          :disabled="activeProfileIndex <= 0"
          @click="moveActiveProfile(-1)"
        >
          <template #icon><ChevronUpIcon class="h-4 w-4" /></template>
        </GlassButton>
        <GlassButton
          title="下移首选配置"
          size="xs"
          variant="secondary"
          :disabled="activeProfileIndex < 0 || activeProfileIndex >= aiSettings.profiles.length - 1"
          @click="moveActiveProfile(1)"
        >
          <template #icon><ChevronDownIcon class="h-4 w-4" /></template>
        </GlassButton>
      </div>

      <div v-if="activeProfile" class="space-y-4">
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >配置名称</label
            >
            <input
              v-model.trim="activeProfile.name"
              type="text"
              autocomplete="off"
              placeholder="例如 Ollama 本地"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>

          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >模型</label
            >
            <input
              v-model.trim="activeProfile.connection.model"
              type="text"
              autocomplete="off"
              placeholder="例如 gpt-4o-mini 或 qwen3:8b"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>
        </div>

        <label
          class="flex items-start gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="activeProfile.connection.visionCapable"
            type="checkbox"
            class="mt-0.5 rounded"
          />
          <span>
            <span class="block">此模型支持图片输入</span>
            <span class="mt-1 block text-xs text-[var(--text-secondary)]">
              启用后可用于内容分析和 AI 自动标签；未启用时仍可用于标题翻译。
            </span>
          </span>
        </label>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <label
            class="flex items-start gap-2 text-sm text-[var(--text-primary)]"
          >
            <input
              v-model="activeProfile.connection.streamResponse"
              type="checkbox"
              class="mt-0.5 rounded"
            />
              <span>
                <span class="block">使用流式响应</span>
                <span class="mt-1 block text-xs text-[var(--text-secondary)]">
                OpenAI 兼容接口使用 SSE，Ollama 原生接口使用 NDJSON；任务仍会在完整结果校验后完成。
                </span>
              </span>
          </label>

          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >首字超时（秒）</label
            >
            <input
              v-model.number="activeProfile.connection.firstTokenTimeoutSeconds"
              type="number"
              min="1"
              :max="aiSettings.execution.timeoutSeconds"
              :disabled="!activeProfile.connection.streamResponse"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-60"
            />
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              从发送请求到收到首个模型输出的上限；仅流式响应生效，且不能超过总超时。
            </p>
          </div>
        </div>

        <div
          v-if="activeProfile.connection.provider === 'ollama'"
          class="grid grid-cols-1 gap-4 sm:grid-cols-2"
        >
          <label class="flex items-start gap-2 text-sm text-[var(--text-primary)]">
            <input
              v-model="activeProfile.connection.ollamaUseGpu"
              type="checkbox"
              class="mt-0.5 rounded"
            />
            <span>
              <span class="block">尽量使用 GPU 加速</span>
              <span class="mt-1 block text-xs text-[var(--text-secondary)]">
                向 Ollama 发送 <code>num_gpu: -1</code>，将可卸载的模型层尽量放到 GPU。
              </span>
            </span>
          </label>

          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >上下文窗口（num_ctx）</label
            >
            <input
              v-model.number="activeProfile.connection.ollamaMaxNumCtx"
              type="number"
              min="0"
              max="1048576"
              step="256"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-60"
            />
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              为当前模型直接设置 <code>num_ctx</code>；0 表示采用 Ollama 的模型默认值。视觉模型建议从 16384 开始。
            </p>
          </div>

          <label class="flex items-start gap-2 text-sm text-[var(--text-primary)]">
            <input
              v-model="activeProfile.connection.ollamaThinking"
              type="checkbox"
              class="mt-0.5 rounded"
            />
            <span>
              <span class="block">启用思考输出</span>
              <span class="mt-1 block text-xs text-[var(--text-secondary)]">
                向支持该能力的模型发送 <code>think: true</code>；结构化任务通常保持关闭，以减少输出长度。
              </span>
            </span>
          </label>
        </div>

        <div class="max-w-sm">
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >请求间隔（秒）</label
          >
          <input
            v-model.number="activeProfile.connection.requestIntervalSeconds"
            type="number"
            min="0"
            max="3600"
            step="1"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
          <p class="mt-1 text-xs text-[var(--text-secondary)]">
            两次向此模型发起请求之间至少等待的时间。用于降低本地模型的显卡散热压力；0 表示不额外等待。
          </p>
        </div>

        <div class="max-w-sm">
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >总请求超时（秒）</label
          >
          <input
            v-model.number="aiSettings.execution.timeoutSeconds"
            type="number"
            min="10"
            max="1800"
            @change="clampProfileFirstTokenTimeouts"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
          <p class="mt-1 text-xs text-[var(--text-secondary)]">
            所有模型请求从建立连接到收完响应的总上限。默认 180 秒；本地模型或视觉分析可按需提高。
          </p>
        </div>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >服务协议</label
            >
            <select
              v-model="activeProfile.connection.provider"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              @change="applyProviderDefaults"
            >
              <option value="openaiCompatible">OpenAI-compatible Chat Completions</option>
              <option value="ollama">Ollama 原生 API</option>
            </select>
          </div>
          <label
            class="flex items-center gap-2 self-end pb-2 text-sm text-[var(--text-primary)]"
          >
            <input
              v-model="activeProfile.enabled"
              type="checkbox"
              class="rounded"
              :disabled="!canToggleActiveProfile"
              @change="switchFromDisabledActiveProfile"
            />
            启用此配置
          </label>
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >Base URL</label
          >
          <input
            v-model.trim="activeProfile.connection.baseUrl"
            type="url"
            autocomplete="url"
            :placeholder="baseUrlPlaceholder"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >认证方式</label
            >
            <select
              v-model="activeProfile.connection.authMode"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            >
              <option value="bearer">Bearer API Key</option>
              <option value="none">无认证</option>
            </select>
          </div>
          <div v-if="activeProfile.connection.authMode === 'bearer'">
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >API Key</label
            >
            <input
              v-model="activeProfile.connection.apiKey"
              type="password"
              autocomplete="new-password"
              :placeholder="apiKeyPlaceholder"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              {{ apiKeyHint }}
            </p>
          </div>
        </div>

        <div class="flex flex-wrap gap-2">
          <GlassButton
            :disabled="
              aiLoading || !canTestConnection || !activeProfile.enabled
            "
            :loading="testingConnection"
            loading-text="测试中..."
            variant="secondary"
            size="sm"
            @click="emit('test-connection')"
          >
            测试连接
          </GlassButton>
          <GlassButton
            :disabled="aiLoading || aiSettings.profiles.length === 1"
            variant="danger"
            size="sm"
            @click="removeActiveProfile"
          >
            删除配置
          </GlassButton>
        </div>
      </div>
    </GlassCard>

    <GlassCard v-if="section === 'automation'" size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">内容分析</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            每本新漫画入库后会在后台抽取封面和部分正文页，识别题材与内容特征，为随机精选和偏好规则提供依据。
          </p>
        </div>
        <span class="rounded-md border border-[var(--border)] bg-[var(--bg-tertiary)] px-2 py-1 text-xs text-[var(--text-secondary)]">异步执行</span>
      </div>
      <dl class="mt-5 divide-y divide-[var(--border)] border-y border-[var(--border)]">
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">模型要求</dt>
          <dd class="text-[var(--text-secondary)]">
            优先使用支持图片输入的视觉模型；没有可用视觉模型时，会结合标题、OCR
            和插件元数据完成文本分析。
          </dd>
        </div>
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">何时生效</dt>
          <dd class="text-[var(--text-secondary)]">扫描新漫画后自动入队，不阻塞入库或阅读；分析失败时保留漫画并在后台重试。</dd>
        </div>
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">OCR 辅助</dt>
          <dd class="text-[var(--text-secondary)]">可在“OCR 辅助”中启用本地文字识别，为页面分析补充文字线索；它不提供阅读器翻译或全库文字搜索。</dd>
        </div>
      </dl>

      <div class="mt-5 border-t border-[var(--border)] pt-5">
        <h3 class="text-sm font-medium text-[var(--text-primary)]">推荐方式</h3>
        <p class="mt-1 text-sm text-[var(--text-secondary)]">
          默认按每位读者的阅读喜好推荐。多人使用时，可选择帮助改进推荐，系统会比较两种选书方式的整体阅读表现。
        </p>
        <div role="radiogroup" aria-label="推荐方式" class="mt-3 inline-flex overflow-hidden rounded-lg border border-[var(--border)]">
          <button
            type="button"
            role="radio"
            :aria-checked="!aiSettings.features.recommendations.multiUserExperimentEnabled"
            class="border-r border-[var(--border)] px-3 py-2 text-sm transition-colors"
            :class="
              !aiSettings.features.recommendations.multiUserExperimentEnabled
                ? 'bg-[var(--accent)] text-white'
                : 'bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
            "
            @click="aiSettings.features.recommendations.multiUserExperimentEnabled = false"
          >
            按你的喜好推荐
          </button>
          <button
            type="button"
            role="radio"
            :aria-checked="aiSettings.features.recommendations.multiUserExperimentEnabled"
            class="px-3 py-2 text-sm transition-colors"
            :class="
              aiSettings.features.recommendations.multiUserExperimentEnabled
                ? 'bg-[var(--accent)] text-white'
                : 'bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
            "
            @click="aiSettings.features.recommendations.multiUserExperimentEnabled = true"
          >
            帮助改进推荐
          </button>
        </div>
        <p v-if="aiSettings.features.recommendations.multiUserExperimentEnabled" class="mt-2 text-xs text-[var(--text-secondary)]">
          系统会稳定地将少部分读者放入对照组；读者较少时，结果只供参考。
        </p>
      </div>

      <div class="mt-5 max-w-sm">
        <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">
          内容理解更新间隔（天）
        </label>
        <input
          v-model.number="aiSettings.features.recommendations.analysisRefreshAfterDays"
          type="number"
          min="30"
          max="730"
          step="1"
          class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        />
        <p class="mt-1 text-xs text-[var(--text-secondary)]">
          一部漫画再次获得阅读反馈，且上次内容理解已超过这个时间时，系统会在后台更新分析；不会影响正在阅读的内容。默认 180 天。
        </p>
      </div>
    </GlassCard>

    <GlassCard v-if="section === 'runtime'" size="md" radius="lg">
      <h2 class="mb-4 text-lg font-medium text-[var(--text-primary)]">
        执行参数
      </h2>
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >模型请求并发</label
          >
          <input
            v-model.number="llmConcurrency"
            type="number"
            min="1"
            max="16"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >OCR 并发</label
          >
          <input
            v-model.number="ocrConcurrency"
            type="number"
            min="1"
            max="16"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >插件并发</label
          >
          <input
            v-model.number="pluginConcurrency"
            type="number"
            min="1"
            max="16"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >编排并发</label
          >
          <input
            v-model.number="orchestrationConcurrency"
            type="number"
            min="1"
            max="16"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>
      </div>
      <div class="mt-4 max-w-sm">
        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >最大重试次数</label
          >
          <input
            v-model.number="aiSettings.execution.maxRetries"
            type="number"
            min="0"
            max="10"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>
      </div>
      <div class="mt-6 border-t border-[var(--border)] pt-5">
        <h3 class="text-sm font-medium text-[var(--text-primary)]">任务输入与输出预算</h3>
        <div class="mt-3 grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">最大输出 token</label>
            <input v-model.number="aiSettings.execution.outputTokenLimit" type="number" min="128" max="32768" step="1" class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]" />
          </div>
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">提示词安全余量 token</label>
            <input v-model.number="aiSettings.execution.promptSafetyMargin" type="number" min="0" max="16384" step="1" class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]" />
          </div>
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">每任务最大图片数</label>
            <input v-model.number="aiSettings.execution.maxImagesPerTask" type="number" min="1" max="64" step="1" class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]" />
          </div>
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">每张图片 token 预算</label>
            <input v-model.number="aiSettings.execution.imageTokenBudget" type="number" min="256" max="32768" step="1" class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]" />
          </div>
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">上下文溢出重试</label>
            <input v-model.number="aiSettings.execution.adaptiveContextRetries" type="number" min="0" max="5" step="1" class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]" />
          </div>
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">OCR 最大页面数</label>
            <input v-model.number="aiSettings.execution.ocrMaxPages" type="number" min="1" max="64" step="1" class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]" />
          </div>
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">每页 OCR 最大字符</label>
            <input v-model.number="aiSettings.execution.ocrCharsPerPage" type="number" min="100" max="20000" step="1" class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]" />
          </div>
        </div>
        <p class="mt-2 text-xs text-[var(--text-secondary)]">最大输出 token 会作为 OpenAI-compatible 的 <code>max_tokens</code> 和 Ollama 的 <code>num_predict</code> 发送。</p>
      </div>
    </GlassCard>

    <GlassCard v-if="section === 'automation'" size="md" radius="lg">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            标题翻译
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            译文作为副标题显示，原始标题保持不变。
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <GlassButton
            :disabled="
              aiLoading ||
              repairingTranslations ||
              retranslatingTranslations ||
              !aiSettings.features.titleTranslation.enabled
            "
            :loading="backfillingTranslations"
            loading-text="加入队列中..."
            variant="secondary"
            size="sm"
            @click="emit('backfill-title-translations')"
          >
            {{ aiDirty ? "保存并加入队列" : "批量补翻译" }}
          </GlassButton>
          <GlassButton
            :disabled="
              aiLoading ||
              backfillingTranslations ||
              retranslatingTranslations ||
              !aiSettings.features.titleTranslation.enabled
            "
            :loading="repairingTranslations"
            loading-text="筛选并入队中..."
            variant="secondary"
            size="sm"
            @click="emit('repair-suspicious-title-translations')"
          >
            修复失败/疑似拒答
          </GlassButton>
          <GlassButton
            :disabled="
              aiLoading ||
              backfillingTranslations ||
              repairingTranslations ||
              !aiSettings.features.titleTranslation.enabled
            "
            :loading="retranslatingTranslations"
            loading-text="重新入队中..."
            variant="secondary"
            size="sm"
            @click="emit('force-retranslate-title-translations')"
          >
            <template #icon>
              <ArrowPathIcon class="mr-1.5 h-4 w-4" />
            </template>
            全部重新翻译
          </GlassButton>
        </div>
      </div>

      <div class="space-y-4">
        <label
          class="flex items-center gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.titleTranslation.enabled"
            type="checkbox"
            class="rounded"
          />
          自动翻译刮削后的标题
        </label>

        <label
          class="flex items-start gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.titleTranslation.displayTranslatedTitle"
            type="checkbox"
            class="mt-0.5 rounded"
          />
          <span>
            <span class="block">主标题显示译文</span>
            <span class="mt-1 block text-xs text-[var(--text-secondary)]">
              有可用译文时，译文显示为主标题，原始标题显示为副标题；没有译文时自动回退为原始标题。
            </span>
          </span>
        </label>

        <div class="max-w-sm">
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >目标语言</label
          >
          <select
            v-model="aiSettings.features.titleTranslation.targetLanguage"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          >
            <option
              v-if="
                !isKnownTargetLanguage(
                  aiSettings.features.titleTranslation.targetLanguage,
                )
              "
              :value="aiSettings.features.titleTranslation.targetLanguage"
            >
              当前配置（{{
                aiSettings.features.titleTranslation.targetLanguage
              }}）
            </option>
            <option
              v-for="language in titleTranslationLanguages"
              :key="language.code"
              :value="language.code"
            >
              {{ language.label }}
            </option>
          </select>
        </div>

        <label
          class="flex items-center gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.titleTranslation.skipIfTargetLanguage"
            type="checkbox"
            class="rounded"
          />
          跳过已为目标语言的标题
        </label>

        <label
          class="flex items-center gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="
              aiSettings.features.titleTranslation.retranslateOnTitleChange
            "
            type="checkbox"
            class="rounded"
          />
          原始标题变更后重新翻译
        </label>

        <div class="border-t border-[var(--border)] pt-5">
          <h3 class="text-sm font-medium text-[var(--text-primary)]">标题生成参数</h3>
          <div class="mt-3 grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <div>
              <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">温度</label>
              <input
                v-model.number="aiSettings.features.titleTranslation.temperature"
                type="number"
                min="0"
                max="2"
                step="0.05"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">结构化输出</label>
              <select
                v-model="aiSettings.features.titleTranslation.structuredOutputMode"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              >
                <option value="jsonSchema">JSON Schema</option>
                <option value="jsonObject">JSON object</option>
                <option value="promptOnly">仅提示词</option>
              </select>
            </div>
            <div>
              <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">Ollama repeat_penalty</label>
              <input
                v-model.number="aiSettings.features.titleTranslation.ollamaRepeatPenalty"
                type="number"
                min="0"
                max="2"
                step="0.01"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">Ollama repeat_last_n</label>
              <input
                v-model.number="aiSettings.features.titleTranslation.ollamaRepeatLastN"
                type="number"
                min="0"
                max="32768"
                step="1"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
          </div>
          <p class="mt-2 text-xs text-[var(--text-secondary)]">
            输出字段始终固定为 <code>{"title":"..."}</code>。JSON Schema 使用提供商原生 schema；不兼容时选择 JSON object 或仅提示词。
          </p>
        </div>

        <div class="border-t border-[var(--border)] pt-5">
          <h3 class="text-sm font-medium text-[var(--text-primary)]">标题翻译试运行</h3>
          <div class="mt-3 flex flex-col gap-3 sm:flex-row">
            <input
              v-model="previewTitle"
              type="text"
              autocomplete="off"
              placeholder="输入一个标题"
              class="min-w-0 flex-1 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              @keyup.enter="emit('preview-title-translation', previewTitle)"
            />
            <GlassButton
              size="sm"
              variant="secondary"
              :disabled="!previewTitle.trim() || previewingTitleTranslation"
              :loading="previewingTitleTranslation"
              loading-text="运行中..."
              @click="emit('preview-title-translation', previewTitle)"
            >
              试运行
            </GlassButton>
          </div>
          <p
            v-if="titleTranslationPreview && !titleTranslationPreview.success"
            class="mt-3 text-sm text-red-500"
          >
            {{ titleTranslationPreview.message || "试运行失败" }}
          </p>
          <div v-else-if="titleTranslationPreview?.preview" class="mt-3 space-y-3 text-sm">
            <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
              <div><span class="text-[var(--text-secondary)]">解析标题</span><p class="mt-1 break-words text-[var(--text-primary)]">{{ titleTranslationPreview.preview.parsedTitle || "-" }}</p></div>
              <div><span class="text-[var(--text-secondary)]">结束原因</span><p class="mt-1 break-words text-[var(--text-primary)]">{{ titleTranslationPreview.preview.finishReason || "-" }}</p></div>
              <div><span class="text-[var(--text-secondary)]">校验 / 耗时</span><p class="mt-1 break-words" :class="titleTranslationPreview.preview.validationError ? 'text-red-500' : 'text-green-600 dark:text-green-400'">{{ titleTranslationPreview.preview.validationError || "通过" }} · {{ titleTranslationPreview.preview.elapsedMs }} ms</p></div>
            </div>
            <details>
              <summary class="cursor-pointer text-[var(--text-secondary)]">最终请求</summary>
              <pre class="mt-2 max-h-64 overflow-auto whitespace-pre-wrap break-words rounded border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-xs text-[var(--text-primary)]">{{ formattedTitleTranslationPreviewRequest }}</pre>
            </details>
            <details>
              <summary class="cursor-pointer text-[var(--text-secondary)]">模型输出</summary>
              <pre class="mt-2 max-h-64 overflow-auto whitespace-pre-wrap break-words rounded border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-xs text-[var(--text-primary)]">{{ titleTranslationPreview.preview.rawOutput || "-" }}</pre>
            </details>
          </div>
        </div>
      </div>
    </GlassCard>

    <GlassCard v-if="section === 'automation' || section === 'review'" size="md" radius="lg">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            {{ section === "automation" ? "AI 自动标签" : "标签审核" }}
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            {{ section === "automation" ? "标签生成会等待内容分析收集到翻译、OCR 和插件元数据后执行。" : "查看模型生成的标签建议、证据和应用结果，再决定是否采纳。" }}
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <GlassButton
            title="刷新待审核建议"
            :disabled="loadingTagSuggestions"
            :loading="loadingTagSuggestions"
            variant="secondary"
            size="sm"
            @click="emit('refresh-tag-suggestions')"
          >
            <template #icon>
              <ArrowPathIcon class="h-4 w-4" />
            </template>
          </GlassButton>
          <GlassButton
            v-if="section === 'automation'"
            :disabled="
              aiLoading ||
              !aiSettings.features.autoTagging.enabled ||
              backfillingTagging
            "
            :loading="backfillingTagging"
            loading-text="加入队列中..."
            variant="secondary"
            size="sm"
            @click="emit('backfill-auto-tagging')"
          >
            {{ aiDirty ? "保存并加入队列" : "批量分析并打标签" }}
          </GlassButton>
          <GlassButton
            v-if="section === 'automation'"
            :disabled="aiLoading || backfillingTagLocalizations"
            :loading="backfillingTagLocalizations"
            loading-text="加入队列中..."
            variant="secondary"
            size="sm"
            @click="emit('backfill-tag-localizations')"
          >
            批量生成标签中文名
          </GlassButton>
        </div>
      </div>

      <div v-if="section === 'automation'" class="space-y-4">
        <label
          class="flex items-start gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.autoTagging.enabled"
            type="checkbox"
            class="mt-0.5 rounded"
          />
          <span>
            <span class="block">启用 AI 自动标签</span>
            <span class="mt-1 block text-xs text-[var(--text-secondary)]">
              有视觉模型时可直接使用页面图像，翻译、OCR 和元数据未完成不会阻塞；仅文本模型会等待 OCR 完成。
            </span>
          </span>
        </label>

        <label
          class="flex items-start gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.autoTagging.autoProcessNewArchives"
            type="checkbox"
            class="mt-0.5 rounded"
            :disabled="!aiSettings.features.autoTagging.enabled"
          />
          <span>
            <span class="block">新入库时生成标签</span>
            <span class="mt-1 block text-xs text-[var(--text-secondary)]">
              关闭后只在手动批量分析或单本重新分析时生成标签建议。
            </span>
          </span>
        </label>

        <div>
          <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">
            应用方式
          </div>
          <div
            role="radiogroup"
            aria-label="AI 自动标签应用方式"
            class="inline-flex overflow-hidden rounded-lg border border-[var(--border)]"
          >
            <button
              type="button"
              role="radio"
              :aria-checked="
                aiSettings.features.autoTagging.mode === 'suggestions'
              "
              :disabled="!aiSettings.features.autoTagging.enabled"
              class="border-r border-[var(--border)] px-3 py-2 text-sm transition-colors disabled:cursor-not-allowed disabled:opacity-50"
              :class="
                aiSettings.features.autoTagging.mode === 'suggestions'
                  ? 'bg-[var(--accent)] text-white'
                  : 'bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
              "
              @click="aiSettings.features.autoTagging.mode = 'suggestions'"
            >
              生成建议
            </button>
            <button
              type="button"
              role="radio"
              :aria-checked="
                aiSettings.features.autoTagging.mode === 'autoApplyReliable'
              "
              :disabled="!aiSettings.features.autoTagging.enabled"
              class="px-3 py-2 text-sm transition-colors disabled:cursor-not-allowed disabled:opacity-50"
              :class="
                aiSettings.features.autoTagging.mode === 'autoApplyReliable'
                  ? 'bg-[var(--accent)] text-white'
                  : 'bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
              "
              @click="
                aiSettings.features.autoTagging.mode = 'autoApplyReliable'
              "
            >
              自动应用可靠结果
            </button>
          </div>
        </div>

        <p class="max-w-xl text-xs leading-5 text-[var(--text-secondary)]">
          自动应用以可验证证据为准，不再依赖模型自评分。带页码的视觉或 OCR 证据、可信元数据，或标题中的精确标签匹配都会自动写入；没有可验证证据的结果会保留为待补充证据，可在后续重新分析时再次判定。
        </p>
        <p class="max-w-xl text-xs leading-5 text-[var(--text-secondary)]">
          标签内部始终使用英文规范值；中文名称仅用于界面展示和搜索。新建的元数据与 AI 标签会自动补充中文名，存量标签可通过上方操作批量加入队列。
        </p>
      </div>

      <div v-if="section === 'review'" class="border-t border-[var(--border)] pt-4">
        <div class="mb-3 flex items-center justify-between gap-3">
          <h3 class="text-sm font-medium text-[var(--text-primary)]">
            近期标签判定
          </h3>
          <span class="text-xs text-[var(--text-secondary)]">
            {{ tagSuggestionsTotal }} 项
          </span>
        </div>

        <div
          v-if="loadingTagSuggestions && tagSuggestions.length === 0"
          class="py-4 text-sm text-[var(--text-secondary)]"
        >
          正在读取建议...
        </div>
        <p
          v-else-if="tagSuggestions.length === 0"
          class="py-2 text-sm text-[var(--text-secondary)]"
        >
          当前没有近期标签判定。
        </p>
        <div
          v-else
          ref="tagSuggestionsList"
          class="max-h-[30rem] space-y-2 overflow-y-auto overscroll-contain pr-1"
        >
          <div
            v-for="suggestion in tagSuggestions"
            :key="suggestion.id"
            class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3"
          >
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="min-w-0">
                <p
                  class="truncate text-sm font-medium text-[var(--text-primary)]"
                >
                  {{ suggestion.archiveTitle }}
                </p>
                <div
                  class="mt-1 flex flex-wrap items-center gap-2 text-xs text-[var(--text-secondary)]"
                >
                  <span
                    class="rounded border border-[var(--border)] px-1.5 py-0.5 text-[var(--text-primary)]"
                  >
                    {{ suggestion.namespace }}:{{ suggestion.name }}
                  </span>
                  <span
                    >置信度 {{ formatConfidence(suggestion.confidence) }}</span
                  >
                  <span
                    :class="
                      suggestion.applicationDecision.outcome === 'autoApplied'
                        ? 'text-green-600 dark:text-green-400'
                        : 'text-amber-600 dark:text-amber-400'
                    "
                  >
                    {{ decisionLabel(suggestion.applicationDecision.outcome) }}
                  </span>
                  <span>判定时间 {{ formatSuggestionDate(suggestion.createdAt) }}</span>
                </div>
                <p class="mt-2 text-xs text-[var(--text-secondary)]">
                  {{ decisionSummary(suggestion) }}
                </p>
              </div>

              <div
                v-if="
                  editingSuggestionId !== suggestion.id &&
                  suggestion.applicationDecision.outcome === 'retainedAsSuggestion'
                "
                class="flex shrink-0 items-center gap-1"
              >
                <GlassButton
                  title="采纳建议"
                  :disabled="reviewingTagSuggestionId !== null"
                  :loading="reviewingTagSuggestionId === suggestion.id"
                  variant="success"
                  size="xs"
                  @click="
                    emit('review-tag-suggestion', suggestion.id, {
                      action: 'approve',
                    })
                  "
                >
                  <template #icon><CheckIcon class="h-4 w-4" /></template>
                </GlassButton>
                <GlassButton
                  title="编辑后采纳"
                  :disabled="reviewingTagSuggestionId !== null"
                  variant="secondary"
                  size="xs"
                  @click="startEditing(suggestion)"
                >
                  <template #icon><PencilIcon class="h-4 w-4" /></template>
                </GlassButton>
                <GlassButton
                  title="拒绝建议"
                  :disabled="reviewingTagSuggestionId !== null"
                  variant="danger"
                  size="xs"
                  @click="
                    emit('review-tag-suggestion', suggestion.id, {
                      action: 'reject',
                    })
                  "
                >
                  <template #icon><XMarkIcon class="h-4 w-4" /></template>
                </GlassButton>
              </div>
              <span
                v-else-if="editingSuggestionId !== suggestion.id"
                class="shrink-0 text-xs text-[var(--text-secondary)]"
              >
                {{ decisionLabel(suggestion.applicationDecision.outcome) }}
              </span>
            </div>

            <div
              v-if="
                editingSuggestionId === suggestion.id &&
                suggestion.applicationDecision.outcome === 'retainedAsSuggestion'
              "
              class="mt-3 grid grid-cols-1 gap-2 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]"
            >
              <input
                v-model.trim="editedSuggestionName"
                type="text"
                aria-label="编辑标签名"
                class="min-w-0 rounded-md border border-[var(--border)] bg-[var(--bg-secondary)] px-2 py-1.5 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
              <input
                v-model.trim="editedSuggestionNamespace"
                type="text"
                aria-label="编辑标签命名空间"
                class="min-w-0 rounded-md border border-[var(--border)] bg-[var(--bg-secondary)] px-2 py-1.5 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
              <div class="flex gap-1">
                <GlassButton
                  title="保存并采纳"
                  :disabled="
                    reviewingTagSuggestionId !== null ||
                    !editedSuggestionName ||
                    !editedSuggestionNamespace
                  "
                  :loading="reviewingTagSuggestionId === suggestion.id"
                  variant="success"
                  size="xs"
                  @click="submitEdit(suggestion.id)"
                >
                  <template #icon><CheckIcon class="h-4 w-4" /></template>
                </GlassButton>
                <GlassButton
                  title="取消编辑"
                  :disabled="reviewingTagSuggestionId !== null"
                  variant="secondary"
                  size="xs"
                  @click="cancelEditing"
                >
                  <template #icon><XMarkIcon class="h-4 w-4" /></template>
                </GlassButton>
              </div>
            </div>
          </div>
        </div>

        <div
          v-if="tagSuggestionsTotal > 0"
          class="mt-3 flex items-center justify-between border-t border-[var(--border)] pt-3"
        >
          <span class="text-xs text-[var(--text-secondary)]">
            第 {{ tagSuggestionsPage }} / {{ tagSuggestionsPageCount }} 页
          </span>
          <div class="flex items-center gap-1">
            <GlassButton
              title="上一页"
              :disabled="loadingTagSuggestions || tagSuggestionsPage <= 1"
              variant="secondary"
              size="xs"
              @click="emit('change-tag-suggestions-page', tagSuggestionsPage - 1)"
            >
              <template #icon><ChevronLeftIcon class="h-4 w-4" /></template>
            </GlassButton>
            <GlassButton
              title="下一页"
              :disabled="
                loadingTagSuggestions ||
                tagSuggestionsPage >= tagSuggestionsPageCount
              "
              variant="secondary"
              size="xs"
              @click="emit('change-tag-suggestions-page', tagSuggestionsPage + 1)"
            >
              <template #icon><ChevronRightIcon class="h-4 w-4" /></template>
            </GlassButton>
          </div>
        </div>

        <div
          v-if="recentTaggingRunIds.length > 0"
          class="mt-4 flex flex-wrap items-center gap-2 border-t border-[var(--border)] pt-4"
        >
          <span class="text-xs text-[var(--text-secondary)]"
            >本次会话中已审核的批次：</span
          >
          <GlassButton
            v-for="runId in recentTaggingRunIds"
            :key="runId"
            :title="`撤销批次 ${runId}`"
            :disabled="undoingTaggingRunId !== null"
            :loading="undoingTaggingRunId === runId"
            loading-text="撤销中..."
            variant="secondary"
            size="xs"
            @click="emit('undo-tagging-run', runId)"
          >
            <template #icon
              ><ArrowUturnLeftIcon class="mr-1 h-4 w-4"
            /></template>
            撤销已应用标签
          </GlassButton>
        </div>
      </div>
    </GlassCard>

    <GlassCard v-if="aiStatus && (section === 'overview' || section === 'review')" size="md" radius="lg">
      <h2 class="mb-3 text-lg font-medium text-[var(--text-primary)]">
        运行状态
      </h2>
      <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-[var(--accent)]">
            {{ aiStatus.queueSize }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">队列中</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-green-500">
            {{ aiStatus.processingCount }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">处理中</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-[var(--accent)]">
            {{ aiStatus.completedToday }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">今日完成</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-red-500">
            {{ aiStatus.failedToday }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">今日失败</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-[var(--accent)]">
            {{ aiStatus.languageDetectionPending }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">待语言确认</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-amber-500">
            {{ aiStatus.retryScheduled }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">等待重试</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-red-500">
            {{ aiStatus.unresolvedFailureCount }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">待处理失败</div>
        </div>
      </div>
      <section class="mt-6 border-t border-[var(--border)] pt-5">
        <h3 class="text-sm font-medium text-[var(--text-primary)]">
          执行器状态
        </h3>
        <div class="mt-3 divide-y divide-[var(--border)] border-y border-[var(--border)]">
          <div
            v-for="executorLane in aiStatus.executorLanes"
            :key="executorLane.executorLane"
            class="flex flex-wrap items-center justify-between gap-3 py-3 text-sm"
          >
            <p class="font-medium text-[var(--text-primary)]">
              {{ executorLaneLabel(executorLane.executorLane) }}
            </p>
            <p class="text-xs text-[var(--text-secondary)]">
              待处理 {{ executorLane.pendingCount }} · 处理中 {{ executorLane.processingCount }} · 并发上限 {{ executorLane.maxConcurrentJobs }}
            </p>
          </div>
        </div>
      </section>
      <section class="mt-6 border-t border-[var(--border)] pt-5">
        <h3 class="text-sm font-medium text-[var(--text-primary)]">模型状态</h3>
        <div class="mt-3 divide-y divide-[var(--border)] border-y border-[var(--border)]">
          <div
            v-for="modelState in aiStatus.modelStates"
            :key="modelState.profileId"
            class="flex flex-wrap items-center justify-between gap-3 py-3 text-sm"
          >
            <div class="min-w-0">
              <p class="truncate font-medium text-[var(--text-primary)]">
                {{ modelState.profileName || "未命名配置" }}
              </p>
              <p class="truncate text-xs text-[var(--text-secondary)]">
                {{ modelState.model }}
              </p>
              <p
                v-if="modelState.blockedUntil"
                class="mt-1 text-xs text-amber-600 dark:text-amber-400"
              >
                {{ formatStatusDate(modelState.blockedUntil) }} 后重试
              </p>
              <p
                v-if="modelState.lastError"
                :title="modelState.lastError"
                class="mt-1 truncate text-xs text-[var(--text-secondary)]"
              >
                {{ modelState.lastError }}
              </p>
            </div>
            <div class="flex shrink-0 items-center gap-2">
              <GlassButton
                v-if="modelState.state === 'rate_limited' || modelState.state === 'unavailable'"
                :disabled="Boolean(controllingModel)"
                :loading="controllingModel === modelState.profileId"
                loading-text="处理中..."
                variant="secondary"
                size="sm"
                @click="emit('force-continue-model', modelState.profileId)"
              >
                强制继续
              </GlassButton>
              <span
                class="rounded-md border px-2 py-1 text-xs"
                :class="modelStateClass(modelState.state)"
              >
                {{ modelStateLabel(modelState.state) }}
              </span>
            </div>
            <p
              v-if="modelState.forceAttemptsRemaining > 0"
              class="basis-full text-xs text-amber-600 dark:text-amber-400"
            >
              强制试运行剩余 {{ modelState.forceAttemptsRemaining }} 次
            </p>
          </div>
        </div>
      </section>

      <section class="mt-6 border-t border-[var(--border)] pt-5">
        <h3 class="text-sm font-medium text-[var(--text-primary)]">任务队列</h3>
        <div class="mt-3 divide-y divide-[var(--border)] border-y border-[var(--border)]">
          <div
            v-for="taskQueue in aiStatus.taskQueues"
            :key="taskQueue.jobType"
            class="flex flex-wrap items-center justify-between gap-3 py-3"
          >
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
                <p class="text-sm font-medium text-[var(--text-primary)]">
                  {{ taskQueueLabel(taskQueue.jobType) }}
                </p>
                <span
                  class="rounded-md border px-2 py-0.5 text-xs"
                  :class="taskQueueClass(taskQueue.state)"
                >
                  {{ taskQueueStateLabel(taskQueue.state) }}
                </span>
              </div>
              <p class="mt-1 text-xs text-[var(--text-secondary)]">
                待处理 {{ taskQueue.pendingCount }} · 处理中 {{ taskQueue.processingCount }}
                <template v-if="taskQueue.waitingForModelCount > 0">
                  · 等待模型 {{ taskQueue.waitingForModelCount }}
                </template>
              </p>
              <p
                v-if="taskQueue.state === 'waiting_for_model' && taskQueue.blockedUntil"
                class="mt-1 text-xs text-amber-600 dark:text-amber-400"
              >
                {{ formatStatusDate(taskQueue.blockedUntil) }} 后自动继续
              </p>
            </div>
            <GlassButton
              v-if="taskQueue.state === 'manually_paused'"
              :disabled="Boolean(controllingTaskQueue)"
              :loading="controllingTaskQueue === taskQueue.jobType"
              loading-text="处理中..."
              variant="secondary"
              size="sm"
              @click="emit('control-task-queue', taskQueue.jobType, 'resume')"
            >
              继续
            </GlassButton>
            <GlassButton
              v-else-if="taskQueue.state === 'waiting_for_model'"
              :disabled="Boolean(controllingTaskQueue)"
              :loading="controllingTaskQueue === taskQueue.jobType"
              loading-text="处理中..."
              variant="secondary"
              size="sm"
              @click="emit('control-task-queue', taskQueue.jobType, 'forceContinue')"
            >
              强制继续
            </GlassButton>
            <GlassButton
              v-else
              :disabled="Boolean(controllingTaskQueue)"
              :loading="controllingTaskQueue === taskQueue.jobType"
              loading-text="处理中..."
              variant="secondary"
              size="sm"
              @click="emit('control-task-queue', taskQueue.jobType, 'pause')"
            >
              暂停
            </GlassButton>
          </div>
        </div>
      </section>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  ArrowPathIcon,
  ArrowUturnLeftIcon,
  ChevronDownIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  ChevronUpIcon,
  CheckIcon,
  PencilIcon,
  XMarkIcon,
} from "@heroicons/vue/24/outline";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import SettingsSaveBar from "@/components/settings/SettingsSaveBar.vue";
import type {
  AIConnectionProfile,
  AISettings,
  AIStatus,
  AITitleTranslationPreviewResponse,
  PendingAITagSuggestion,
  ReviewAITagSuggestionRequest,
} from "@/types/api";

export type AISettingsSection =
  | "overview"
  | "models"
  | "automation"
  | "review"
  | "runtime";

interface Props {
  section: AISettingsSection;
  aiSettings: AISettings;
  aiStatus?: AIStatus;
  aiLoading: boolean;
  aiDirty: boolean;
  savedMessage: string | null;
  testingConnection: boolean;
  previewingTitleTranslation: boolean;
  titleTranslationPreview: AITitleTranslationPreviewResponse | null;
  backfillingTranslations: boolean;
  repairingTranslations: boolean;
  retranslatingTranslations: boolean;
  backfillingTagging: boolean;
  backfillingTagLocalizations: boolean;
  loadingTagSuggestions: boolean;
  tagSuggestions: readonly PendingAITagSuggestion[];
  tagSuggestionsTotal: number;
  tagSuggestionsPage: number;
  tagSuggestionsPageCount: number;
  reviewingTagSuggestionId: string | null;
  undoingTaggingRunId: string | null;
  recentTaggingRunIds: readonly string[];
  controllingTaskQueue: string | null;
  controllingModel: string | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  save: [];
  discard: [];
  "test-connection": [];
  "preview-title-translation": [title: string];
  "backfill-title-translations": [];
  "repair-suspicious-title-translations": [];
  "force-retranslate-title-translations": [];
  "backfill-auto-tagging": [];
  "backfill-tag-localizations": [];
  "refresh-tag-suggestions": [];
  "change-tag-suggestions-page": [page: number];
  "review-tag-suggestion": [
    suggestionId: string,
    payload: ReviewAITagSuggestionRequest,
  ];
  "undo-tagging-run": [runId: string];
  "control-task-queue": [
    jobType: string,
    action: "pause" | "resume" | "forceContinue",
  ];
  "force-continue-model": [profileId: string];
  "update-execution-lane": [
    lane: "llm" | "ocr" | "plugin" | "orchestration",
    limit: number,
  ];
}>();

const editingSuggestionId = ref<string | null>(null);
const previewTitle = ref("");
const editedSuggestionName = ref("");
const editedSuggestionNamespace = ref("");
const tagSuggestionsList = ref<HTMLElement | null>(null);

watch(
  () => props.tagSuggestionsPage,
  () => tagSuggestionsList.value?.scrollTo({ top: 0 }),
);

type ExecutorLane = "llm" | "ocr" | "plugin" | "orchestration";

const executorLaneConcurrency = (lane: ExecutorLane) =>
  computed({
    get: () => props.aiSettings.execution.lanes[lane],
    set: (limit: number) => emit("update-execution-lane", lane, limit),
  });

const llmConcurrency = executorLaneConcurrency("llm");
const ocrConcurrency = executorLaneConcurrency("ocr");
const pluginConcurrency = executorLaneConcurrency("plugin");
const orchestrationConcurrency = executorLaneConcurrency("orchestration");

const formattedTitleTranslationPreviewRequest = computed(() => {
  const request = props.titleTranslationPreview?.preview?.request;
  return request ? JSON.stringify(request, null, 2) : "";
});

const clampProfileFirstTokenTimeouts = () => {
  const overallTimeout = Math.max(1, props.aiSettings.execution.timeoutSeconds);
  for (const profile of props.aiSettings.profiles) {
    const configured = profile.connection.firstTokenTimeoutSeconds;
    profile.connection.firstTokenTimeoutSeconds = Math.min(
      overallTimeout,
      Math.max(1, Number.isFinite(configured) ? configured : 30),
    );
  }
};

const titleTranslationLanguages = [
  { code: "zh-CN", label: "简体中文（zh-CN）" },
  { code: "zh-TW", label: "繁体中文（zh-TW）" },
  { code: "ja", label: "日语（ja）" },
  { code: "ko", label: "韩语（ko）" },
  { code: "en", label: "英语（en）" },
  { code: "fr", label: "法语（fr）" },
  { code: "de", label: "德语（de）" },
  { code: "es", label: "西班牙语（es）" },
  { code: "pt", label: "葡萄牙语（pt）" },
  { code: "it", label: "意大利语（it）" },
  { code: "ru", label: "俄语（ru）" },
  { code: "uk", label: "乌克兰语（uk）" },
] as const;

const isKnownTargetLanguage = (language: string) =>
  titleTranslationLanguages.some((option) => option.code === language);

const activeProfile = computed(() =>
  props.aiSettings.profiles.find(
    (profile) => profile.id === props.aiSettings.activeProfileId,
  ),
);

const activeProfileIndex = computed(() =>
  props.aiSettings.profiles.findIndex(
    (profile) => profile.id === props.aiSettings.activeProfileId,
  ),
);

const moveActiveProfile = (direction: -1 | 1) => {
  const index = activeProfileIndex.value;
  const nextIndex = index + direction;
  if (index < 0 || nextIndex < 0 || nextIndex >= props.aiSettings.profiles.length) {
    return;
  }
  const profiles = props.aiSettings.profiles;
  [profiles[index], profiles[nextIndex]] = [profiles[nextIndex], profiles[index]];
};

const newProfileId = () =>
  globalThis.crypto?.randomUUID?.() ??
  `profile-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const addProfile = () => {
  const profile: AIConnectionProfile = {
    id: newProfileId(),
    name: "新 AI 配置",
    enabled: true,
    connection: {
      provider: "openaiCompatible",
      baseUrl: "http://localhost:11434/v1",
      model: "",
      streamResponse: false,
      firstTokenTimeoutSeconds: 30,
      requestIntervalSeconds: 0,
      ollamaUseGpu: false,
      ollamaAutoNumCtx: false,
      ollamaMaxNumCtx: 16_384,
      ollamaThinking: false,
      visionCapable: false,
      authMode: "none",
      apiKeyConfigured: false,
    },
  };
  props.aiSettings.profiles.push(profile);
  props.aiSettings.activeProfileId = profile.id;
};

const baseUrlPlaceholder = computed(() =>
  activeProfile.value?.connection.provider === "ollama"
    ? "http://localhost:11434"
    : "https://api.openai.com/v1 或 http://localhost:11434/v1",
);

const applyProviderDefaults = () => {
  const connection = activeProfile.value?.connection;
  if (!connection) return;

  if (connection.provider === "ollama") {
    if (
      !connection.baseUrl ||
      connection.baseUrl === "https://api.openai.com/v1" ||
      connection.baseUrl === "http://localhost:11434/v1"
    ) {
      connection.baseUrl = "http://localhost:11434";
    }
    if (
      connection.authMode === "bearer" &&
      !connection.apiKeyConfigured &&
      !connection.apiKey?.trim()
    ) {
      connection.authMode = "none";
    }
    return;
  }

  if (connection.baseUrl === "http://localhost:11434") {
    connection.baseUrl = "http://localhost:11434/v1";
  }
};

const removeActiveProfile = () => {
  const profile = activeProfile.value;
  if (!profile || props.aiSettings.profiles.length <= 1) return;
  const index = props.aiSettings.profiles.findIndex(
    (item) => item.id === profile.id,
  );
  props.aiSettings.profiles.splice(index, 1);
  props.aiSettings.activeProfileId = props.aiSettings.profiles[0].id;
};

const canToggleActiveProfile = computed(
  () =>
    activeProfile.value?.enabled === false ||
    props.aiSettings.profiles.filter((profile) => profile.enabled).length > 1,
);

const switchFromDisabledActiveProfile = () => {
  const profile = activeProfile.value;
  if (!profile || profile.enabled) return;
  const nextProfile = props.aiSettings.profiles.find(
    (item) => item.enabled && item.id !== profile.id,
  );
  if (nextProfile) props.aiSettings.activeProfileId = nextProfile.id;
};

const canTestConnection = computed(() => {
  const connection = activeProfile.value?.connection;
  if (!connection) return false;
  const { baseUrl, model, apiKey, apiKeyConfigured, authMode } = connection;
  return Boolean(
    baseUrl.trim() &&
    model.trim() &&
    (authMode === "none" || apiKey?.trim() || apiKeyConfigured),
  );
});

const apiKeyPlaceholder = computed(() =>
  activeProfile.value?.connection.apiKeyConfigured
    ? "已配置。留空时保留现有密钥"
    : "输入 API Key",
);

const apiKeyHint = computed(() =>
  activeProfile.value?.connection.apiKeyConfigured
    ? "密钥已配置，保存时留空将继续使用现有密钥。"
    : "密钥仅在保存或测试连接时发送，不会在此页面回显。",
);

const formatStatusDate = (value: string) => new Date(value).toLocaleString();

const modelStateLabel = (state: string) => {
  const labels: Record<string, string> = {
    available: "可用",
    rate_limited: "限流中",
    unavailable: "不可用",
    force_retrying: "试运行中",
    disabled: "已停用",
  };
  return labels[state] ?? "未知";
};

const modelStateClass = (state: string) => {
  if (state === "available") {
    return "border-green-500/40 bg-green-500/10 text-green-700 dark:text-green-300";
  }
  if (
    state === "rate_limited" ||
    state === "unavailable" ||
    state === "force_retrying"
  ) {
    return "border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300";
  }
  return "border-[var(--border)] bg-[var(--bg-tertiary)] text-[var(--text-secondary)]";
};

const executorLaneLabels: Record<string, string> = {
  llm: "模型请求",
  ocr: "OCR",
  plugin: "插件",
  orchestration: "工作流编排",
};

const executorLaneLabel = (executorLane: string) =>
  executorLaneLabels[executorLane] ?? executorLane;

const taskQueueLabels: Record<string, string> = {
  title_translation: "标题翻译",
  title_language_detection: "标题语言识别",
  tag_localization: "标签中文翻译",
  content_analysis_reconcile: "内容分析协调",
  content_analysis_synthesize: "内容分析",
  ocr_extract: "OCR 提取",
  metadata_extract: "元数据提取",
  auto_tagging: "自动标签",
};

const taskQueueLabel = (jobType: string) => taskQueueLabels[jobType] ?? jobType;

const taskQueueStateLabel = (state: string) => {
  const labels: Record<string, string> = {
    running: "运行中",
    manually_paused: "已暂停",
    waiting_for_model: "等待模型",
    idle: "空闲",
  };
  return labels[state] ?? "未知";
};

const taskQueueClass = (state: string) => {
  if (state === "running") {
    return "border-green-500/40 bg-green-500/10 text-green-700 dark:text-green-300";
  }
  if (state === "waiting_for_model") {
    return "border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300";
  }
  return "border-[var(--border)] bg-[var(--bg-tertiary)] text-[var(--text-secondary)]";
};

const formatConfidence = (confidence: number) =>
  `${Math.round(Math.max(0, Math.min(1, confidence)) * 100)}%`;

const formatSuggestionDate = (value: string) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "未知时间";
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
};

const evidenceSummary = (evidence: unknown) => {
  if (Array.isArray(evidence)) {
    const text = evidence
      .map((item) => {
        if (typeof item === "string") return item.trim();
        if (!item || typeof item !== "object") return "";
        const value = item as Record<string, unknown>;
        const source = typeof value.source === "string" ? value.source : "证据";
        const page = typeof value.page === "number" ? `第 ${value.page} 页` : "";
        const detail =
          typeof value.excerpt === "string"
            ? value.excerpt
            : typeof value.reason === "string"
              ? value.reason
              : "";
        return [source, page, detail].filter(Boolean).join("：");
      })
      .filter(Boolean)
      .slice(0, 2)
      .join("；");
    if (text) return text;
    return evidence.length > 0
      ? `已保存 ${evidence.length} 条分析证据。`
      : "无额外证据。";
  }
  if (typeof evidence === "string" && evidence.trim()) return evidence.trim();
  return "已保存用于生成该建议的分析证据。";
};

const decisionLabel = (outcome: string) => {
  switch (outcome) {
    case "autoApplied":
      return "已自动应用";
    case "retainedAsSuggestion":
      return "保留为建议";
    case "waitingForEvidence":
      return "等待证据";
    default:
      return "未应用";
  }
};

const decisionSummary = (suggestion: PendingAITagSuggestion) => {
  const evidence = evidenceSummary(suggestion.evidence);
  switch (suggestion.applicationDecision.reason) {
    case "verifiedEvidence":
      return `已自动应用，可验证证据：${evidence}`;
    case "missingVerifiedEvidence":
      return "未自动应用：模型返回的证据无法对应到本次输入，可在后续重新分析时再次判定。";
    case "automaticApplicationDisabled":
      return `当前为仅生成建议模式，可验证证据：${evidence}`;
    case "rejected":
      return "该建议已被拒绝。";
    case "undone":
      return "该标签的应用已撤销。";
    default:
      return evidence;
  }
};

const startEditing = (suggestion: PendingAITagSuggestion) => {
  editingSuggestionId.value = suggestion.id;
  editedSuggestionName.value = suggestion.name;
  editedSuggestionNamespace.value = suggestion.namespace;
};

const cancelEditing = () => {
  editingSuggestionId.value = null;
  editedSuggestionName.value = "";
  editedSuggestionNamespace.value = "";
};

const submitEdit = (suggestionId: string) => {
  const editedName = editedSuggestionName.value.trim();
  const editedNamespace = editedSuggestionNamespace.value.trim();
  if (!editedName || !editedNamespace) return;
  emit("review-tag-suggestion", suggestionId, {
    action: "edit",
    editedName,
    editedNamespace,
  });
  cancelEditing();
};
</script>
