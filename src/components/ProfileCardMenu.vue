<template>
  <el-dropdown trigger="click" @command="handleCommand" @click.stop>
    <button class="action-btn more-btn menu-trigger-btn" @click.stop>
      <el-icon><MoreFilled /></el-icon>
    </button>

    <template #dropdown>
      <el-dropdown-menu class="profile-dropdown-menu">
        <el-dropdown-item command="edit">
          <el-icon><Edit /></el-icon>
          <span>{{ $t('profile.actions.edit') }}</span>
        </el-dropdown-item>
        <!-- 只有正在运行的配置才需要「关闭」 -->
        <el-dropdown-item v-if="props.profile.is_running" command="close">
          <el-icon><SwitchButton /></el-icon>
          <span>{{ $t('profile.actions.close') }}</span>
        </el-dropdown-item>
        <el-dropdown-item command="performance">
          <el-icon><DataLine /></el-icon>
          <span>{{ $t('profile.actions.performance') }}</span>
        </el-dropdown-item>
        <el-dropdown-item command="backup">
          <el-icon><DocumentCopy /></el-icon>
          <span>{{ $t('profile.actions.backup') }}</span>
        </el-dropdown-item>
        <el-dropdown-item command="openDir">
          <el-icon><FolderOpened /></el-icon>
          <span>{{ $t('profile.actions.openDir') }}</span>
        </el-dropdown-item>
        <el-dropdown-item command="clearCache">
          <el-icon><Brush /></el-icon>
          <span>{{ $t('profile.actions.clearCache') }}</span>
        </el-dropdown-item>
        <el-dropdown-item command="toggleTemplate">
          <el-icon><Files /></el-icon>
          <span>{{
            isTemplate ? $t('profile.actions.unsetTemplate') : $t('profile.actions.setTemplate')
          }}</span>
        </el-dropdown-item>
        <el-dropdown-item command="repair">
          <el-icon><Refresh /></el-icon>
          <span>{{ $t('profile.actions.repair') }}</span>
        </el-dropdown-item>
        <el-dropdown-item divided class="move-item">
          <el-dropdown placement="left-start" trigger="hover" @command="handleMoveCommand">
            <span class="move-trigger">
              <el-icon><Folder /></el-icon>
              <span>{{ $t('workspace.actions.moveTo') }}</span>
              <el-icon class="move-arrow"><ArrowRight /></el-icon>
            </span>
            <template #dropdown>
              <el-dropdown-menu class="move-dropdown-menu">
                <el-dropdown-item :command="UNASSIGNED">
                  <span>{{ $t('workspace.unassigned') }}</span>
                </el-dropdown-item>
                <el-dropdown-item
                  v-for="ws in workspaceStore.workspaces"
                  :key="ws.id"
                  :command="ws.id"
                >
                  <span class="ws-dot" :style="{ background: ws.color || '#909399' }" />
                  <span>{{ ws.icon || '📁' }} {{ ws.name }}</span>
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </el-dropdown-item>
        <el-dropdown-item divided command="delete" class="danger-item">
          <el-icon><Delete /></el-icon>
          <span>{{ $t('profile.actions.delete') }}</span>
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  Edit,
  Delete,
  DocumentCopy,
  FolderOpened,
  MoreFilled,
  DataLine,
  Refresh,
  Folder,
  ArrowRight,
  Brush,
  Files,
  SwitchButton
} from '@element-plus/icons-vue';
import type { Profile } from '../types';
import { api } from '../api';
import { useWorkspaceStore, UNASSIGNED } from '../store/workspace';
import { useProfileStore } from '../store/profile';
import { formatSize } from '../utils/format';

const props = defineProps<{ profile: Profile }>();

/** 菜单自身能完成的动作（清理缓存、模板、移动）就地处理；其余抛回父组件 */
const emit = defineEmits<{
  edit: [profile: Profile];
  backup: [profile: Profile];
  performance: [profile: Profile];
  delete: [profile: Profile];
  openDir: [profile: Profile];
  repair: [profile: Profile];
}>();

const { t } = useI18n();
const profileStore = useProfileStore();
const workspaceStore = useWorkspaceStore();

const isTemplate = computed(() => profileStore.templateIds.includes(props.profile.id));

const handleMoveCommand = async (command: string) => {
  const target = command === UNASSIGNED ? null : command;

  try {
    await workspaceStore.moveProfile(props.profile.id, target);
    ElMessage.success(t('common.success'));
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  }
};

const handleToggleTemplate = async () => {
  try {
    await profileStore.toggleTemplate(props.profile.id);
    ElMessage.success(
      isTemplate.value ? t('profile.messages.templateUnset') : t('profile.messages.templateSet')
    );
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  }
};

const handleClearCache = async () => {
  try {
    await ElMessageBox.confirm(
      t('profile.messages.clearCacheContent'),
      t('profile.messages.clearCacheTitle'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        type: 'warning',
      }
    );

    const freed = await api.clearProfileCache(props.profile.id);
    await profileStore.loadSizes();
    ElMessage.success(t('profile.messages.cacheFreed', { size: formatSize(freed) }));
  } catch (error) {
    if (error !== 'cancel') {
      // 后端错误（如配置正在运行）会以字符串形式抛出，直接展示给用户
      ElMessage.error(typeof error === 'string' ? error : t('common.error'));
      console.error(error);
    }
  }
};

const handleCommand = (command: string) => {
  switch (command) {
    case 'close':
      profileStore.handleClose(props.profile);
      break;
    case 'edit':
      emit('edit', props.profile);
      break;
    case 'backup':
      emit('backup', props.profile);
      break;
    case 'openDir':
      emit('openDir', props.profile);
      break;
    case 'delete':
      emit('delete', props.profile);
      break;
    case 'performance':
      emit('performance', props.profile);
      break;
    case 'repair':
      emit('repair', props.profile);
      break;
    case 'clearCache':
      handleClearCache();
      break;
    case 'toggleTemplate':
      handleToggleTemplate();
      break;
  }
};
</script>

<style lang="scss">
// 触发按钮的样式无法继承父组件的 scoped 样式，这里单独定义一份
// 下拉菜单本体通过 teleport 挂到 body，样式沿用 ProfileCard 里的全局段
.menu-trigger-btn {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-medium);
  background: var(--bg-primary);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s ease;
  outline: none;

  .el-icon {
    font-size: 16px;
  }

  &:hover {
    border-color: var(--primary-300);
    color: var(--primary-600);
    background: var(--primary-50);
    transform: rotate(90deg);
  }

  &:active {
    transform: scale(0.95);
  }
}

.move-trigger {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  color: inherit;
  font-size: inherit;

  .move-arrow {
    margin-left: auto;
    font-size: 12px;
    opacity: 0.6;
  }
}

.ws-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
  flex: 0 0 auto;
}
</style>
