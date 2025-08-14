<template>
  <!-- 背景层：渐变和几何图形 -->
  <div class="login-background min-h-screen w-full relative overflow-hidden">
    <!-- 动态几何背景 -->
    <div
      class="absolute inset-0 bg-linear-to-br from-blue-900 via-purple-900 to-indigo-900"
    >
      <!-- 浮动几何形状 -->
      <div class="floating-shapes">
        <div class="shape shape-1" />
        <div class="shape shape-2" />
        <div class="shape shape-3" />
        <div class="shape shape-4" />
        <div class="shape shape-5" />
      </div>
    </div>

    <!-- 内容容器 -->
    <div
      class="relative z-10 min-h-screen flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8"
    >
      <GlassCard
        size="lg"
        radius="2xl"
        glow-effect
        shine-effect
        class="w-full max-w-md"
      >
        <!-- 标题区域 -->
        <div class="text-center mb-8">
          <div class="flex justify-center mb-4">
            <div class="w-20 h-20 flex items-center justify-center">
              <img
                src="/icon.png"
                alt="OtamoryX Logo"
                class="w-20 h-20 rounded-2xl shadow-lg"
              >
            </div>
          </div>
          <h1 class="text-3xl font-bold text-white mb-2">欢迎使用 OtamoryX</h1>
          <p class="text-white/70">
            {{ isInitializing ? "系统初始化" : "登录到您的漫画图书馆" }}
          </p>
        </div>

        <!-- 系统状态检查 -->
        <div v-if="systemStatusLoading"
class="text-center py-8">
          <div class="inline-flex items-center space-x-3">
            <div
              class="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin"
            />
            <span class="text-white/80">检查系统状态...</span>
          </div>
        </div>

        <!-- 初始化表单 -->
        <form
          v-else-if="isInitializing"
          class="space-y-6"
          @submit.prevent="handleInitialize"
        >
          <div class="space-y-4">
            <GlassInput
              v-model="initForm.username"
              label="管理员用户名"
              placeholder="请输入管理员用户名"
              required
            >
              <template #prefix>
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
                  />
                </svg>
              </template>
            </GlassInput>

            <GlassInput
              v-model="initForm.email"
              label="邮箱地址"
              type="email"
              placeholder="邮箱地址（可选）"
              helper-text="用于接收系统通知"
            >
              <template #prefix>
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M16 12a4 4 0 10-8 0 4 4 0 008 0zm0 0v1.5a2.5 2.5 0 005 0V12a9 9 0 10-9 9m4.5-1.206a8.959 8.959 0 01-4.5 1.207"
                  />
                </svg>
              </template>
            </GlassInput>

            <GlassInput
              v-model="initForm.password"
              label="密码"
              type="password"
              placeholder="请设置管理员密码"
              required
            >
              <template #prefix>
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
                  />
                </svg>
              </template>
            </GlassInput>
          </div>

          <GlassButton
            type="submit"
            variant="primary"
            size="lg"
            :loading="initLoading"
            loading-text="初始化中..."
            full-width
            glow-effect
          >
            初始化系统
          </GlassButton>
        </form>

        <!-- 登录表单 -->
        <form v-else
@submit.prevent="handleLogin" class="space-y-6">
          <div class="space-y-4">
            <GlassInput
              v-model="loginForm.username"
              label="用户名"
              placeholder="请输入用户名"
              required
            >
              <template #prefix>
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
                  />
                </svg>
              </template>
            </GlassInput>

            <GlassInput
              v-model="loginForm.password"
              label="密码"
              type="password"
              placeholder="请输入密码"
              required
            >
              <template #prefix>
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
                  />
                </svg>
              </template>
            </GlassInput>
          </div>

          <GlassButton
            type="submit"
            variant="primary"
            size="lg"
            :loading="loginLoading"
            loading-text="登录中..."
            full-width
            glow-effect
          >
            登录
          </GlassButton>
        </form>

        <!-- 错误提示 -->
        <div v-if="error"
class="mt-6">
          <GlassCard size="sm"
class="bg-red-500/20 border-red-400/30">
            <div class="flex items-center space-x-2">
              <svg
                class="w-5 h-5 text-red-400 shrink-0"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <span class="text-red-200 text-sm">{{ error }}</span>
            </div>
          </GlassCard>
        </div>
      </GlassCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { getSystemStatus, initializeSystem, login } from "@/utils/api";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassInput from "@/components/base/GlassInput.vue";

const router = useRouter();
const authStore = useAuthStore();

const systemStatusLoading = ref(true);
const isInitializing = ref(false);
const initLoading = ref(false);
const loginLoading = ref(false);
const error = ref("");

const initForm = ref({
  username: "",
  email: "",
  password: "",
});

const loginForm = ref({
  username: "",
  password: "",
});

const checkSystemStatus = async () => {
  try {
    const status = await getSystemStatus();
    isInitializing.value = !status.initialized;
  } catch (err) {
    console.error("Failed to check system status:", err);
    error.value = "无法连接到服务器";
  } finally {
    systemStatusLoading.value = false;
  }
};

const handleInitialize = async () => {
  if (!initForm.value.username || !initForm.value.password) {
    error.value = "请填写用户名和密码";
    return;
  }

  initLoading.value = true;
  error.value = "";

  try {
    const response = await initializeSystem(initForm.value);
    await authStore.login(response.token, response.user);
    router.push("/library");
  } catch (err: any) {
    error.value = err.response?.data?.message || "初始化失败";
  } finally {
    initLoading.value = false;
  }
};

const handleLogin = async () => {
  if (!loginForm.value.username || !loginForm.value.password) {
    error.value = "请填写用户名和密码";
    return;
  }

  loginLoading.value = true;
  error.value = "";

  try {
    const response = await login(loginForm.value);
    await authStore.login(response.token, response.user);
    router.push("/library");
  } catch (err: any) {
    error.value = err.response?.data?.message || "登录失败";
  } finally {
    loginLoading.value = false;
  }
};

onMounted(() => {
  // 如果已经登录，直接跳转
  if (authStore.isAuthenticated) {
    router.push("/library");
    return;
  }

  checkSystemStatus();
});
</script>

<style scoped>
/* 确保登录页面占满全屏 */
.min-h-screen {
  min-height: 100vh;
  width: 100vw;
}

/* 背景动效 */
.login-background {
  background: linear-gradient(135deg, #1e3a8a 0%, #7c3aed 50%, #3730a3 100%);
  background-size: 400% 400%;
  animation: gradientShift 20s ease infinite;
}

@keyframes gradientShift {
  0% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0% 50%;
  }
}

/* 浮动几何形状 */
.floating-shapes {
  position: absolute;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.shape {
  position: absolute;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 50%;
  animation: float 20s infinite linear;
}

.shape-1 {
  width: 80px;
  height: 80px;
  top: 20%;
  left: 10%;
  animation-delay: 0s;
  animation-duration: 25s;
}

.shape-2 {
  width: 120px;
  height: 120px;
  top: 60%;
  right: 10%;
  animation-delay: -5s;
  animation-duration: 30s;
  border-radius: 30% 70% 70% 30% / 30% 30% 70% 70%;
}

.shape-3 {
  width: 60px;
  height: 60px;
  top: 80%;
  left: 20%;
  animation-delay: -10s;
  animation-duration: 20s;
  clip-path: polygon(50% 0%, 0% 100%, 100% 100%);
  border-radius: 0;
}

.shape-4 {
  width: 100px;
  height: 100px;
  top: 30%;
  right: 30%;
  animation-delay: -15s;
  animation-duration: 35s;
  border-radius: 0;
  transform: rotate(45deg);
}

.shape-5 {
  width: 70px;
  height: 70px;
  top: 10%;
  left: 60%;
  animation-delay: -20s;
  animation-duration: 28s;
  border-radius: 30% 70% 70% 30% / 30% 30% 70% 70%;
}

@keyframes float {
  from {
    transform: translateY(100vh) rotate(0deg);
  }
  to {
    transform: translateY(-100vh) rotate(360deg);
  }
}

/* 响应式调整 */
@media (max-width: 640px) {
  .shape {
    opacity: 0.5;
  }

  .shape-1,
  .shape-2,
  .shape-4 {
    width: 50px;
    height: 50px;
  }

  .shape-3,
  .shape-5 {
    width: 35px;
    height: 35px;
  }
}

/* 毛玻璃增强效果 */
.glass-card {
  box-shadow:
    0 25px 50px -12px rgba(0, 0, 0, 0.25),
    0 0 0 1px rgba(255, 255, 255, 0.1),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
}
</style>
