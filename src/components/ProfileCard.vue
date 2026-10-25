<template>
  <div
    class="profile-card"
    :class="{ 
      'is-launching': props.isLaunching, 
      'is-running': profile.is_running,
      'is-selected': props.isSelected 
    }"
    draggable="true"
    @click="$emit('click', profile)"
    @dragstart="handleDragStart"
  >
    <!-- 全选按钮 (仅移动端或悬停显示，或者常驻) -->
    <div class="selection-box" @click.stop="$emit('toggleSelection', profile.id)">
      <div class="checkbox" :class="{ 'checked': props.isSelected }">
        <el-icon v-if="props.isSelected"><Check /></el-icon>
      </div>
    </div>
    <!-- 状态指示条 -->
    <div class="status-bar"></div>
    <!-- 所属工作区的颜色，未分组时隐藏 -->
    <div v-if="workspaceColor" class="ws-bar" :style="{ background: workspaceColor }"></div>

    <div class="profile-card-content">
      <!-- 头像区域：没有自定义图标时用名字首字生成色块，便于一眼区分 -->
      <div class="profile-avatar-wrapper">
        <div
          class="profile-avatar"
          :style="avatarStyle"
        >
          <img v-if="profile.icon_base64" :src="profile.icon_base64" :alt="profile.name" />
          <span v-else class="avatar-initial">{{ avatarInitial }}</span>
        </div>
      </div>
      
      <!-- 信息区域 -->
      <div class="profile-info">
        <h3 class="profile-name" :title="profile.name">{{ profile.name }}</h3>
        <div class="profile-tags" v-if="isTemplate || tagList.length > 0">
          <span v-if="isTemplate" class="profile-tag template-tag">
            <Copy :size="11" :stroke-width="2" />
            {{ $t('profile.template.badge') }}
          </span>
          <span
            v-for="tag in tagList.slice(0, 3)"
            :key="tag"
            class="profile-tag"
          >
            {{ tag }}
          </span>
        </div>
        <div class="profile-meta">
          <span
            v-if="profile.is_running"
            class="status-tag running compact"
            :title="$t('profile.status.running')"
          >
            <span class="status-dot"></span>
            <span v-if="profile.debug_port" class="port-info" :title="$t('profile.status.cdpPort')">
              <span class="port-divider">|</span>
              <span class="port-num">{{ profile.debug_port }}</span>
            </span>
            <span v-if="profile.forward_port" class="port-info forward" :title="$t('profile.status.forwardPort')">
              <span class="port-divider">➔</span>
              <span class="port-num">{{ profile.forward_port }}</span>
            </span>
          </span>
          <span v-else-if="profile.last_opened_at" class="last-opened">
            <el-icon><Timer /></el-icon>
            <span>{{ formatDate(profile.last_opened_at) }}</span>
          </span>
          <span v-else class="last-opened never">
            <el-icon><Timer /></el-icon>
            <span>{{ $t('profile.status.unused') }}</span>
          </span>
          <span v-if="profile.homepage" class="homepage-indicator" :title="profile.homepage">
            <el-icon><Link /></el-icon>
          </span>
          <span v-if="sizeLabel" class="size-info" :title="$t('profile.status.size')">
            <el-icon><Coin /></el-icon>
            <span>{{ sizeLabel }}</span>
          </span>
        </div>
      </div>
      
      <!-- 操作区域 -->
      <div class="profile-actions">
        <el-tooltip
          :content="$t('profile.actions.launch')"
          placement="top"
          :show-after="300"
        >
          <button
            :class="['action-btn', 'launch-btn', { 'is-loading': props.isLaunching }]"
            @click.stop="handleLaunch"
            :disabled="props.isLaunching"
          >
            <el-icon v-if="props.isLaunching" class="loading-icon"><Loading /></el-icon>
            <el-icon v-else><VideoPlay /></el-icon>
          </button>
        </el-tooltip>
        
        <ProfileCardMenu
          :profile="profile"
          @edit="emit('edit', props.profile)"
          @backup="emit('backup', props.profile)"
          @performance="emit('performance', props.profile)"
          @delete="emit('delete', props.profile)"
          @open-dir="emit('openDir', props.profile)"
          @repair="emit('repair', props.profile)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Timer,
  VideoPlay,
  Loading,
  Link,
  Check,
  Coin
} from '@element-plus/icons-vue';
import { Copy } from 'lucide-vue-next';
import type { Profile } from '../types';
import { useProfileStore } from '../store/profile';
import { formatSize } from '../utils/format';
import { macaronColor, textColorFor } from '../utils/macaron';
import { PROFILE_DRAG_TYPE, PROFILE_REORDER_TYPE } from '../utils/dnd';
import ProfileCardMenu from './ProfileCardMenu.vue';

const profileStore = useProfileStore();

const props = defineProps<{
  profile: Profile;
  isLaunching?: boolean;
  isSelected?: boolean;
  /** 所属工作区的颜色，用于卡片左侧色条 */
  workspaceColor?: string | null;
}>();

const emit = defineEmits<{
  click: [profile: Profile];
  launch: [profile: Profile];
  edit: [profile: Profile];
  backup: [profile: Profile];
  delete: [profile: Profile];
  openDir: [profile: Profile];
  performance: [profile: Profile];
  toggleSelection: [id: string];
  repair: [profile: Profile];
}>();

const { t, locale } = useI18n();

const tagList = computed(() => {
  return props.profile.tags?.split(',').map(t => t.trim()).filter(Boolean) || [];
});

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);
  
  if (minutes < 1) return t('profile.status.justNow');
  if (minutes < 60) return t('profile.status.minutesAgo', { n: minutes });
  if (hours < 24) return t('profile.status.hoursAgo', { n: hours });
  if (days < 30) return t('profile.status.daysAgo', { n: days });
  
  return date.toLocaleDateString(locale.value === 'zh' ? 'zh-CN' : 'en-US', { month: 'short', day: 'numeric' });
};

const isTemplate = computed(() => profileStore.templateIds.includes(props.profile.id));

/** 无自定义图标时：优先用配置自己选的颜色，否则按 id 取一个稳定的马卡龙色 */
const avatarColor = computed(() => {
  const custom = props.profile.icon_color;

  return custom ? { bg: custom, fg: textColorFor(custom) } : macaronColor(props.profile.id);
});

const avatarInitial = computed(() => (props.profile.name || '?').trim().slice(0, 1).toUpperCase());

const avatarStyle = computed(() =>
  props.profile.icon_base64
    ? undefined
    : { background: avatarColor.value.bg, color: avatarColor.value.fg }
);

/**
 * 拖拽时：若当前卡片属于已勾选集合，则带上全部勾选项，支持一次拖多个；
 * 否则只拖这一张。
 */
const handleDragStart = (event: DragEvent) => {
  if (!event.dataTransfer) return;

  const ids = props.isSelected
    ? Array.from(profileStore.selectedIds)
    : [props.profile.id];

  // 拖到侧边栏 = 移动到工作区；拖回网格内 = 重新排序，两者用不同 MIME 区分
  event.dataTransfer.setData(PROFILE_DRAG_TYPE, JSON.stringify(ids));
  event.dataTransfer.setData(PROFILE_REORDER_TYPE, props.profile.id);
  event.dataTransfer.effectAllowed = 'move';
};

const sizeLabel = computed(() => {
  const bytes = profileStore.sizeOf(props.profile.id);
  return bytes === undefined ? '' : formatSize(bytes);
});

const handleLaunch = async () => {
  if (props.isLaunching) {
    return;
  }
  emit('launch', props.profile);
};

</script>

<style scoped lang="scss">
// 飞书风格 ProfileCard
.profile-card {
  position: relative;
  background: var(--bg-primary);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-light);
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;

  &:hover {
    border-color: var(--primary-300);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);

    .status-bar {
      opacity: 1;
    }
  }

  &:active {
    transform: translateY(0);
    box-shadow: var(--shadow-sm);
  }

  // 运行中的配置用主色描边 + 常显顶条，比文字标签更省地方
  &.is-running {
    border-color: var(--primary-300);

    .status-bar {
      opacity: 1;
    }
  }

  // 启动中状态
  &.is-launching {
    pointer-events: none;
    opacity: 0.8;

    .launch-btn {
      background: var(--primary-50);
      color: var(--primary-500);
    }
  }

  // 选中状态
  &.is-selected {
    border-color: var(--primary-500);
    background: var(--primary-50);
    
    .status-bar {
      opacity: 1;
    }

    .selection-box {
      opacity: 1;
    }

    :global(.dark) & {
      background: rgba(12, 157, 248, 0.15); // Slightly brighter for dark mode selected state
    }
  }
}

// 选择框
.selection-box {
  position: absolute;
  top: 8px;
  left: 8px;
  width: 20px;
  height: 20px;
  z-index: 2;
  opacity: 0;
  transition: opacity 0.2s ease;

  .checkbox {
    width: 100%;
    height: 100%;
    border-radius: 4px;
    border: 1.5px solid var(--border-medium);
    background: var(--bg-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    font-size: 12px;
    transition: all 0.2s ease;

    &.checked {
      background: var(--primary-500);
      border-color: var(--primary-500);
    }
  }
}

.profile-card:hover .selection-box {
  opacity: 1;
}

// 顶部状态条
.status-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--primary-500);
  opacity: 0;
  transition: opacity 0.2s ease;
}

// 左侧色条：一眼看出配置属于哪个工作区
.ws-bar {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: 3px;
  z-index: 1;
}

// 模板标记改成中性描边胶囊：不再跟主按钮抢蓝色
.template-tag {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  background: transparent;
  border: 1px solid var(--bg-tertiary);
  color: var(--text-tertiary);
  font-weight: var(--font-medium);
}

// 状态标签
.status-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: var(--font-medium);
  
  &.running {
    color: var(--primary-600);
    
    .status-dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--primary-500);
      box-shadow: 0 0 4px var(--primary-300);
      animation: pulse 2s infinite;
    }
  }
}

@keyframes pulse {
  0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(var(--primary-500-rgb, 0, 100, 250), 0.4); }
  70% { transform: scale(1); box-shadow: 0 0 0 6px rgba(var(--primary-500-rgb, 0, 100, 250), 0); }
  100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(var(--primary-500-rgb, 0, 100, 250), 0); }
}

// 卡片内容
.profile-card-content {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-4);
}

// 头像包装器
.profile-avatar-wrapper {
  position: relative;
  flex-shrink: 0;
}

// 头像
.profile-avatar {
  width: 52px;
  height: 52px;
  border-radius: var(--radius-lg);
  overflow: hidden;
  background: var(--gray-100);
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-light);
  transition: all 0.2s ease;

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .avatar-icon {
    font-size: 24px;
    color: var(--gray-400);
  }
}

// 无自定义图标时的首字色块：底色是马卡龙浅色，首字用同色系深色（继承 profile-avatar 的 color）
.avatar-initial {
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  color: inherit;
  user-select: none;
}

// 信息区域
.profile-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

// 配置名称
.profile-name {
  margin: 0;
  font-size: var(--text-base);
  font-weight: var(--font-semibold);
  color: var(--text-primary);
  line-height: 1.4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

// 标签区域
.profile-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}

.profile-tag {
  font-size: 12px;
  font-weight: var(--font-medium);
  color: var(--primary-600);
  background: var(--primary-50);
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--primary-200);
  line-height: 1.4;
  transition: all 0.2s ease;
}

// 元信息
.profile-meta {
  font-size: 12px;
  color: var(--text-tertiary);
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-1);
}

.last-opened {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);

  .el-icon {
    font-size: 13px;
  }

  &.never {
    color: var(--text-quaternary);
  }
}

.size-info {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  color: var(--text-tertiary);

  .el-icon {
    font-size: 12px;
    cursor: help;
  }
}

.homepage-indicator {
  display: flex;
  align-items: center;
  margin-left: var(--space-1);
  color: var(--primary-400);
  
  .el-icon {
    font-size: 14px;
    cursor: help;
  }
}

// 操作区域
.profile-actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  align-items: center;
}

// 操作按钮基础样式
.action-btn {
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
  }

  &:active {
    transform: scale(0.95);
  }

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

// 启动按钮
.launch-btn {
  background: var(--primary-500);
  color: white;
  border-color: var(--primary-500);

  &:hover:not(:disabled) {
    background: var(--primary-600);
    border-color: var(--primary-600);
    color: white;
  }

  &:active:not(:disabled) {
    background: var(--primary-700);
  }

  &.is-loading {
    background: var(--primary-50);
    color: var(--primary-500);
    border-color: var(--primary-200);
  }

  .loading-icon {
    animation: spin 1s linear infinite;
  }
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

  .more-btn {
    &:hover {
      transform: rotate(90deg);
    }
  }

  .port-info {
    display: inline-flex;
    align-items: center;
    margin-left: 6px;
    color: var(--primary-600);
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    background: var(--primary-50);
    padding: 0 4px;
    border-radius: 4px;
    border: 1px solid var(--primary-200);
    
    .port-divider {
      margin: 0 4px;
      opacity: 0.5;
    }

    &.forward {
      background: var(--secondary-50);
      border-color: var(--secondary-200);
      color: var(--secondary-600);
      margin-left: 2px;
      
      .port-num {
        color: var(--secondary-700);
      }
    }
  }

</style>

<style lang="scss">
// 下拉菜单全局样式 (因为通过 teleport 挂载到 body)
.profile-dropdown-menu {
  padding: var(--space-2);
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-light) !important;
  box-shadow: var(--shadow-lg) !important;
  min-width: 140px;
  background-color: var(--bg-elevated) !important;

  .el-dropdown-menu__item {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    transition: all 0.15s ease;

    .el-icon {
      font-size: 14px;
      color: var(--text-tertiary);
    }

    &:hover {
      background: var(--bg-hover) !important;
      color: var(--text-primary);

      .el-icon {
        color: var(--text-secondary);
      }
    }

    &.danger-item {
      color: var(--danger-color);

      .el-icon {
        color: var(--danger-color);
      }

      &:hover {
        background: rgba(245, 63, 63, 0.1) !important;
        color: var(--danger-color);
        .el-icon {
          color: var(--danger-color);
        }
      }
    }
  }

  .el-dropdown-menu__item--divided {
    margin: var(--space-2) 0;
    border-top: 1px solid var(--border-light);

    &::before {
      display: none;
    }
  }
}
</style>

<style scoped lang="scss">
// 列表视图适配
:deep(.list-view) {
  .profile-card-content {
    padding: var(--space-3) var(--space-4);
  }

  .profile-avatar {
    width: 48px;
    height: 48px;
  }
}

// 响应式适配
@media (max-width: 640px) {
  .profile-card-content {
    padding: var(--space-3);
    gap: var(--space-2);
  }

  .profile-avatar {
    width: 44px;
    height: 44px;

    .avatar-icon {
      font-size: 20px;
    }
  }

  .profile-name {
    font-size: var(--text-sm);
  }

  .profile-tag {
    font-size: 11px;
    padding: 1px 6px;
  }

  .profile-meta {
    font-size: 11px;
  }

  .last-opened .el-icon {
    font-size: 12px;
  }

  .action-btn {
    width: 32px;
    height: 32px;

    .el-icon {
      font-size: 14px;
    }
  }
}

// 菜单内部元素（.move-trigger / .ws-dot）已随菜单一起迁入 ProfileCardMenu
:deep(.move-dropdown-menu) {
  max-height: 300px;
  overflow-y: auto;

  .el-dropdown-menu__item {
    display: flex;
    align-items: center;
    gap: 6px;
  }
}

</style>
