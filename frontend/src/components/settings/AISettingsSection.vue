<template>
  <div class="space-y-6">
    <SettingsSaveBar
      :dirty="aiDirty"
      :saving="aiLoading"
      :saved-message="savedMessage"
      @save="emit('save')"
      @discard="emit('discard')"
    />

    <GlassCard size="md" radius="lg">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">AI 连接</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            标题翻译会保留入队时的配置；内容分析会在执行时使用当前启用的配置。
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
            profile.enabled ? "已启用" : "已停用"
          }}</span>
        </button>
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
          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >服务协议</label
            >
            <div
              class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)]"
            >
              OpenAI-compatible Chat Completions
            </div>
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
            placeholder="https://api.openai.com/v1 或 http://localhost:11434/v1"
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

    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">内容分析</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            每本新漫画入库后会在后台抽取封面和 8 到 20 张正文页，识别题材与内容特征，为随机精选和偏好规则提供依据。
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

    <GlassCard size="md" radius="lg">
      <h2 class="mb-4 text-lg font-medium text-[var(--text-primary)]">
        执行参数
      </h2>
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >最大并发任务数</label
          >
          <input
            v-model.number="aiSettings.execution.maxConcurrentTasks"
            type="number"
            min="1"
            max="10"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >请求超时（秒）</label
          >
          <input
            v-model.number="aiSettings.execution.timeoutSeconds"
            type="number"
            min="10"
            max="1800"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

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
    </GlassCard>

    <GlassCard size="md" radius="lg">
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
      </div>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            AI 自动标签
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            标签生成会等待内容分析收集到翻译、OCR 和插件元数据后执行。
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
        </div>
      </div>

      <div class="space-y-4">
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
            <span class="block">新入库漫画自动加入工作流</span>
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

        <div class="max-w-sm">
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
          >
            自动应用可靠性阈值
          </label>
          <input
            v-model.number="aiSettings.features.autoTagging.autoApplyThreshold"
            type="number"
            min="0"
            max="1"
            step="0.05"
            :disabled="
              !aiSettings.features.autoTagging.enabled ||
              aiSettings.features.autoTagging.mode !== 'autoApplyReliable'
            "
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-60"
          />
          <p class="mt-1 text-xs text-[var(--text-secondary)]">
            模型置信度达到该值且包含可追溯证据的标签会自动写入；其余结果保留为待审核建议。
          </p>
        </div>
      </div>

      <div class="mt-6 border-t border-[var(--border)] pt-4">
        <div class="mb-3 flex items-center justify-between gap-3">
          <h3 class="text-sm font-medium text-[var(--text-primary)]">
            待审核建议
          </h3>
          <span class="text-xs text-[var(--text-secondary)]">
            {{ pendingTagSuggestions.length }} 项
          </span>
        </div>

        <div
          v-if="loadingTagSuggestions && pendingTagSuggestions.length === 0"
          class="py-4 text-sm text-[var(--text-secondary)]"
        >
          正在读取建议...
        </div>
        <p
          v-else-if="pendingTagSuggestions.length === 0"
          class="py-2 text-sm text-[var(--text-secondary)]"
        >
          当前没有待审核的标签建议。
        </p>
        <div v-else class="space-y-2">
          <div
            v-for="suggestion in pendingTagSuggestions"
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
                </div>
                <p class="mt-2 text-xs text-[var(--text-secondary)]">
                  {{ evidenceSummary(suggestion.evidence) }}
                </p>
              </div>

              <div
                v-if="editingSuggestionId !== suggestion.id"
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
            </div>

            <div
              v-if="editingSuggestionId === suggestion.id"
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

    <GlassCard v-if="aiStatus" size="md" radius="lg">
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
      <p
        v-if="aiStatus.providerBlockedUntil"
        class="mt-3 text-sm text-amber-600 dark:text-amber-400"
      >
        AI 服务限流中，{{
          new Date(aiStatus.providerBlockedUntil).toLocaleString()
        }}
        后自动恢复。
      </p>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import {
  ArrowPathIcon,
  ArrowUturnLeftIcon,
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
  PendingAITagSuggestion,
  ReviewAITagSuggestionRequest,
} from "@/types/api";

interface Props {
  aiSettings: AISettings;
  aiStatus?: AIStatus;
  aiLoading: boolean;
  aiDirty: boolean;
  savedMessage: string | null;
  testingConnection: boolean;
  backfillingTranslations: boolean;
  repairingTranslations: boolean;
  retranslatingTranslations: boolean;
  backfillingTagging: boolean;
  loadingTagSuggestions: boolean;
  pendingTagSuggestions: readonly PendingAITagSuggestion[];
  reviewingTagSuggestionId: string | null;
  undoingTaggingRunId: string | null;
  recentTaggingRunIds: readonly string[];
}

const props = defineProps<Props>();

const emit = defineEmits<{
  save: [];
  discard: [];
  "test-connection": [];
  "backfill-title-translations": [];
  "repair-suspicious-title-translations": [];
  "force-retranslate-title-translations": [];
  "backfill-auto-tagging": [];
  "refresh-tag-suggestions": [];
  "review-tag-suggestion": [
    suggestionId: string,
    payload: ReviewAITagSuggestionRequest,
  ];
  "undo-tagging-run": [runId: string];
}>();

const editingSuggestionId = ref<string | null>(null);
const editedSuggestionName = ref("");
const editedSuggestionNamespace = ref("");

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
      visionCapable: false,
      authMode: "none",
      apiKeyConfigured: false,
    },
  };
  props.aiSettings.profiles.push(profile);
  props.aiSettings.activeProfileId = profile.id;
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

const formatConfidence = (confidence: number) =>
  `${Math.round(Math.max(0, Math.min(1, confidence)) * 100)}%`;

const evidenceSummary = (evidence: unknown) => {
  if (Array.isArray(evidence)) {
    const text = evidence
      .filter((item): item is string => typeof item === "string")
      .map((item) => item.trim())
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
