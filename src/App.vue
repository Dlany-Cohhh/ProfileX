<template>
  <div class="app-container">
    <WindowResizeHandles />

    <AppHeader @settings="showSettings = true" />

    <div class="app-body">
      <WorkspaceSidebar />

      <div class="app-content">
        <AppMain
          @create="handleCreate"
          @edit="handleEdit"
          @backup="handleBackup"
          @performance="handleShowPerformance"
        />
      </div>
    </div>

    <ProfileForm
      v-model="showForm"
      :profile="editingProfile"
      :existing-tags="profileStore.existingTags"
      :default-workspace-id="activeWorkspaceId"
      @success="handleFormSuccess"
    />

    <BackupDialog
      v-model="showBackupDialog"
      :profile="selectedProfile"
      @success="profileStore.loadProfiles"
    />

    <PerformanceDialog
      v-model="showPerformanceDialog"
      :profile="selectedProfile"
    />

    <SettingsDialog v-model="showSettings" @changed="handleFormSuccess" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import AppHeader from './components/layout/AppHeader.vue';
import AppMain from './components/layout/AppMain.vue';
import WorkspaceSidebar from './components/layout/WorkspaceSidebar.vue';
import WindowResizeHandles from './components/layout/WindowResizeHandles.vue';
import ProfileForm from './components/ProfileForm.vue';
import BackupDialog from './components/BackupDialog.vue';
import PerformanceDialog from './components/PerformanceDialog.vue';
import SettingsDialog from './components/SettingsDialog.vue';
import type { Profile } from './types';
import { useProfileStore } from './store/profile';
import { useWorkspaceStore, UNASSIGNED } from './store/workspace';

const profileStore = useProfileStore();
const workspaceStore = useWorkspaceStore();

/** 新建配置时预选当前工作区（「全部」「未分组」不预选） */
const activeWorkspaceId = computed(() => {
  const id = workspaceStore.activeWorkspaceId;
  return id && id !== UNASSIGNED ? id : null;
});

const handleFormSuccess = async () => {
  await profileStore.loadProfiles();
  await profileStore.loadTemplates();
  await workspaceStore.loadWorkspaces();
  profileStore.loadSizes();
};

const showForm = ref(false);
const showBackupDialog = ref(false);
const showPerformanceDialog = ref(false);
const showSettings = ref(false);
const editingProfile = ref<Profile | null>(null);
const selectedProfile = ref<Profile | null>(null);

const handleCreate = () => {
  editingProfile.value = null;
  showForm.value = true;
};

const handleEdit = (profile: Profile) => {
  editingProfile.value = profile;
  showForm.value = true;
};

const handleBackup = (profile: Profile) => {
  selectedProfile.value = profile;
  showBackupDialog.value = true;
};

const handleShowPerformance = (profile: Profile) => {
  selectedProfile.value = profile;
  showPerformanceDialog.value = true;
};

onMounted(async () => {
  await workspaceStore.loadWorkspaces();
  await profileStore.loadProfiles();
  await profileStore.loadTemplates();
  await profileStore.initStatusListener();
  // 体积统计要遍历文件，比较慢，放后台跑不阻塞界面
  profileStore.loadSizes();
});
</script>

<style lang="scss">
.app-container {
  min-height: 100vh;
  background: var(--bg-secondary);
}

.app-body {
  display: flex;
  align-items: flex-start;
}

.app-content {
  flex: 1;
  min-width: 0;
}
</style>
