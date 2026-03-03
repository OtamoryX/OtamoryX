import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { User } from "@/types/api";

const normalizeRole = (role: unknown): "admin" | "user" => {
  if (typeof role === "string" && role.toLowerCase() === "admin") {
    return "admin";
  }
  return "user";
};

const normalizeUser = (userData: User): User => ({
  ...userData,
  role: normalizeRole((userData as unknown as { role?: unknown }).role),
});

export const useAuthStore = defineStore("auth", () => {
  const apiKey = ref<string>("");
  const user = ref<User | null>(null);
  const isAuthenticated = computed(() => !!apiKey.value);
  const isAdmin = computed(() => user.value?.role === "admin");

  const login = async (key: string, userData: User) => {
    const normalizedUser = normalizeUser(userData);
    apiKey.value = key;
    user.value = normalizedUser;
    localStorage.setItem("apiKey", key);
    localStorage.setItem("user", JSON.stringify(normalizedUser));
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
        const parsedUser = JSON.parse(savedUser) as User;
        user.value = normalizeUser(parsedUser);
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
