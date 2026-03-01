<template>
  <div class="min-h-screen w-full flex items-center justify-center bg-[#0f0f1f] py-12 px-4">
    <!-- 登录卡片 -->
    <div class="w-full max-w-sm bg-[#1b1b2f] border border-[#2d2d44] rounded-lg shadow-xl p-8">
      <!-- Logo + 标题 -->
      <div class="text-center mb-8">
        <div class="flex justify-center mb-4">
          <img src="/icon.png" alt="OtamoryX" class="w-16 h-16 rounded-xl" onerror="this.style.display='none'" />
        </div>
        <h1 class="text-2xl font-bold text-[#e0e0e0] mb-1">OtamoryX</h1>
        <p class="text-sm text-[#808090]">{{ isInitializing ? "初始化系统" : "登录到漫画图书馆" }}</p>
      </div>

      <!-- 系统状态检查 -->
      <div v-if="systemStatusLoading" class="text-center py-8">
        <div class="inline-flex items-center space-x-3">
          <div class="w-5 h-5 border-2 border-[#3d3d5c] border-t-[#7b68ee] rounded-full animate-spin" />
          <span class="text-[#a0a0b0] text-sm">检查系统状态...</span>
        </div>
      </div>

      <!-- 初始化表单 -->
      <form v-else-if="isInitializing" class="space-y-4" @submit.prevent="handleInitialize">
        <div>
          <label class="block text-sm text-[#a0a0b0] mb-1.5">管理员用户名 <span class="text-red-400">*</span></label>
          <input v-model="initForm.username" type="text" required placeholder="设置用户名"
            class="w-full px-3 py-2.5 bg-[#12122a] border border-[#2d2d44] rounded text-[#e0e0e0] placeholder-[#505065]
                   focus:outline-none focus:border-[#7b68ee] focus:ring-1 focus:ring-[#7b68ee]/30 transition-colors" />
        </div>
        <div>
          <label class="block text-sm text-[#a0a0b0] mb-1.5">邮箱 <span class="text-[#505065] text-xs">（可选）</span></label>
          <input v-model="initForm.email" type="email" placeholder="邮箱地址"
            class="w-full px-3 py-2.5 bg-[#12122a] border border-[#2d2d44] rounded text-[#e0e0e0] placeholder-[#505065]
                   focus:outline-none focus:border-[#7b68ee] focus:ring-1 focus:ring-[#7b68ee]/30 transition-colors" />
        </div>
        <div>
          <label class="block text-sm text-[#a0a0b0] mb-1.5">密码 <span class="text-red-400">*</span></label>
          <input v-model="initForm.password" type="password" required placeholder="设置密码"
            class="w-full px-3 py-2.5 bg-[#12122a] border border-[#2d2d44] rounded text-[#e0e0e0] placeholder-[#505065]
                   focus:outline-none focus:border-[#7b68ee] focus:ring-1 focus:ring-[#7b68ee]/30 transition-colors" />
        </div>

        <button type="submit" :disabled="initLoading"
          class="w-full mt-2 py-2.5 bg-[#7b68ee] hover:bg-[#6a5acd] text-white font-medium rounded transition-colors disabled:opacity-50">
          {{ initLoading ? "初始化中..." : "初始化系统" }}
        </button>
      </form>

      <!-- 登录表单 -->
      <form v-else class="space-y-4" @submit.prevent="handleLogin">
        <div>
          <label class="block text-sm text-[#a0a0b0] mb-1.5">用户名</label>
          <input v-model="loginForm.username" type="text" required placeholder="请输入用户名"
            class="w-full px-3 py-2.5 bg-[#12122a] border border-[#2d2d44] rounded text-[#e0e0e0] placeholder-[#505065]
                   focus:outline-none focus:border-[#7b68ee] focus:ring-1 focus:ring-[#7b68ee]/30 transition-colors" />
        </div>
        <div>
          <label class="block text-sm text-[#a0a0b0] mb-1.5">密码</label>
          <input v-model="loginForm.password" type="password" required placeholder="请输入密码"
            class="w-full px-3 py-2.5 bg-[#12122a] border border-[#2d2d44] rounded text-[#e0e0e0] placeholder-[#505065]
                   focus:outline-none focus:border-[#7b68ee] focus:ring-1 focus:ring-[#7b68ee]/30 transition-colors" />
        </div>

        <button type="submit" :disabled="loginLoading"
          class="w-full mt-2 py-2.5 bg-[#7b68ee] hover:bg-[#6a5acd] text-white font-medium rounded transition-colors disabled:opacity-50">
          {{ loginLoading ? "登录中..." : "登录" }}
        </button>
      </form>

      <!-- 错误提示 -->
      <div v-if="error" class="mt-4 px-3 py-2.5 bg-red-500/10 border border-red-500/30 rounded text-red-400 text-sm flex items-center gap-2">
        <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        {{ error }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import { getSystemStatus, initializeSystem, login } from "@/utils/api";

const router = useRouter();
const authStore = useAuthStore();

const systemStatusLoading = ref(true);
const isInitializing = ref(false);
const initLoading = ref(false);
const loginLoading = ref(false);
const error = ref("");

const initForm = ref({ username: "", email: "", password: "" });
const loginForm = ref({ username: "", password: "" });

const checkSystemStatus = async () => {
  try {
    const status = await getSystemStatus();
    isInitializing.value = !status.initialized;
  } catch (err) {
    error.value = "无法连接到服务器";
  } finally {
    systemStatusLoading.value = false;
  }
};

const handleInitialize = async () => {
  if (!initForm.value.username || !initForm.value.password) { error.value = "请填写用户名和密码"; return; }
  initLoading.value = true; error.value = "";
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
  if (!loginForm.value.username || !loginForm.value.password) { error.value = "请填写用户名和密码"; return; }
  loginLoading.value = true; error.value = "";
  try {
    const response = await login(loginForm.value);
    await authStore.login(response.token, response.user);
    router.push("/library");
  } catch (err: any) {
    error.value = err.response?.data?.message || "用户名或密码错误";
  } finally {
    loginLoading.value = false;
  }
};

onMounted(() => {
  if (authStore.isAuthenticated) { router.push("/library"); return; }
  checkSystemStatus();
});
</script>
