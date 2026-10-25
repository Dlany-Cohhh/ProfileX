<template>
  <header class="app-header" @mousedown="startDrag">
    <!-- 无边框窗口：整条标题栏都能拖动，交互元素除外 -->
    <div class="header-left">
      <div class="brand">
        <div class="brand-icon">
          <img src="../../assets/logo.png" alt="SpaceShift logo" class="logo-image" />
        </div>
        <div class="brand-text">
          <h1 class="brand-title">SpaceShift</h1>
          <span class="brand-subtitle">
            <span v-if="runningCount > 0" class="live-dot" />
            {{ $t('header.stats', { running: runningCount, total: totalCount }) }}
          </span>
        </div>
      </div>
    </div>

    <div class="header-center">
      <el-input
        v-model="profileStore.searchQuery"
        :placeholder="$t('header.search')"
        class="search-input"
        clearable
      >
        <template #prefix>
          <el-icon><Search /></el-icon>
        </template>
      </el-input>
    </div>

    <div class="header-right">
      <!-- 语言切换 -->
      <el-dropdown trigger="click" @command="handleLanguageChange">
        <button class="icon-btn">
          <span class="lang-text">{{ currentLocale === 'zh' ? '中' : 'EN' }}</span>
        </button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="zh" :disabled="currentLocale === 'zh'">简体中文</el-dropdown-item>
            <el-dropdown-item command="en" :disabled="currentLocale === 'en'">English</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>

      <el-tooltip :content="$t('header.refresh')" placement="bottom" :show-after="300">
        <button class="icon-btn" :class="{ spinning: refreshing }" @click="handleRefresh">
          <el-icon><Refresh /></el-icon>
        </button>
      </el-tooltip>

      <el-tooltip :content="$t('header.settings')" placement="bottom" :show-after="300">
        <button class="icon-btn" @click="$emit('settings')">
          <el-icon><Setting /></el-icon>
        </button>
      </el-tooltip>

      <el-tooltip :content="themeStore.isDark ? $t('header.theme.light') : $t('header.theme.dark')" placement="bottom" :show-after="300">
        <button class="icon-btn" @click="themeStore.toggleTheme">
          <el-icon>
            <Sunny v-if="themeStore.isDark" />
            <Moon v-else />
          </el-icon>
        </button>
      </el-tooltip>

      <span class="divider" />

      <!-- 视图切换：自绘分段控件，比 el-radio-group 窄一半 -->
      <div class="view-toggle">
        <el-tooltip :content="$t('main.gridView')" placement="bottom" :show-after="300">
          <button
            class="view-btn"
            :class="{ active: profileStore.viewMode === 'grid' }"
            @click="profileStore.viewMode = 'grid'"
          >
            <el-icon><Grid /></el-icon>
          </button>
        </el-tooltip>
        <el-tooltip :content="$t('main.listView')" placement="bottom" :show-after="300">
          <button
            class="view-btn"
            :class="{ active: profileStore.viewMode === 'list' }"
            @click="profileStore.viewMode = 'list'"
          >
            <el-icon><List /></el-icon>
          </button>
        </el-tooltip>
      </div>

      <el-button type="primary" @click="$emit('create')" class="create-btn">
        <el-icon><Plus /></el-icon>
        <span class="create-text">{{ $t('header.newProfile') }}</span>
      </el-button>
    </div>

    <!-- 窗口控制按钮绝对定位在右上角，不参与伸缩，永远不会被内容挤出可视区 -->
    <div class="window-controls">
      <button class="win-btn" :title="$t('header.window.minimize')" @click="runAction('minimize')">
        <svg viewBox="0 0 10 10" width="10" height="10"><path d="M0 5h10" stroke="currentColor" stroke-width="1" /></svg>
      </button>
      <button class="win-btn" :title="$t('header.window.maximize')" @click="runAction('maximize')">
        <svg viewBox="0 0 10 10" width="10" height="10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1" /></svg>
      </button>
      <button class="win-btn close" :title="$t('header.window.close')" @click="runAction('close')">
        <svg viewBox="0 0 10 10" width="10" height="10"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1" /></svg>
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Search, Grid, List, Plus, Sunny, Moon, Refresh, Setting } from '@element-plus/icons-vue';
import { ElMessage } from 'element-plus';
import { api } from '../../api';
import { useProfileStore } from '../../store/profile';
import { useWorkspaceStore } from '../../store/workspace';
import { useThemeStore } from '../../store/theme';

defineEmits(['create', 'settings']);
const profileStore = useProfileStore();
const workspaceStore = useWorkspaceStore();
const themeStore = useThemeStore();
const { t, locale } = useI18n();

const currentLocale = computed(() => locale.value);
const refreshing = ref(false);

/** 无边框窗口：自己提供最小化 / 最大化 / 关闭 */
const runAction = async (action: 'minimize' | 'maximize' | 'close' | 'drag') => {
  try {
    await api.windowAction(action);
  } catch (error) {
    console.error(error);
  }
};

/**
 * 拖动窗口。落在按钮、输入框等交互元素上时不触发，
 * 否则会连搜索框里选文字都选不了
 */
const startDrag = (event: MouseEvent) => {
  const target = event.target as HTMLElement | null;

  if (target?.closest('button, input, textarea, select, label, a, .el-input, .view-toggle')) {
    return;
  }

  runAction('drag');
};

const totalCount = computed(() => profileStore.profiles.length);
const runningCount = computed(() => profileStore.profiles.filter(p => p.is_running).length);

const handleLanguageChange = (lang: string) => {
  locale.value = lang;
};

/** 一键刷新：重新拉取配置、工作区，并重算磁盘占用 */
const handleRefresh = async () => {
  if (refreshing.value) return;

  refreshing.value = true;
  try {
    await Promise.all([profileStore.loadProfiles(), workspaceStore.loadWorkspaces()]);
    profileStore.loadSizes();
    ElMessage.success(t('header.refreshed'));
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  } finally {
    // 旋转动画至少转一圈，避免瞬时完成看着像没反应
    setTimeout(() => {
      refreshing.value = false;
    }, 400);
  }
};
</script>

<style lang="scss" scoped>
// ── 标题栏：52px 单行，右侧固定预留 96px 给窗口按钮 ──
.app-header {
  position: sticky;
  top: 0;
  z-index: var(--z-sticky);
  display: flex;
  align-items: center;
  gap: var(--space-3);
  height: 52px;
  padding: 0 100px 0 var(--space-4);
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-light);
  user-select: none;
}

.header-left {
  display: flex;
  align-items: center;
  flex: 0 0 auto;
}

// ── 品牌区 ──
.brand {
  display: flex;
  align-items: center;
  gap: 10px;

  &-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.28);
    transition: transform var(--transition-fast), box-shadow var(--transition-fast);

    &:hover {
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(59, 130, 246, 0.36);
    }

    .logo-image {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: contain;
    }
  }

  &-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    line-height: 1.15;
  }

  &-title {
    margin: 0;
    font-size: 14px;
    font-weight: var(--font-semibold);
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }

  &-subtitle {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
  }
}

// 有配置在跑时的小绿点，替代原来干巴巴的数字
.live-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--success-500, #22c55e);
  animation: live-pulse 2s ease-out infinite;
}

@keyframes live-pulse {
  0% {
    box-shadow: 0 0 0 0 rgba(0, 181, 120, 0.45);
  }
  70% {
    box-shadow: 0 0 0 5px rgba(0, 181, 120, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(0, 181, 120, 0);
  }
}

// ── 搜索框：唯一可伸缩的区域，空间不够时先压它 ──
.header-center {
  flex: 1 1 0;
  min-width: 0;
  max-width: 420px;
  // 左右 auto：搜索框落在中间的空白里，两侧留白对称
  margin: 0 auto;
}

.search-input {
  width: 100%;

  :deep(.el-input__wrapper) {
    height: 30px;
    padding: 0 10px;
    border-radius: 8px;
    background: var(--bg-secondary);
    border: 1px solid transparent;
    box-shadow: none;
    transition: background var(--transition-fast), border-color var(--transition-fast),
      box-shadow var(--transition-fast);

    &:hover {
      background: var(--bg-tertiary);
    }

    &.is-focus {
      background: var(--bg-primary);
      border-color: var(--primary-400);
      box-shadow: 0 0 0 3px var(--primary-100);
    }
  }

  :deep(.el-input__inner) {
    font-size: 13px;
    color: var(--text-primary);

    &::placeholder {
      color: var(--text-tertiary);
    }
  }

  :deep(.el-input__prefix) {
    margin-right: 6px;
    color: var(--text-tertiary);

    .el-icon {
      font-size: 14px;
    }
  }

  :deep(.el-input__suffix) {
    color: var(--text-tertiary);
  }
}

// ── 右侧操作区 ──
.header-right {
  display: flex;
  align-items: center;
  gap: 2px;
  flex: 0 0 auto;
}

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast),
    transform var(--transition-fast);

  .el-icon {
    font-size: 15px;
  }

  .lang-text {
    font-size: 11px;
    font-weight: var(--font-bold);
    letter-spacing: 0.02em;
  }

  &:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  &:active {
    transform: scale(0.94);
  }

  &.spinning .el-icon {
    animation: icon-spin 0.8s linear infinite;
  }
}

@keyframes icon-spin {
  to {
    transform: rotate(360deg);
  }
}

.divider {
  flex: 0 0 auto;
  width: 1px;
  height: 18px;
  margin: 0 8px;
  background: var(--bg-tertiary);
}

// ── 视图切换：分段控件 ──
.view-toggle {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  border-radius: 8px;
  background: var(--bg-secondary);
}

.view-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast),
    box-shadow var(--transition-fast);

  .el-icon {
    font-size: 14px;
  }

  &:hover {
    color: var(--text-primary);
  }

  &.active {
    background: var(--bg-primary);
    color: var(--primary-500);
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.12);
  }
}

.create-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 30px;
  margin-left: 6px;
  padding: 0 12px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: var(--font-medium);
  box-shadow: 0 1px 2px rgba(59, 130, 246, 0.25);
  transition: box-shadow var(--transition-fast), transform var(--transition-fast);

  .el-icon {
    font-size: 14px;
  }

  &:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.32);
  }

  &:active {
    transform: none;
  }
}

// ── 窗口控制按钮：贴右上角，整块高度可点 ──
.window-controls {
  position: absolute;
  top: 0;
  right: 0;
  z-index: 2;
  display: flex;
  align-items: stretch;
  height: 100%;
  border-left: 1px solid var(--border-light);
}

.win-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;

  &:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  &:active {
    background: var(--bg-tertiary);
  }

  // 关闭按钮沿用系统习惯的红色
  &.close:hover {
    background: #e81123;
    color: #fff;
  }

  &.close:active {
    background: #f1707a;
    color: #fff;
  }
}

// ── 窄窗口逐级让位：先收搜索框，再藏文字和次要控件 ──
@media (max-width: 1040px) {
  .header-center {
    max-width: 200px;
  }

  .create-text {
    display: none;
  }

  .create-btn {
    padding: 0 9px;
  }
}

@media (max-width: 900px) {
  .view-toggle,
  .divider {
    display: none;
  }
}

@media (max-width: 760px) {
  .brand-text {
    display: none;
  }
}
</style>
