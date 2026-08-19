import { createRouter, createWebHistory } from "vue-router";
import { useAuthStore } from "@/stores/auth";
import LibraryView from "@/views/LibraryView.vue";

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: "/",
      redirect: "/library",
    },
    {
      path: "/login",
      name: "login",
      component: () => import("@/views/LoginView.vue"),
      meta: { requiresGuest: true },
    },
    {
      path: "/library",
      name: "library",
      component: LibraryView,
      meta: { requiresAuth: true, keepAlive: true },
    },
    {
      path: "/reader/:id",
      name: "reader",
      component: () => import("@/views/ReaderView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/version-compare",
      name: "version-compare",
      component: () => import("@/views/VersionComparisonView.vue"),
      meta: { requiresAuth: true },
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("@/views/SettingsView.vue"),
      meta: { requiresAuth: true },
    },
    // 管理员路由
    {
      path: "/admin/settings",
      name: "admin-settings",
      component: () => import("@/views/SettingsView.vue"),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
  ],
});

// 路由守卫
router.beforeEach((to, from, next) => {
  const authStore = useAuthStore();

  // 需要认证的路由
  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    // 保留完整目标地址，登录成功后可恢复 query/hash 及阅读器上下文。
    next({
      name: "login",
      query: { redirect: to.fullPath },
    });
    return;
  }

  // 访客页面（已登录用户不能访问）
  if (to.meta.requiresGuest && authStore.isAuthenticated) {
    next("/library");
    return;
  }

  // 管理员权限检查
  if (to.meta.requiresAdmin && !authStore.isAdmin) {
    // 非管理员用户尝试访问管理页面，重定向到主页
    next("/library");
    return;
  }

  next();
});

export default router;
