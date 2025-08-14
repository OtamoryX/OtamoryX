<template>
  <div class="container mx-auto px-4 py-6">
    <!-- 页面标题和操作按钮 -->
    <div class="flex justify-between items-center mb-6">
      <div>
        <h1 class="text-2xl font-bold text-gray-900">用户管理</h1>
        <p class="text-gray-600">管理系统用户和权限</p>
      </div>
      <button
        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-md flex items-center gap-2"
        @click="showCreateModal = true"
      >
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 6v6m0 0v6m0-6h6m-6 0H6"
          />
        </svg>
        创建用户
      </button>
    </div>

    <!-- 用户列表 -->
    <div class="bg-white shadow rounded-lg overflow-hidden">
      <div v-if="loading"
class="p-6 text-center">
        <div
          class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto"
        />
        <p class="mt-2 text-gray-600">加载中...</p>
      </div>

      <div v-else-if="error"
class="p-6 text-center text-red-600">
        <p>{{ error }}</p>
        <button
          class="mt-2 text-blue-600 hover:text-blue-700"
          @click="loadUsers"
        >
          重试
        </button>
      </div>

      <div v-else>
        <table class="min-w-full divide-y divide-gray-200">
          <thead class="bg-gray-50">
            <tr>
              <th
                class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                用户名
              </th>
              <th
                class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                邮箱
              </th>
              <th
                class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                角色
              </th>
              <th
                class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                创建时间
              </th>
              <th
                class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
              >
                操作
              </th>
            </tr>
          </thead>
          <tbody class="bg-white divide-y divide-gray-200">
            <tr v-for="user in users"
:key="user.id" class="hover:bg-gray-50">
              <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center">
                  <div class="flex-shrink-0 h-10 w-10">
                    <div
                      class="h-10 w-10 rounded-full bg-gray-300 flex items-center justify-center"
                    >
                      <span class="text-sm font-medium text-gray-700">
                        {{ user.username.charAt(0).toUpperCase() }}
                      </span>
                    </div>
                  </div>
                  <div class="ml-4">
                    <div class="text-sm font-medium text-gray-900">
                      {{ user.username }}
                    </div>
                  </div>
                </div>
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <div class="text-sm text-gray-900">
                  {{ user.email || "-" }}
                </div>
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <span
                  :class="getRoleBadgeClass(user.role)"
                  class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full"
                >
                  {{ getRoleText(user.role) }}
                </span>
              </td>
              <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                {{ formatDate(user.createdAt) }}
              </td>
              <td
                class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium"
              >
                <div class="flex justify-end gap-2">
                  <button
                    class="text-blue-600 hover:text-blue-900"
                    @click="editUser(user)"
                  >
                    编辑
                  </button>
                  <button
                    v-if="user.role !== 'admin' || adminCount > 1"
                    class="text-red-600 hover:text-red-900"
                    @click="confirmDelete(user)"
                  >
                    删除
                  </button>
                  <span v-else
class="text-gray-400"> 删除 </span>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 创建用户模态框 -->
    <div
      v-if="showCreateModal"
      class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"
    >
      <div
        class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white"
      >
        <div class="mt-3 text-center">
          <h3 class="text-lg font-medium text-gray-900 mb-4">创建新用户</h3>

          <form class="space-y-4" @submit.prevent="createUserSubmit">
            <div class="text-left">
              <label class="block text-sm font-medium text-gray-700 mb-1">用户名</label>
              <input
                v-model="createForm.username"
                type="text"
                required
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              />
            </div>

            <div class="text-left">
              <label class="block text-sm font-medium text-gray-700 mb-1">邮箱 (可选)</label>
              <input
                v-model="createForm.email"
                type="email"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              />
            </div>

            <div class="text-left">
              <label class="block text-sm font-medium text-gray-700 mb-1">密码</label>
              <input
                v-model="createForm.password"
                type="password"
                required
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              />
            </div>

            <div class="text-left">
              <label class="block text-sm font-medium text-gray-700 mb-1">角色</label>
              <select
                v-model="createForm.role"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="user">普通用户</option>
                <option value="admin">管理员</option>
              </select>
            </div>

            <div class="flex gap-3 pt-4">
              <button
                type="button"
                class="flex-1 px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400"
                @click="showCreateModal = false"
              >
                取消
              </button>
              <button
                type="submit"
                :disabled="creating"
                class="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {{ creating ? "创建中..." : "创建" }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>

    <!-- 编辑用户模态框 -->
    <div
      v-if="showEditModal"
      class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"
    >
      <div
        class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white"
      >
        <div class="mt-3 text-center">
          <h3 class="text-lg font-medium text-gray-900 mb-4">编辑用户</h3>

          <form class="space-y-4" @submit.prevent="updateUserSubmit">
            <div class="text-left">
              <label class="block text-sm font-medium text-gray-700 mb-1">用户名</label>
              <input
                v-model="editForm.username"
                type="text"
                required
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              />
            </div>

            <div class="text-left">
              <label class="block text-sm font-medium text-gray-700 mb-1">邮箱</label>
              <input
                v-model="editForm.email"
                type="email"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              />
            </div>

            <div class="text-left">
              <label class="block text-sm font-medium text-gray-700 mb-1">新密码 (留空则不修改)</label>
              <input
                v-model="editForm.password"
                type="password"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              />
            </div>

            <div class="text-left">
              <label class="block text-sm font-medium text-gray-700 mb-1">角色</label>
              <select
                v-model="editForm.role"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="user">普通用户</option>
                <option value="admin">管理员</option>
              </select>
            </div>

            <div class="flex gap-3 pt-4">
              <button
                type="button"
                class="flex-1 px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400"
                @click="showEditModal = false"
              >
                取消
              </button>
              <button
                type="submit"
                :disabled="updating"
                class="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {{ updating ? "更新中..." : "更新" }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>

    <!-- 删除确认模态框 -->
    <div
      v-if="showDeleteModal"
      class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"
    >
      <div
        class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white"
      >
        <div class="mt-3 text-center">
          <svg
            class="w-12 h-12 mx-auto text-red-600 mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
            />
          </svg>
          <h3 class="text-lg font-medium text-gray-900 mb-2">确认删除</h3>
          <p class="text-gray-600 mb-4">
            确定要删除用户 "{{ userToDelete?.username }}" 吗？此操作无法撤销。
          </p>

          <div class="flex gap-3">
            <button
              class="flex-1 px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400"
              @click="showDeleteModal = false"
            >
              取消
            </button>
            <button
              :disabled="deleting"
              class="flex-1 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50"
              @click="deleteUserConfirm"
            >
              {{ deleting ? "删除中..." : "删除" }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { getUsers, createUser, updateUser, deleteUser } from "@/utils/api";
import type { User, CreateUserRequest, UpdateUserRequest } from "@/types/api";

// 响应式数据
const users = ref<User[]>([]);
const loading = ref(false);
const error = ref<string>("");

// 模态框状态
const showCreateModal = ref(false);
const showEditModal = ref(false);
const showDeleteModal = ref(false);

// 表单数据
const createForm = ref<CreateUserRequest>({
  username: "",
  email: "",
  password: "",
  role: "user",
});

const editForm = ref<UpdateUserRequest>({
  username: "",
  email: "",
  password: "",
  role: "user",
});

const editingUserId = ref<string>("");
const userToDelete = ref<User | null>(null);

// 操作状态
const creating = ref(false);
const updating = ref(false);
const deleting = ref(false);

// 计算属性
const adminCount = computed(
  () => users.value.filter((u) => u.role === "admin").length,
);

// 加载用户列表
const loadUsers = async () => {
  loading.value = true;
  error.value = "";
  try {
    users.value = await getUsers();
  } catch (err: any) {
    error.value = err.response?.data?.message || "加载用户列表失败";
  } finally {
    loading.value = false;
  }
};

// 创建用户
const createUserSubmit = async () => {
  creating.value = true;
  try {
    await createUser(createForm.value);
    showCreateModal.value = false;
    resetCreateForm();
    await loadUsers();
  } catch (err: any) {
    error.value = err.response?.data?.message || "创建用户失败";
  } finally {
    creating.value = false;
  }
};

// 编辑用户
const editUser = (user: User) => {
  editingUserId.value = user.id;
  editForm.value = {
    username: user.username,
    email: user.email || "",
    password: "",
    role: user.role,
  };
  showEditModal.value = true;
};

const updateUserSubmit = async () => {
  updating.value = true;
  try {
    const updateData: UpdateUserRequest = {
      username: editForm.value.username,
      email: editForm.value.email || undefined,
      role: editForm.value.role,
    };

    // 只有在用户输入了新密码时才包含密码字段
    if (editForm.value.password?.trim()) {
      updateData.password = editForm.value.password;
    }

    await updateUser(editingUserId.value, updateData);
    showEditModal.value = false;
    await loadUsers();
  } catch (err: any) {
    error.value = err.response?.data?.message || "更新用户失败";
  } finally {
    updating.value = false;
  }
};

// 删除用户确认
const confirmDelete = (user: User) => {
  userToDelete.value = user;
  showDeleteModal.value = true;
};

const deleteUserConfirm = async () => {
  if (!userToDelete.value) return;

  deleting.value = true;
  try {
    await deleteUser(userToDelete.value.id);
    showDeleteModal.value = false;
    userToDelete.value = null;
    await loadUsers();
  } catch (err: any) {
    error.value = err.response?.data?.message || "删除用户失败";
  } finally {
    deleting.value = false;
  }
};

// 重置创建表单
const resetCreateForm = () => {
  createForm.value = {
    username: "",
    email: "",
    password: "",
    role: "user",
  };
};

// 工具函数
const getRoleText = (role: string) => {
  return role === "admin" ? "管理员" : "普通用户";
};

const getRoleBadgeClass = (role: string) => {
  return role === "admin"
    ? "bg-red-100 text-red-800"
    : "bg-green-100 text-green-800";
};

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString("zh-CN");
};

// 初始化
onMounted(() => {
  loadUsers();
});
</script>
