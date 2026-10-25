<template>
  <main class="app-main">
    <div v-if="profileStore.loading" class="loading-container">
      <el-skeleton :rows="4" animated />
    </div>

    <el-empty
      v-else-if="workspaceStore.visibleProfiles.length === 0"
      :description="emptyDescription"
      class="empty-state"
    >
      <el-button v-if="!profileStore.searchQuery" type="primary" @click="$emit('create')">
        {{ $t('header.newProfile') }}
      </el-button>
    </el-empty>

    <template v-else>
      <div class="list-toolbar">
        <el-checkbox
          :model-value="allSelected"
          :indeterminate="partiallySelected"
          @change="profileStore.toggleSelectAll(visibleIds)"
        >
          {{ $t('main.selectAll') }}
        </el-checkbox>
        <span class="toolbar-hint">{{ $t('main.selectAllHint', { n: visibleIds.length }) }}</span>
      </div>

      <div
        ref="gridRef"
        :class="['profiles-container', profileStore.viewMode === 'grid' ? 'grid-view' : 'list-view']"
        @dragstart="onDragStart"
        @dragend="draggingId = null"
        @dragover.prevent="onDragOver"
        @dragleave="onDragLeave"
        @drop.prevent="onDrop"
      >
        <template
          v-for="(profile, index) in workspaceStore.visibleProfiles"
          :key="profile.id"
        >
          <div v-if="dropIndex === index" class="drop-indicator" />
          <ProfileCard
            :profile="profile"
            :is-launching="profileStore.launchingProfiles.has(profile.id)"
            :is-selected="profileStore.selectedIds.has(profile.id)"
            :class="{ 'is-dragging': draggingId === profile.id }"
            :style="{ animationDelay: `${index * 50}ms` }"
            @launch="profileStore.handleLaunch"
            @toggleSelection="profileStore.toggleSelection"
            @edit="$emit('edit', profile)"
            @backup="$emit('backup', profile)"
            @performance="$emit('performance', profile)"
            @delete="handleDelete"
            @openDir="handleOpenDir"
            @repair="handleRepair"
          />
        </template>
        <div v-if="dropIndex === workspaceStore.visibleProfiles.length" class="drop-indicator" />
      </div>
    </template>

    <!-- 批量操作栏 -->
    <transition name="slide-up">
      <div v-if="profileStore.selectedIds.size > 0" class="batch-bar">
        <div class="batch-info">
          <span class="batch-count">{{ $t('main.batchSelected', { n: profileStore.selectedIds.size }) }}</span>
        </div>
        <div class="batch-actions">
          <el-button type="primary" plain @click="profileStore.handleBatchLaunch">
            <el-icon><VideoPlay /></el-icon>
            {{ $t('profile.actions.launch') }}
          </el-button>

          <el-dropdown trigger="click" @command="handleBatchMove">
            <el-button plain>
              <el-icon><Folder /></el-icon>
              {{ $t('workspace.actions.moveTo') }}
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item :command="UNASSIGNED">
                  {{ $t('workspace.unassigned') }}
                </el-dropdown-item>
                <el-dropdown-item
                  v-for="ws in workspaceStore.workspaces"
                  :key="ws.id"
                  :command="ws.id"
                >
                  {{ ws.icon || '📁' }} {{ ws.name }}
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
          <el-button plain :loading="clearingCache" @click="handleBatchClearCache">
            <el-icon v-if="!clearingCache"><Brush /></el-icon>
            {{ $t('profile.actions.clearCache') }}
          </el-button>
          <el-button type="info" plain @click="profileStore.clearSelection">
            {{ $t('common.cancel') }}
          </el-button>
        </div>
      </div>
    </transition>
  </main>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed, ref } from 'vue';
import { useProfileStore } from '../../store/profile';
import { useWorkspaceStore } from '../../store/workspace';
import ProfileCard from '../ProfileCard.vue';
import { api } from '../../api';
import { ElMessage, ElMessageBox } from 'element-plus';
import { VideoPlay, Brush, Folder } from '@element-plus/icons-vue';
import { formatSize } from '../../utils/format';
import { PROFILE_REORDER_TYPE } from '../../utils/dnd';
import { UNASSIGNED } from '../../store/workspace';
import type { Profile } from '../../types';

defineEmits(['create', 'edit', 'backup', 'performance']);
const profileStore = useProfileStore();
const workspaceStore = useWorkspaceStore();
const { t } = useI18n();

const clearingCache = ref(false);

const visibleIds = computed(() => workspaceStore.visibleProfiles.map(p => p.id));

const allSelected = computed(
  () => visibleIds.value.length > 0 && visibleIds.value.every(id => profileStore.selectedIds.has(id))
);

const partiallySelected = computed(
  () => profileStore.selectedIds.size > 0 && !allSelected.value
);

const gridRef = ref<HTMLElement | null>(null);
/** 插入位置下标（0 ~ 列表长度） */
const dropIndex = ref<number | null>(null);
const draggingId = ref<string | null>(null);

const onDragStart = (event: DragEvent) => {
  draggingId.value = event.dataTransfer?.getData(PROFILE_REORDER_TYPE) || null;
};

/** 按阅读顺序判断指针落在哪个卡片之前 */
const onDragOver = (event: DragEvent) => {
  const container = gridRef.value;
  if (!container) return;

  const cards = Array.from(container.querySelectorAll('.profile-card')) as HTMLElement[];

  let index = cards.length;

  for (let i = 0; i < cards.length; i += 1) {
    const rect = cards[i].getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;

    // 同一行内比 X，跨行比 Y，兼容网格与列表两种视图
    const sameRow = event.clientY >= rect.top && event.clientY <= rect.bottom;
    const before = sameRow ? event.clientX < centerX : event.clientY < centerY;

    if (before) {
      index = i;
      break;
    }
  }

  dropIndex.value = index;
};

const onDragLeave = (event: DragEvent) => {
  // 只有真正离开容器才清除，避免子元素间移动导致闪烁
  if (!gridRef.value?.contains(event.relatedTarget as Node | null)) {
    dropIndex.value = null;
  }
};

const onDrop = async (event: DragEvent) => {
  const id = event.dataTransfer?.getData(PROFILE_REORDER_TYPE);
  const index = dropIndex.value;

  dropIndex.value = null;
  draggingId.value = null;

  if (!id || index === null) return;

  const visible = visibleIds.value;
  const from = visible.indexOf(id);
  if (from === -1) return;

  // 移除自身后，其后的插入点要前移一位
  let to = index > from ? index - 1 : index;
  if (to === from) return;

  const anchorId = visible[to];

  // 只对可见列表重排会打乱被过滤掉的配置，因此按全局顺序重算
  const next = profileStore.profiles.map(p => p.id).filter(pid => pid !== id);
  const anchorPos = next.indexOf(anchorId);
  const insertAt = to > from ? anchorPos + 1 : anchorPos;

  next.splice(insertAt < 0 ? next.length : insertAt, 0, id);

  try {
    await api.reorderProfiles(next);
    await profileStore.loadProfiles();
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  }
};

const handleBatchMove = async (command: string) => {
  const ids = Array.from(profileStore.selectedIds);
  if (ids.length === 0) return;

  try {
    await workspaceStore.moveProfiles(ids, command === UNASSIGNED ? null : command);
    profileStore.clearSelection();
    ElMessage.success(t('common.success'));
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  }
};

const handleBatchClearCache = async () => {
  const ids = Array.from(profileStore.selectedIds);
  if (ids.length === 0) return;

  try {
    await ElMessageBox.confirm(
      t('profile.messages.clearCacheBatchContent', { count: ids.length }),
      t('profile.messages.clearCacheTitle'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        type: 'warning',
      }
    );
  } catch {
    return;
  }

  clearingCache.value = true;
  try {
    const result = await api.clearProfilesCache(ids);

    await Promise.all([profileStore.loadProfiles(), profileStore.loadSizes()]);
    profileStore.clearSelection();

    if (result.skipped > 0) {
      ElMessage.warning(
        t('profile.messages.cacheFreedPartial', {
          size: formatSize(result.freed_bytes),
          skipped: result.skipped,
        })
      );
    } else {
      ElMessage.success(t('profile.messages.cacheFreed', { size: formatSize(result.freed_bytes) }));
    }
  } catch (error) {
    ElMessage.error(typeof error === 'string' ? error : t('common.error'));
    console.error(error);
  } finally {
    clearingCache.value = false;
  }
};

const emptyDescription = computed(() => {
  if (profileStore.searchQuery) return t('common.error');
  if (workspaceStore.activeWorkspaceId !== null) return t('workspace.messages.noProfiles');
  return t('main.noProfiles');
});

const handleOpenDir = async (profile: Profile) => {
  try {
    await api.openProfileDirectory(profile.data_dir_path);
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  }
};

const handleRepair = async (profile: Profile) => {
  try {
    await api.unlockProfile(profile.id);
    ElMessage.success(t('common.success'));
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  }
};

const handleDelete = async (profile: Profile) => {
  try {
    await ElMessageBox.confirm(
      t('main.deleteConfirm.content'),
      t('main.deleteConfirm.title'),
      {
        confirmButtonText: t('common.delete'),
        cancelButtonText: t('common.cancel'),
        type: 'warning',
      }
    );

    await api.deleteProfile(profile.id);
    ElMessage.success(t('common.success'));
    await profileStore.loadProfiles();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(t('common.error'));
      console.error(error);
    }
  }
};
</script>

<style lang="scss" scoped>
.app-main {
  padding: var(--space-5);
  max-width: 1400px;
  margin: 0 auto;
  min-height: calc(100vh - 60px);
}

.loading-container {
  padding: var(--space-10);
}

.empty-state {
  padding: var(--space-16) 0;

  :deep(.el-empty__image) {
    width: 100px;
    height: 100px;
  }

  :deep(.el-empty__description) {
    margin-top: var(--space-4);
    margin-bottom: var(--space-4);
  }
}

.list-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-3);

  :deep(.el-checkbox__label) {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
}

.toolbar-hint {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.profiles-container {
  display: grid;
  gap: var(--space-4);

  &.grid-view {
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  }

  &.list-view {
    grid-template-columns: 1fr;
    max-width: 800px;
    margin: 0 auto;
  }

  > * {
    animation: fadeInUp 0.4s ease-out backwards;
  }
}

// 拖拽排序：插入位置的占位块
.drop-indicator {
  border: 2px dashed var(--primary-400);
  border-radius: var(--radius-lg);
  background: var(--primary-50, rgba(59, 130, 246, 0.08));
  min-height: 90px;

  .list-view & {
    min-height: 8px;
  }
}

.is-dragging {
  opacity: 0.4;
}

// 批量操作栏样式
.batch-bar {
  position: fixed;
  bottom: var(--space-8);
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-elevated);
  border: 1px solid var(--border-medium);
  border-radius: var(--radius-2xl);
  padding: var(--space-3) var(--space-6);
  display: flex;
  align-items: center;
  justify-content: center;
  // 按内容撑开而不是被视口压扁，否则文字会被挤成一字一行
  width: max-content;
  max-width: calc(100vw - var(--space-8));
  flex-wrap: wrap;
  gap: var(--space-6);
  box-shadow: var(--shadow-xl);
  z-index: var(--z-fixed);
  backdrop-filter: blur(12px);
  
  // Apply transparent background if we are in dark mode (html.dark)
  :global(.dark) & {
    background: rgba(39, 46, 59, 0.85); // Transparent version of bg-elevated
  }
}

.batch-info {
  flex: 0 0 auto;

  .batch-count {
    font-size: var(--text-base);
    font-weight: var(--font-semibold);
    color: var(--primary-600);
    white-space: nowrap;
  }
}

.batch-actions {
  display: flex;
  flex: 0 0 auto;
  flex-wrap: wrap;
  gap: var(--space-3);
  
  .el-button {
    height: 36px;
    border-radius: var(--radius-lg);
  }
  
  .el-button--info.is-plain {
    background: transparent;
    border-color: var(--border-medium);
    color: var(--text-secondary);
    
    &:hover {
      border-color: var(--text-primary);
      color: var(--text-primary);
      background: var(--bg-hover);
    }
  }
}

.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}

.slide-up-enter-from,
.slide-up-leave-to {
  transform: translate(-50%, 100%);
  opacity: 0;
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (max-width: 1024px) {
  .app-main {
    padding: var(--space-4);
  }

  .profiles-container.grid-view {
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: var(--space-3);
  }
}

@media (max-width: 768px) {
  .app-main {
    padding: var(--space-3);
  }

  .profiles-container.grid-view {
    grid-template-columns: 1fr;
    gap: var(--space-3);
  }

  .batch-bar {
    width: 90%;
    bottom: var(--space-4);
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
  }
}
</style>
