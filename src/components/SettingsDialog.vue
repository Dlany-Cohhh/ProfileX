<template>
  <el-dialog
    :model-value="modelValue"
    :title="$t('settings.title')"
    width="620px"
    append-to-body
    destroy-on-close
    @update:model-value="$emit('update:modelValue', $event)"
  >
    <div v-loading="loading" class="settings-container">
      <el-alert type="info" :closable="false" show-icon class="settings-tip">
        {{ $t('settings.migrateTip') }}
      </el-alert>

      <el-form label-position="top">
        <el-form-item :label="$t('settings.dataDir')">
          <div class="path-row">
            <el-input v-model="dataDir" :placeholder="$t('settings.dataDirPlaceholder')" :disabled="saving" />
            <el-button :disabled="!dataDir || saving" @click="handleOpen(dataDir)">
              {{ $t('profile.actions.openDir') }}
            </el-button>
          </div>
          <div class="form-tip">{{ $t('settings.currentPath', { path: current.profiles_dir }) }}</div>
        </el-form-item>

        <el-form-item :label="$t('settings.backupDir')">
          <div class="path-row">
            <el-input v-model="backupDir" :placeholder="$t('settings.backupDirPlaceholder')" :disabled="saving" />
            <el-button :disabled="!backupDir || saving" @click="handleOpen(backupDir)">
              {{ $t('profile.actions.openDir') }}
            </el-button>
          </div>
          <div class="form-tip">{{ $t('settings.currentPath', { path: current.backup_dir }) }}</div>
        </el-form-item>

        <el-form-item :label="$t('settings.backupRetention')">
          <el-input-number v-model="retention" :min="0" :max="99" />
          <div class="form-tip">{{ $t('settings.backupRetentionTip') }}</div>
        </el-form-item>

        <el-form-item :label="$t('settings.accent')">
          <div class="accent-row">
            <button
              v-for="preset in ACCENT_PRESETS"
              :key="preset.color"
              type="button"
              class="accent-swatch"
              :class="{ active: isAccent(preset.color) }"
              :style="{ background: preset.color, '--swatch': preset.color }"
              :title="$t(`settings.accentColors.${preset.key}`)"
              @click="themeStore.setAccent(preset.color)"
            >
              <el-icon v-if="isAccent(preset.color)"><Check /></el-icon>
            </button>

            <el-color-picker v-model="customAccent" class="accent-custom" @change="themeStore.setAccent" />
          </div>
          <div class="form-tip">{{ $t('settings.accentTip') }}</div>
        </el-form-item>

        <div v-if="migrating" class="migrate-progress">
          <el-progress :percentage="progressPercent" :stroke-width="14" striped striped-flow />
          <div class="form-tip">
            {{ $t('settings.migrating', { done: progressCurrent, total: progressTotal }) }}
            <template v-if="progressLabel"> · {{ progressLabel }}</template>
          </div>
        </div>

        <el-form-item :label="$t('settings.appDataDir')">
          <el-input :model-value="current.app_data_dir" readonly />
          <div class="form-tip">{{ $t('settings.appDataDirTip') }}</div>
        </el-form-item>
      </el-form>
    </div>

    <template #footer>
      <el-button @click="$emit('update:modelValue', false)">{{ $t('common.cancel') }}</el-button>
      <el-button type="primary" :loading="saving" @click="handleSave">
        {{ $t('common.save') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Check } from '@element-plus/icons-vue';
import { listen } from '@tauri-apps/api/event';
import { api } from '../api';
import { useThemeStore } from '../store/theme';
import { ACCENT_PRESETS } from '../utils/accent';
import type { PathsInfo } from '../types';

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits(['update:modelValue', 'changed']);

const { t } = useI18n();
const themeStore = useThemeStore();

/** 自定义取色器的值，跟着 store 走，点预设时也会同步显示 */
const customAccent = ref(themeStore.accent);
watch(
  () => themeStore.accent,
  (val) => {
    customAccent.value = val;
  }
);

/** 区分大小写地比对当前主题色 */
const isAccent = (color: string) => themeStore.accent.toLowerCase() === color.toLowerCase();

const loading = ref(false);
const saving = ref(false);
const dataDir = ref('');
const backupDir = ref('');

/** 搬迁进度：后端通过 migration-progress 事件逐个目录上报 */
const migrating = ref(false);
const progressCurrent = ref(0);
const progressTotal = ref(0);
const progressLabel = ref('');

const progressPercent = computed(() =>
  progressTotal.value === 0 ? 0 : Math.round((progressCurrent.value / progressTotal.value) * 100)
);

interface MigrationProgressPayload {
  current: number;
  total: number;
  label: string;
}

const current = reactive<PathsInfo>({
  app_data_dir: '',
  profiles_dir: '',
  backup_dir: '',
  using_custom_data_dir: false,
  using_custom_backup_dir: false,
  backup_retention: 0,
});

/** 备份保留数量，0 = 不限制 */
const retention = ref(0);

const loadPaths = async () => {
  loading.value = true;
  try {
    const info = await api.getPaths();
    Object.assign(current, info);
    dataDir.value = info.profiles_dir;
    backupDir.value = info.backup_dir;
    retention.value = info.backup_retention;
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  } finally {
    loading.value = false;
  }
};

watch(
  () => props.modelValue,
  (visible) => {
    if (visible) loadPaths();
  }
);

const handleOpen = async (path: string) => {
  try {
    await api.openProfileDirectory(path);
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  }
};

const handleSave = async () => {
  saving.value = true;
  let changed = false;
  let unlisten: (() => void) | undefined;

  try {
    const nextDataDir = dataDir.value.trim();
    const nextBackupDir = backupDir.value.trim();

    const needsDataMove = Boolean(nextDataDir) && nextDataDir !== current.profiles_dir;
    const needsBackupMove = Boolean(nextBackupDir) && nextBackupDir !== current.backup_dir;
    const needsRetentionChange = retention.value !== current.backup_retention;

    if (needsDataMove || needsBackupMove) {
      migrating.value = true;
      progressCurrent.value = 0;
      progressTotal.value = 0;
      progressLabel.value = '';

      unlisten = await listen<MigrationProgressPayload>('migration-progress', (event) => {
        progressCurrent.value = event.payload.current;
        progressTotal.value = event.payload.total;
        progressLabel.value = event.payload.label;
      });
    }

    if (needsDataMove) {
      const info = await api.setDataDir(nextDataDir);
      Object.assign(current, info);
      dataDir.value = info.profiles_dir;
      changed = true;
    }

    if (needsBackupMove) {
      const info = await api.setBackupDir(nextBackupDir);
      Object.assign(current, info);
      backupDir.value = info.backup_dir;
      changed = true;
    }

    if (needsRetentionChange) {
      const info = await api.setBackupRetention(retention.value);
      Object.assign(current, info);
      retention.value = info.backup_retention;
      changed = true;
    }

    if (changed) {
      ElMessage.success(t('settings.saved'));
      emit('changed');
    }

    emit('update:modelValue', false);
  } catch (error) {
    // 后端会以字符串抛出具体原因（如仍有配置在运行）
    ElMessage.error(typeof error === 'string' ? error : t('common.error'));
    console.error(error);
  } finally {
    unlisten?.();
    migrating.value = false;
    saving.value = false;
  }
};
</script>

<style scoped lang="scss">
.settings-container {
  min-height: 220px;
}

.settings-tip {
  margin-bottom: var(--space-4);
}

.migrate-progress {
  margin-bottom: var(--space-4);
}

.path-row {
  display: flex;
  gap: var(--space-2);
  width: 100%;
}

.accent-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.accent-swatch {
  width: 26px;
  height: 26px;
  padding: 0;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: var(--radius-full);
  cursor: pointer;
  color: #fff;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.15s ease, box-shadow 0.15s ease;

  &:hover {
    transform: scale(1.12);
  }

  // 选中环用色块自己的颜色，浅色背景下也能看清
  &.active {
    box-shadow: 0 0 0 2px var(--bg-primary), 0 0 0 4px var(--swatch);
  }
}

.accent-custom {
  margin-left: var(--space-1);
}

.form-tip {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  margin-top: var(--space-1);
  line-height: 1.4;
  word-break: break-all;
}
</style>
