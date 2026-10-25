<template>
  <aside ref="sidebarRef" class="workspace-sidebar" :class="{ collapsed, resizing }" :style="sidebarStyle">
    <!-- 右边框拖拽调宽 -->
    <div v-if="!collapsed" class="resize-handle" @mousedown.prevent="startResize" />

    <div class="sidebar-title">
      <span v-if="!collapsed" class="title-text">{{ $t('workspace.title') }}</span>
      <div class="title-actions">
        <el-button link size="small" class="header-btn" :title="$t('workspace.title')" @click="openCreate">
          <el-icon><Plus /></el-icon>
        </el-button>
        <el-button
          link
          size="small"
          class="header-btn"
          :class="{ 'collapse-btn': true }"
          @click="toggleCollapsed"
        >
          <el-icon>
            <Fold v-if="!collapsed" />
            <Expand v-else />
          </el-icon>
        </el-button>
      </div>
    </div>

    <ul class="workspace-list">
      <li
        :class="['ws-item', { active: workspaceStore.activeWorkspaceId === null }]"
        :title="collapsed ? $t('workspace.all') : undefined"
        @click="workspaceStore.activeWorkspaceId = null"
      >
        <el-icon class="ws-lead"><Menu /></el-icon>
        <span v-if="!collapsed" class="ws-name">{{ $t('workspace.all') }}</span>
        <span v-if="!collapsed" class="ws-count">{{ profileStore.profiles.length }}</span>
      </li>

      <li
        :class="[
          'ws-item',
          { active: workspaceStore.activeWorkspaceId === UNASSIGNED, 'drop-target': dragOverId === UNASSIGNED }
        ]"
        :title="collapsed ? $t('workspace.unassigned') : undefined"
        @click="workspaceStore.activeWorkspaceId = UNASSIGNED"
        @dragover.prevent="dragOverId = UNASSIGNED"
        @dragleave="clearDragOver(UNASSIGNED)"
        @drop.prevent="handleDrop($event, UNASSIGNED)"
      >
        <el-icon class="ws-lead"><FolderOpened /></el-icon>
        <span v-if="!collapsed" class="ws-name">{{ $t('workspace.unassigned') }}</span>
        <span v-if="!collapsed" class="ws-count">{{ workspaceStore.counts[UNASSIGNED] ?? 0 }}</span>
      </li>
    </ul>

    <el-divider class="ws-divider" />

    <ul class="workspace-list scrollable">
      <li
        v-for="ws in workspaceStore.workspaces"
        :key="ws.id"
        :class="[
          'ws-item',
          { active: workspaceStore.activeWorkspaceId === ws.id, 'drop-target': dragOverId === ws.id }
        ]"
        @click="workspaceStore.activeWorkspaceId = ws.id"
        @dragover.prevent="dragOverId = ws.id"
        @dragleave="clearDragOver(ws.id)"
        @drop.prevent="handleDrop($event, ws.id)"
      >
        <span class="ws-lead ws-badge" :style="{ background: tint(ws.color) }">{{ ws.icon || '📁' }}</span>
        <span v-if="!collapsed" class="ws-name" :title="ws.description || ws.name">{{ ws.name }}</span>
        <span v-if="!collapsed" class="ws-count">{{ workspaceStore.counts[ws.id] ?? 0 }}</span>
        <span v-if="!collapsed" class="ws-actions">
          <el-icon class="act" @click.stop="openEdit(ws)"><EditPen /></el-icon>
          <el-icon class="act danger" @click.stop="handleDelete(ws)"><Delete /></el-icon>
        </span>
      </li>

      <li v-if="workspaceStore.workspaces.length === 0" class="ws-empty">
        {{ $t('workspace.noWorkspaces') }}
      </li>
    </ul>

    <div class="sidebar-footer">
      <el-button
        class="footer-btn"
        size="small"
        :title="collapsed ? $t('workspace.actions.launchAll') : undefined"
        :loading="workspaceStore.launching"
        @click="workspaceStore.launchActiveWorkspace"
      >
        <el-icon><VideoPlay /></el-icon>
        <span v-if="!collapsed">{{ $t('workspace.actions.launchAll') }}</span>
      </el-button>
      <el-button
        class="footer-btn"
        size="small"
        :title="collapsed ? $t('workspace.actions.backupAll') : undefined"
        :loading="workspaceStore.backing"
        @click="workspaceStore.backupActiveWorkspace"
      >
        <el-icon v-if="!workspaceStore.backing"><DocumentCopy /></el-icon>
        <span v-if="!collapsed && workspaceStore.backing && workspaceStore.backupProgress">
          {{
            $t('workspace.actions.backingProgress', {
              current: workspaceStore.backupProgress.current,
              total: workspaceStore.backupProgress.total
            })
          }}
        </span>
        <span v-else-if="!collapsed">{{ $t('workspace.actions.backupAll') }}</span>
      </el-button>

      <div v-if="workspaceStore.backing" class="backup-progress">
        <el-progress
          :percentage="backupPercent"
          :stroke-width="4"
          :show-text="false"
          striped
          striped-flow
        />
        <span class="backup-progress-name">{{ workspaceStore.backupProgress?.name || '' }}</span>
      </div>
    </div>

    <WorkspaceDialog v-model="showDialog" :workspace="editingWorkspace" @success="onDialogSuccess" />
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  Plus,
  Menu,
  FolderOpened,
  EditPen,
  Delete,
  VideoPlay,
  DocumentCopy,
  Fold,
  Expand,
} from '@element-plus/icons-vue';
import type { Workspace } from '../../types';
import { useProfileStore } from '../../store/profile';
import { useWorkspaceStore, UNASSIGNED } from '../../store/workspace';
import WorkspaceDialog from '../WorkspaceDialog.vue';
import { PROFILE_DRAG_TYPE } from '../../utils/dnd';

const { t } = useI18n();
const profileStore = useProfileStore();
const workspaceStore = useWorkspaceStore();

const showDialog = ref(false);
const editingWorkspace = ref<Workspace | null>(null);
/** 当前被拖拽悬停的工作区 id，用于拖放高亮 */
const dragOverId = ref<string | null>(null);

const SIDEBAR_COLLAPSED_KEY = 'spaceshift.sidebar.collapsed';
const SIDEBAR_WIDTH_KEY = 'spaceshift.sidebar.width';
const MIN_SIDEBAR_WIDTH = 180;
const MAX_SIDEBAR_WIDTH = 420;
const COLLAPSED_SIDEBAR_WIDTH = 56;

const sidebarRef = ref<HTMLElement | null>(null);

/** 收起状态和宽度都记住，下次打开还是老样子 */
const collapsed = ref(localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === '1');

const clampWidth = (value: number) =>
  Number.isFinite(value) ? Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, value)) : 220;

const storedWidth = Number(localStorage.getItem(SIDEBAR_WIDTH_KEY));
const sidebarWidth = ref(storedWidth > 0 ? clampWidth(storedWidth) : 220);

const sidebarStyle = computed(() => {
  const width = collapsed.value ? COLLAPSED_SIDEBAR_WIDTH : sidebarWidth.value;
  // flex-basis 一起改，否则 flex 布局里宽度不生效
  return { width: `${width}px`, flexBasis: `${width}px` };
});

const toggleCollapsed = () => {
  collapsed.value = !collapsed.value;
  localStorage.setItem(SIDEBAR_COLLAPSED_KEY, collapsed.value ? '1' : '0');
};

// 拖右边框调宽：监听挂在 document 上，鼠标移出侧边栏也不会断
const resizing = ref(false);

const onResizeMove = (event: MouseEvent) => {
  if (!resizing.value) return;

  const left = sidebarRef.value?.getBoundingClientRect().left ?? 0;
  sidebarWidth.value = clampWidth(event.clientX - left);
};

const stopResize = () => {
  if (!resizing.value) return;

  resizing.value = false;
  document.removeEventListener('mousemove', onResizeMove);
  document.removeEventListener('mouseup', stopResize);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
  localStorage.setItem(SIDEBAR_WIDTH_KEY, String(sidebarWidth.value));
};

const startResize = () => {
  if (resizing.value || collapsed.value) return;

  resizing.value = true;
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
  document.addEventListener('mousemove', onResizeMove);
  document.addEventListener('mouseup', stopResize);
};

onBeforeUnmount(stopResize);

const backupPercent = computed(() => {
  const progress = workspaceStore.backupProgress;
  if (!progress || progress.total === 0) return 0;
  return Math.round((progress.current / progress.total) * 100);
});

const clearDragOver = (id: string) => {
  if (dragOverId.value === id) dragOverId.value = null;
};

/** 接收从配置卡片拖入的配置（可能是一批勾选项） */
const handleDrop = async (event: DragEvent, target: string) => {
  dragOverId.value = null;

  const raw = event.dataTransfer?.getData(PROFILE_DRAG_TYPE);
  if (!raw) return;

  let ids: string[] = [];
  try {
    ids = JSON.parse(raw);
  } catch {
    return;
  }

  if (!Array.isArray(ids) || ids.length === 0) return;

  try {
    await workspaceStore.moveProfiles(ids, target === UNASSIGNED ? null : target);
    profileStore.clearSelection();
    ElMessage.success(t('common.success'));
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  }
};

/** 将工作区主题色转为淡背景色，用于图标槽底色 */
const tint = (color?: string | null) => {
  const hex = (color || '#909399').replace('#', '');
  const full = hex.length === 3 ? hex.split('').map(c => c + c).join('') : hex;

  if (full.length !== 6) return 'rgba(144, 147, 153, 0.15)';

  const r = parseInt(full.slice(0, 2), 16);
  const g = parseInt(full.slice(2, 4), 16);
  const b = parseInt(full.slice(4, 6), 16);

  if ([r, g, b].some(n => Number.isNaN(n))) return 'rgba(144, 147, 153, 0.15)';

  return `rgba(${r}, ${g}, ${b}, 0.15)`;
};

const openCreate = () => {
  editingWorkspace.value = null;
  showDialog.value = true;
};

const openEdit = (workspace: Workspace) => {
  editingWorkspace.value = workspace;
  showDialog.value = true;
};

const onDialogSuccess = async () => {
  await workspaceStore.loadWorkspaces();
};

const handleDelete = async (workspace: Workspace) => {
  try {
    await ElMessageBox.confirm(
      t('workspace.deleteConfirm.content'),
      t('workspace.deleteConfirm.title'),
      {
        confirmButtonText: t('common.delete'),
        cancelButtonText: t('common.cancel'),
        type: 'warning',
      }
    );

    await workspaceStore.removeWorkspace(workspace.id);
    ElMessage.success(t('common.success'));
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(t('common.error'));
      console.error(error);
    }
  }
};
</script>

<style scoped lang="scss">
.workspace-sidebar {
  width: 220px;
  flex: 0 0 220px;
  height: calc(100vh - 60px);
  position: sticky;
  top: 60px;
  display: flex;
  flex-direction: column;
  padding: var(--space-4) var(--space-3);
  background: var(--bg-primary);
  border-right: 1px solid var(--border-light);
  overflow: hidden;
  // 收起/展开时平滑过渡；拖拽调宽时不加，避免跟手感打架
  transition: width 0.18s ease, flex-basis 0.18s ease, padding 0.18s ease;

  &.resizing {
    transition: none;
  }
}

// 拖右边框改宽度
.resize-handle {
  position: absolute;
  top: 0;
  // 贴在内侧，父级 overflow: hidden 不会裁掉
  right: 0;
  width: 5px;
  height: 100%;
  z-index: 4;
  cursor: col-resize;
  background: transparent;
  transition: background 0.15s ease;

  &:hover {
    background: var(--primary-400);
  }
}

.sidebar-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-2) var(--space-2);
  font-size: var(--text-xs);
  font-weight: var(--font-semibold);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-tertiary);

  .title-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: 0 0 auto;
  }

  .header-btn {
    padding: 2px;
    height: 22px;
    color: var(--text-tertiary);

    &:hover {
      color: var(--primary-500);
    }
  }
}

// 收起后只留图标：宽度 56px，标题与文字全部隐藏
.workspace-sidebar.collapsed {
  padding-left: 6px;
  padding-right: 6px;

  .sidebar-title {
    flex-direction: column;
    gap: 6px;
    padding: 0 0 var(--space-2);
  }

  .title-actions {
    flex-direction: column;
    gap: 6px;
  }

  .ws-item {
    justify-content: center;
    padding: var(--space-2) 0;
    gap: 0;
  }

  .ws-divider {
    margin: var(--space-2) 0;
  }

  .sidebar-footer {
    align-items: stretch;
  }

  .footer-btn {
    width: 100%;
    padding: 0;
    justify-content: center;

    :deep(.el-icon) {
      margin-right: 0;
    }
  }

  .backup-progress {
    display: none;
  }
}

.workspace-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;

  &.scrollable {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }
}

.backup-progress {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 0 4px;

  .backup-progress-name {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.ws-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-lg);
  cursor: pointer;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  transition: background 0.18s ease, color 0.18s ease;

  // 拖拽配置悬停时的可放置提示
  &.drop-target {
    background: var(--primary-50, rgba(59, 130, 246, 0.12));
    color: var(--primary-600);
    outline: 1px dashed var(--primary-400);
    outline-offset: -1px;
  }

  &:hover {
    background: var(--bg-hover);
    color: var(--text-primary);

    .ws-actions {
      opacity: 1;
    }
  }

  &.active {
    background: var(--primary-50);
    color: var(--primary-700);
    font-weight: var(--font-medium);

    :global(.dark) & {
      background: rgba(99, 102, 241, 0.16);
      color: var(--primary-300);
    }
  }
}

.ws-lead {
  width: 20px;
  height: 20px;
  flex: 0 0 20px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  line-height: 1;
}

.ws-badge {
  border-radius: 6px;
  font-size: 12px;
}

.ws-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ws-count {
  flex: 0 0 auto;
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
}

.ws-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.18s ease;

  .act {
    font-size: 14px;
    padding: 2px;
    border-radius: var(--radius-sm);

    &:hover {
      background: var(--bg-active);
      color: var(--primary-600);
    }

    &.danger:hover {
      color: var(--danger-500, #ef4444);
    }
  }
}

.ws-empty {
  padding: var(--space-4) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.ws-divider {
  margin: var(--space-3) 0;
}

.sidebar-footer {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding-top: var(--space-3);
  border-top: 1px solid var(--border-light);

  .footer-btn {
    width: 100%;
    margin: 0;
    justify-content: flex-start;

    & + .footer-btn {
      margin-left: 0;
    }

    :deep(.el-icon) {
      margin-right: 6px;
    }
  }
}

@media (max-width: 1024px) {
  .workspace-sidebar {
    width: 180px;
    flex-basis: 180px;
  }
}
</style>
