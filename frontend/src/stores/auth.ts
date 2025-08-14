import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { User } from "@/types/api";

export const useAuthStore = defineStore("auth", () => {
  const apiKey = ref<string>("");
  const user = ref<User | null>(null);
  const isAuthenticated = computed(() => !!apiKey.value);
  const isAdmin = computed(() => user.value?.role === "admin");

  const login = async (key: string, userData: User) => {
    apiKey.value = key;
    user.value = userData;
    localStorage.setItem("apiKey", key);
    localStorage.setItem("user", JSON.stringify(userData));
  };

  const logout = () => {
    apiKey.value = "";
    user.value = null;
    localStorage.removeItem("apiKey");
    localStorage.removeItem("user");
  };

  // 初始化时从localStorage恢复认证状态
  const initAuth = () => {
    const savedKey = localStorage.getItem("apiKey");
    const savedUser = localStorage.getItem("user");
    if (savedKey && savedUser) {
      apiKey.value = savedKey;
      try {
        user.value = JSON.parse(savedUser);
      } catch (e) {
        console.error("Failed to parse saved user data:", e);
        logout();
      }
    }
  };

  return {
    apiKey,
    user,
    isAuthenticated,
    isAdmin,
    login,
    logout,
    initAuth,
  };
});
