<template>
  <header class="app-header">
    <div class="header-left">
      <div class="brand">
        <div class="brand-icon">
          <img src="../../assets/logo.png" alt="SpaceShift logo" class="logo-image" />
        </div>
        <div class="brand-text">
          <h1 class="brand-title">SpaceShift</h1>
          <span class="brand-subtitle">{{ $t('profile.status.running') }}</span>
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
          <el-icon><i class="el-icon-language"></i><span class="lang-text">{{ currentLocale === 'zh' ? '中' : 'EN' }}</span></el-icon>
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

      <el-radio-group v-model="profileStore.viewMode" size="small" class="view-toggle">
        <el-tooltip :content="$t('main.gridView')" placement="bottom">
          <el-radio-button label="grid">
            <el-icon><Grid /></el-icon>
          </el-radio-button>
        </el-tooltip>
        <el-tooltip :content="$t('main.listView')" placement="bottom">
          <el-radio-button label="list">
            <el-icon><List /></el-icon>
          </el-radio-button>
        </el-tooltip>
      </el-radio-group>

      <el-button type="primary" @click="$emit('create')" class="create-btn">
        <el-icon><Plus /></el-icon>
        <span>{{ $t('header.newProfile') }}</span>
      </el-button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Search, Grid, List, Plus, Sunny, Moon, Refresh, Setting } from '@element-plus/icons-vue';
import { ElMessage } from 'element-plus';
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
  padding: 0 var(--space-5);
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
}

.brand {
  display: flex;
  align-items: center;
  gap: var(--space-3);

  &-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: 0 4px 12px rgba(59, 130, 246, 0.25);
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
    font-size: var(--text-base);
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
  flex: 1;
  max-width: 400px;
  margin: 0 var(--space-6);
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
  gap: var(--space-4);
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

  .lang-text {
    font-size: 11px;
    font-weight: var(--font-bold);
    font-family: var(--font-mono, monospace);
  }

  &:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  
  &:active {
    transform: scale(0.95);
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

.view-toggle {
  :deep(.el-radio-button__inner) {
    padding: 6px 12px;
    font-size: var(--text-sm);

    .el-icon {
      font-size: 16px;
    }
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

  .el-icon {
    font-size: 16px;
  }
}

@media (max-width: 768px) {
  .app-header {
    flex-wrap: wrap;
    height: auto;
    padding: var(--space-3);
    gap: var(--space-3);
  }

  .brand-subtitle {
    display: none;
  }

  .header-center {
    order: 3;
    width: 100%;
    max-width: none;
    margin: 0;
  }

  .header-right {
    margin-left: auto;
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
