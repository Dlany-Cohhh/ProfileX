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
          <Search :size="15" :stroke-width="1.75" />
        </template>
      </el-input>
    </div>

    <div class="header-right">
      <!-- 同组图标靠得更紧，与视图切换、主按钮拉开层级 -->
      <div class="icon-group">
        <!-- 语言切换 -->
        <el-dropdown trigger="click" @command="handleLanguageChange">
          <button class="icon-btn">
            <Languages :size="16" :stroke-width="1.75" />
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
            <RotateCw :size="16" :stroke-width="1.75" />
          </button>
        </el-tooltip>

        <el-tooltip :content="$t('header.settings')" placement="bottom" :show-after="300">
          <button class="icon-btn" @click="$emit('settings')">
            <Settings2 :size="16" :stroke-width="1.75" />
          </button>
        </el-tooltip>

        <el-tooltip :content="themeStore.isDark ? $t('header.theme.light') : $t('header.theme.dark')" placement="bottom" :show-after="300">
          <button class="icon-btn" @click="themeStore.toggleTheme">
            <Moon v-if="themeStore.isDark" :size="16" :stroke-width="1.75" />
            <Sun v-else :size="16" :stroke-width="1.75" />
          </button>
        </el-tooltip>
      </div>

      <el-button type="primary" @click="$emit('create')" class="create-btn">
        <Plus :size="16" :stroke-width="2" />
        <span>{{ $t('header.newProfile') }}</span>
      </el-button>
    </div>

    <!-- 无边框窗口的自绘控制按钮：固定在右上角，不参与右侧那一排的伸缩 -->
    <div class="window-controls">
      <button class="win-btn" :title="$t('header.window.minimize')" @click="runAction('minimize')">
        <Minus :size="12" :stroke-width="1" />
      </button>
      <button class="win-btn" :title="$t('header.window.maximize')" @click="runAction('maximize')">
        <Square :size="11" :stroke-width="1" />
      </button>
      <button class="win-btn close" :title="$t('header.window.close')" @click="runAction('close')">
        <X :size="13" :stroke-width="1" />
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Search,
  Languages,
  RotateCw,
  Settings2,
  Moon,
  Sun,
  Plus,
  Minus,
  Square,
  X,
} from 'lucide-vue-next';
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

  if (target?.closest('button, input, textarea, select, label, a, .el-input, .el-radio-group')) {
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
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  // 右边留给窗口控制按钮，它们不参与伸缩，不会被内容挤出可视区
  padding: 0 100px 0 var(--space-5);
  height: 60px;
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-light);
  position: sticky;
  top: 0;
  z-index: var(--z-sticky);
  box-shadow: var(--shadow-xs);
}

.header-left {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: var(--space-3);

  &-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.22);
    transition: all 0.3s ease;

    &:hover {
      transform: scale(1.05);
      box-shadow: 0 6px 16px rgba(59, 130, 246, 0.35);
    }

    .logo-image {
      width: 100%;
      height: 100%;
      object-fit: contain;
    }
  }

  &-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  &-title {
    font-size: 14px;
    font-weight: var(--font-semibold);
    color: var(--text-primary);
    line-height: 1.2;
    margin: 0;
  }

  &-subtitle {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    line-height: 1.2;
  }
}

.header-center {
  // 不抢空间：配合 outer 的 space-between，搜索框落在品牌区与操作区之间，
  // 窄窗口时优先压缩它
  flex: 0 1 480px;
  min-width: 220px;
  max-width: 480px;
  margin: 0 var(--space-4);
}

.search-input {
  width: 100%;

  :deep(.el-input__wrapper) {
    border-radius: var(--radius-md);
    background: var(--bg-secondary);
    border: 1px solid transparent;
    box-shadow: none;
    transition: all var(--transition-fast);

    &:hover {
      background: var(--bg-tertiary);
    }

    &.is-focus {
      background: var(--bg-primary);
      border-color: var(--primary-400);
      box-shadow: 0 0 0 2px var(--primary-100);
    }
  }

  :deep(.el-input__inner) {
    font-size: var(--text-sm);
    color: var(--text-primary);

    &::placeholder {
      color: var(--text-tertiary);
    }
  }

  :deep(.el-input__prefix) {
    color: var(--text-tertiary);
    margin-right: var(--space-2);
  }
}

.header-right {
  display: flex;
  align-items: center;
  // 组与组之间 16px
  gap: var(--space-4);
  // 窗口控制按钮在最右侧，绝不能被挤到窗口外面去
  flex-shrink: 0;
}

// 同组图标之间留白，避免看着挤成一团
.icon-group {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: 50%;
  transition: all 0.2s ease;

  // Lucide 渲染的是 svg，确保它不吃到行高
  svg {
    display: block;
  }

  &:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  &:active {
    transform: scale(0.95);
  }

  &.spinning svg {
    animation: icon-spin 0.8s linear infinite;
  }
}

@keyframes icon-spin {
  to {
    transform: rotate(360deg);
  }
}

.create-btn {
  height: 34px;
  padding: 0 16px;
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  display: flex;
  align-items: center;
  gap: var(--space-1);

  svg {
    display: block;
  }
}

// 自绘窗口控制按钮
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
  height: 100%;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);

  &:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  &.close:hover {
    background: var(--danger-500);
    color: #fff;
  }
}

// 窗口变窄时先让位给窗口控制按钮
@media (max-width: 1024px) {
  .header-center {
    max-width: 200px;
  }
}

@media (max-width: 768px) {
  // 不换行：右侧 100px 是窗口按钮的位置，换行后会被它们压住
  .app-header {
    padding-right: 100px;
    gap: var(--space-2);
  }

  .brand-subtitle {
    display: none;
  }

  .header-center {
    margin: 0 var(--space-2);
  }
}

@media (max-width: 480px) {
  .brand-icon {
    width: 32px;
    height: 32px;
    font-size: 18px;
  }

  .brand-title {
    font-size: var(--text-sm);
  }

  .create-btn span {
    display: none;
  }

  .create-btn {
    padding: 0 10px;
  }
}
</style>
