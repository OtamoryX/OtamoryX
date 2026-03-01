<template>
  <div class="min-h-screen bg-[var(--bg-secondary)] px-4 py-6">
    <!-- 页面标题 -->
    <div class="max-w-6xl mx-auto">
      <div class="flex justify-between items-center mb-6">
        <div>
          <h1 class="text-xl font-semibold text-[var(--text-primary)]">用户管理</h1>
          <p class="text-sm text-[var(--text-secondary)] mt-0.5">管理系统用户和权限</p>
        </div>
        <button
          class="px-4 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-sm rounded transition-colors flex items-center gap-2"
          @click="showCreateModal = true"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
          </svg>
          创建用户
        </button>
      </div>

      <!-- 用户列表 -->
      <div class="bg-[var(--bg-card)] border border-[var(--border)] rounded-lg overflow-hidden">
        <div v-if="loading" class="p-8 text-center">
          <div class="animate-spin rounded-full h-7 w-7 border-2 border-[var(--border)] border-t-[var(--accent)] mx-auto" />
          <p class="mt-3 text-sm text-[var(--text-secondary)]">加载中...</p>
        </div>

        <div v-else-if="error" class="p-8 text-center text-red-500 text-sm">
          <p>{{ error }}</p>
          <button class="mt-2 text-[var(--accent)] hover:underline text-xs" @click="loadUsers">重试</button>
        </div>

        <div v-else>
          <table class="min-w-full">
            <thead class="bg-[var(--bg-secondary)] border-b border-[var(--border)]">
              <tr>
                <th class="px-5 py-3 text-left text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider">用户名</th>
                <th class="px-5 py-3 text-left text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider">邮箱</th>
                <th class="px-5 py-3 text-left text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider">角色</th>
                <th class="px-5 py-3 text-left text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider">创建时间</th>
                <th class="px-5 py-3 text-right text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider">操作</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-[var(--border)]">
              <tr v-for="user in users" :key="user.id" class="hover:bg-[var(--bg-secondary)] transition-colors">
                <td class="px-5 py-3.5 whitespace-nowrap">
                  <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-full bg-[var(--accent)] flex items-center justify-center text-white text-sm font-semibold shrink-0">
                      {{ user.username.charAt(0).toUpperCase() }}
                    </div>
                    <span class="text-sm font-medium text-[var(--text-primary)]">{{ user.username }}</span>
                  </div>
                </td>
                <td class="px-5 py-3.5 whitespace-nowrap text-sm text-[var(--text-secondary)]">
                  {{ user.email || "—" }}
                </td>
                <td class="px-5 py-3.5 whitespace-nowrap">
                  <span :class="getRoleBadgeClass(user.role)" class="px-2 py-0.5 text-xs font-medium rounded">
                    {{ getRoleText(user.role) }}
                  </span>
                </td>
                <td class="px-5 py-3.5 whitespace-nowrap text-sm text-[var(--text-tertiary)]">
                  {{ formatDate(user.createdAt) }}
                </td>
                <td class="px-5 py-3.5 whitespace-nowrap text-right">
                  <div class="flex justify-end gap-3">
                    <button class="text-sm text-[var(--accent)] hover:underline" @click="editUser(user)">编辑</button>
                    <button
                      v-if="user.role !== 'admin' || adminCount > 1"
                      class="text-sm text-red-500 hover:underline"
                      @click="confirmDelete(user)"
                    >删除</button>
                    <span v-else class="text-sm text-[var(--text-tertiary)]">删除</span>
                  </div>
                </td>
              </tr>
              <tr v-if="users.length === 0">
                <td colspan="5" class="px-5 py-8 text-center text-sm text-[var(--text-tertiary)]">暂无用户</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 创建用户模态框 -->
    <div v-if="showCreateModal" class="fixed inset-0 z-50 flex items-center justify-center">
      <div class="absolute inset-0 bg-black/50" @click="showCreateModal = false" />
      <div class="relative bg-[var(--bg-card)] border border-[var(--border)] rounded-lg shadow-xl w-full max-w-sm mx-4 p-6">
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-5">创建新用户</h3>
        <form class="space-y-4" @submit.prevent="createUserSubmit">
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">用户名</label>
            <input v-model="createForm.username" type="text" required
              class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)]/30 transition-colors text-sm" />
          </div>
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">邮箱 <span class="text-[var(--text-tertiary)]">（可选）</span></label>
            <input v-model="createForm.email" type="email"
              class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)]/30 transition-colors text-sm" />
          </div>
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">密码</label>
            <input v-model="createForm.password" type="password" required
              class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)]/30 transition-colors text-sm" />
          </div>
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">角色</label>
            <select v-model="createForm.role"
              class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] transition-colors text-sm">
              <option value="user">普通用户</option>
              <option value="admin">管理员</option>
            </select>
          </div>
          <div class="flex gap-3 pt-2">
            <button type="button" class="flex-1 px-4 py-2 text-sm bg-[var(--bg-tertiary)] text-[var(--text-secondary)] rounded hover:bg-[var(--border)] transition-colors" @click="showCreateModal = false">取消</button>
            <button type="submit" :disabled="creating" class="flex-1 px-4 py-2 text-sm bg-[var(--accent)] text-white rounded hover:bg-[var(--accent-hover)] disabled:opacity-50 transition-colors">
              {{ creating ? "创建中..." : "创建" }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 编辑用户模态框 -->
    <div v-if="showEditModal" class="fixed inset-0 z-50 flex items-center justify-center">
      <div class="absolute inset-0 bg-black/50" @click="showEditModal = false" />
      <div class="relative bg-[var(--bg-card)] border border-[var(--border)] rounded-lg shadow-xl w-full max-w-sm mx-4 p-6">
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-5">编辑用户</h3>
        <form class="space-y-4" @submit.prevent="updateUserSubmit">
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">用户名</label>
            <input v-model="editForm.username" type="text" required
              class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)]/30 transition-colors text-sm" />
          </div>
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">邮箱</label>
            <input v-model="editForm.email" type="email"
              class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)]/30 transition-colors text-sm" />
          </div>
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">新密码 <span class="text-[var(--text-tertiary)]">（留空不修改）</span></label>
            <input v-model="editForm.password" type="password"
              class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)]/30 transition-colors text-sm" />
          </div>
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">角色</label>
            <select v-model="editForm.role"
              class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] transition-colors text-sm">
              <option value="user">普通用户</option>
              <option value="admin">管理员</option>
            </select>
          </div>
          <div class="flex gap-3 pt-2">
            <button type="button" class="flex-1 px-4 py-2 text-sm bg-[var(--bg-tertiary)] text-[var(--text-secondary)] rounded hover:bg-[var(--border)] transition-colors" @click="showEditModal = false">取消</button>
            <button type="submit" :disabled="updating" class="flex-1 px-4 py-2 text-sm bg-[var(--accent)] text-white rounded hover:bg-[var(--accent-hover)] disabled:opacity-50 transition-colors">
              {{ updating ? "更新中..." : "更新" }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 删除确认模态框 -->
    <div v-if="showDeleteModal" class="fixed inset-0 z-50 flex items-center justify-center">
      <div class="absolute inset-0 bg-black/50" @click="showDeleteModal = false" />
      <div class="relative bg-[var(--bg-card)] border border-[var(--border)] rounded-lg shadow-xl w-full max-w-sm mx-4 p-6 text-center">
        <svg class="w-10 h-10 mx-auto text-red-500 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
        </svg>
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-2">确认删除</h3>
        <p class="text-sm text-[var(--text-secondary)] mb-5">确定要删除用户「{{ userToDelete?.username }}」吗？此操作无法撤销。</p>
        <div class="flex gap-3">
          <button class="flex-1 px-4 py-2 text-sm bg-[var(--bg-tertiary)] text-[var(--text-secondary)] rounded hover:bg-[var(--border)] transition-colors" @click="showDeleteModal = false">取消</button>
          <button :disabled="deleting" class="flex-1 px-4 py-2 text-sm bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50 transition-colors" @click="deleteUserConfirm">
            {{ deleting ? "删除中..." : "删除" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { getUsers, createUser, updateUser, deleteUser } from "@/utils/api";
import type { User, CreateUserRequest, UpdateUserRequest } from "@/types/api";

const users = ref<User[]>([]);
const loading = ref(false);
const error = ref<string>("");
const showCreateModal = ref(false);
const showEditModal = ref(false);
const showDeleteModal = ref(false);

const createForm = ref<CreateUserRequest>({ username: "", email: "", password: "", role: "user" });
const editForm = ref<UpdateUserRequest>({ username: "", email: "", password: "", role: "user" });
const editingUserId = ref<string>("");
const userToDelete = ref<User | null>(null);

const creating = ref(false);
const updating = ref(false);
const deleting = ref(false);

const adminCount = computed(() => users.value.filter((u) => u.role === "admin").length);

const loadUsers = async () => {
  loading.value = true; error.value = "";
  try { users.value = await getUsers(); }
  catch (err: any) { error.value = err.response?.data?.message || "加载用户列表失败"; }
  finally { loading.value = false; }
};

const createUserSubmit = async () => {
  creating.value = true;
  try { await createUser(createForm.value); showCreateModal.value = false; resetCreateForm(); await loadUsers(); }
  catch (err: any) { error.value = err.response?.data?.message || "创建用户失败"; }
  finally { creating.value = false; }
};

const editUser = (user: User) => {
  editingUserId.value = user.id;
  editForm.value = { username: user.username, email: user.email || "", password: "", role: user.role };
  showEditModal.value = true;
};

const updateUserSubmit = async () => {
  updating.value = true;
  try {
    const updateData: UpdateUserRequest = { username: editForm.value.username, email: editForm.value.email || undefined, role: editForm.value.role };
    if (editForm.value.password?.trim()) updateData.password = editForm.value.password;
    await updateUser(editingUserId.value, updateData);
    showEditModal.value = false; await loadUsers();
  } catch (err: any) { error.value = err.response?.data?.message || "更新用户失败"; }
  finally { updating.value = false; }
};

const confirmDelete = (user: User) => { userToDelete.value = user; showDeleteModal.value = true; };

const deleteUserConfirm = async () => {
  if (!userToDelete.value) return;
  deleting.value = true;
  try { await deleteUser(userToDelete.value.id); showDeleteModal.value = false; userToDelete.value = null; await loadUsers(); }
  catch (err: any) { error.value = err.response?.data?.message || "删除用户失败"; }
  finally { deleting.value = false; }
};

const resetCreateForm = () => { createForm.value = { username: "", email: "", password: "", role: "user" }; };

const getRoleText = (role: string) => role === "admin" ? "管理员" : "普通用户";
const getRoleBadgeClass = (role: string) => role === "admin"
  ? "bg-purple-500/15 text-purple-400"
  : "bg-green-500/15 text-green-400";
const formatDate = (dateString: string) => new Date(dateString).toLocaleString("zh-CN");

onMounted(() => { loadUsers(); });
</script>
