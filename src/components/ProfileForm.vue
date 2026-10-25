<template>
  <el-dialog
    v-model="visible"
    :title="isEdit ? $t('profile.form.titleEdit') : $t('profile.form.titleCreate')"
    width="560px"
    destroy-on-close
    class="profile-form-dialog"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="80px"
      label-position="top"
      class="profile-form"
    >
      <el-form-item :label="$t('profile.form.name')" prop="name">
        <el-input
          v-model="form.name"
          :placeholder="$t('profile.form.namePlaceholder')"
          maxlength="50"
          show-word-limit
        />
      </el-form-item>

      <div class="form-row">
        <el-form-item :label="$t('profile.form.chromePath')" prop="chrome_path" class="flex-1">
          <el-select
            v-model="form.chrome_path"
            filterable
            allow-create
            clearable
            default-first-option
            :placeholder="$t('profile.form.chromePathPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="path in chromePathOptions"
              :key="path"
              :label="path"
              :value="path"
            />
          </el-select>
        </el-form-item>

        <el-form-item :label="$t('profile.form.homepage')" prop="homepage" class="flex-1">
          <el-select
            v-model="form.homepage"
            filterable
            allow-create
            clearable
            default-first-option
            :placeholder="$t('profile.form.homepagePlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="url in homepageOptions"
              :key="url"
              :label="url"
              :value="url"
            />
          </el-select>
        </el-form-item>
      </div>

      <div class="form-row">
        <el-form-item :label="$t('profile.form.debuggerMode')" class="toggle-item">
          <div class="toggle-row">
            <el-switch v-model="form.debugger_mode" />
          </div>
        </el-form-item>

        <el-form-item :label="$t('profile.form.disableExtensions')" class="toggle-item">
          <div class="toggle-row">
            <el-switch v-model="form.disable_extensions" />
          </div>
        </el-form-item>
      </div>

      <template v-if="form.debugger_mode">
        <div class="form-row">
          <el-form-item :label="$t('profile.form.debugPortConfig')" prop="debug_port_config" class="flex-1">
            <div class="port-row">
              <el-input-number
                v-model="form.debug_port_config"
                :placeholder="$t('profile.form.debugPortConfigPlaceholder')"
                :min="1"
                :max="65535"
                class="port-input"
              />

              <!-- 端口冲突是启动失败最常见的原因，给个一键找空闲端口 -->
              <el-tooltip :content="$t('profile.form.autoPort')" placement="top" :show-after="300">
                <button
                  type="button"
                  class="port-btn"
                  :disabled="pickingPort"
                  @click="assignFreePort"
                >
                  <el-icon><Aim /></el-icon>
                </button>
              </el-tooltip>
            </div>

            <div v-if="portStatus === 'used'" class="port-tip warn">
              {{ $t('profile.form.portInUse') }}
            </div>
            <div v-else-if="portStatus === 'free'" class="port-tip ok">
              {{ $t('profile.form.portFree') }}
            </div>
          </el-form-item>

          <el-form-item :label="$t('profile.status.forwardPort')" prop="forward_port" class="flex-1">
            <el-input-number
              v-model="form.forward_port"
              :placeholder="$t('profile.form.forwardPortPlaceholder')"
              :min="1"
              :max="65535"
              class="full-width"
            />
          </el-form-item>
        </div>
      </template>

      <div class="form-row">
        <el-form-item :label="$t('profile.form.icon')" class="icon-item">
          <div class="icon-selector">
            <div class="current-icon" @click="triggerFileInput">
              <img v-if="form.icon_base64" :src="form.icon_base64" alt="Icon" />
              <span v-else class="current-icon-fallback" :style="previewStyle">{{ previewInitial }}</span>
              <div class="icon-overlay">
                <el-icon><Camera /></el-icon>
              </div>
            </div>
            <input
              ref="fileInput"
              type="file"
              accept="image/*"
              style="display: none"
              @change="handleFileChange"
            />
          </div>
        </el-form-item>

        <el-form-item :label="$t('profile.form.tags')" class="flex-1">
          <el-select
            v-model="tagList"
            multiple
            filterable
            allow-create
            default-first-option
            :placeholder="$t('profile.form.tagsPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="tag in existingTags"
              :key="tag"
              :label="tag"
              :value="tag"
            />
          </el-select>
        </el-form-item>
      </div>

      <el-form-item :label="$t('profile.form.notes')" class="notes-item">
        <el-input
          v-model="form.notes"
          type="textarea"
          :rows="2"
          maxlength="500"
          show-word-limit
          :placeholder="$t('profile.form.notesPlaceholder')"
        />
      </el-form-item>

      <!-- 图标颜色单独占一行：色块够宽就不换行，也不会跟头像挤在一起 -->
      <el-form-item :label="$t('profile.form.iconColor')" class="icon-color-item">
        <div class="color-picker">
          <button
            v-for="color in macaronPalette"
            :key="color.bg"
            type="button"
            class="color-swatch"
            :class="{ active: form.icon_color === color.bg }"
            :style="{ background: color.bg }"
            @click="toggleColor(color.bg)"
          />
          <button
            type="button"
            class="color-swatch auto"
            :class="{ active: !form.icon_color }"
            :title="$t('profile.form.iconColorAuto')"
            @click="toggleColor('')"
          >
            A
          </button>
        </div>
      </el-form-item>

      <el-form-item v-if="!isEdit && templateProfiles.length > 0" :label="$t('profile.form.template')">
        <el-select
          v-model="form.template_id"
          clearable
          style="width: 100%"
          :placeholder="$t('profile.form.templatePlaceholder')"
          @change="handleTemplateChange"
        >
          <el-option
            v-for="tp in templateProfiles"
            :key="tp.id"
            :label="tp.name"
            :value="tp.id"
          />
        </el-select>
        <div class="form-tip">{{ $t('profile.form.templateTip') }}</div>
      </el-form-item>

      <el-form-item v-if="!isEdit && form.template_id" :label="$t('profile.form.templateMode')">
        <el-radio-group v-model="form.template_mode">
          <el-radio value="extensions">{{ $t('profile.form.templateModeExtensions') }}</el-radio>
          <el-radio value="full">{{ $t('profile.form.templateModeFull') }}</el-radio>
        </el-radio-group>
        <div class="form-tip">
          {{
            form.template_mode === 'full'
              ? $t('profile.form.templateModeFullTip')
              : $t('profile.form.templateModeExtensionsTip')
          }}
        </div>
      </el-form-item>

      <el-form-item class="toggle-item">
        <div class="toggle-row">
          <el-switch v-model="showLaunchArgs" />
          <span>{{ $t('profile.form.launchArgs') }}</span>
        </div>
        <div class="form-tip">{{ $t('profile.form.launchArgsTip') }}</div>
      </el-form-item>

      <template v-if="showLaunchArgs">
        <div class="form-row">
          <el-form-item :label="$t('profile.form.proxy')" class="flex-1">
            <div class="proxy-row">
              <el-input
                v-model="form.launch_args.proxy"
                :placeholder="$t('profile.form.proxyPlaceholder')"
                clearable
                class="proxy-input"
              />
              <el-button plain :loading="testingProxy" @click="handleTestProxy">
                {{ $t('profile.form.testProxy') }}
              </el-button>
            </div>

            <!-- 代理填错的话，这里直接看到出口 IP 或失败原因 -->
            <div v-if="proxyResult" class="proxy-result" :class="proxyResult.ok ? 'ok' : 'fail'">
              {{
                proxyResult.ok
                  ? $t('profile.form.proxyOk', { ip: proxyResult.ip, ms: proxyResult.elapsed_ms })
                  : $t('profile.form.proxyFail', { error: proxyResult.error })
              }}
            </div>
          </el-form-item>

          <el-form-item :label="$t('profile.form.language')" class="flex-1">
            <el-input
              v-model="form.launch_args.language"
              :placeholder="$t('profile.form.languagePlaceholder')"
              clearable
            />
          </el-form-item>
        </div>

        <div class="form-row">
          <el-form-item :label="$t('profile.form.windowSize')" class="flex-1">
            <el-input
              v-model="form.launch_args.windowSize"
              :placeholder="$t('profile.form.windowSizePlaceholder')"
              clearable
            />
          </el-form-item>

          <el-form-item :label="$t('profile.form.windowPosition')" class="flex-1">
            <el-input
              v-model="form.launch_args.windowPosition"
              :placeholder="$t('profile.form.windowPositionPlaceholder')"
              clearable
            />
          </el-form-item>
        </div>

        <el-form-item :label="$t('profile.form.userAgent')">
          <el-input
            v-model="form.launch_args.userAgent"
            :placeholder="$t('profile.form.userAgentPlaceholder')"
            clearable
          />
        </el-form-item>

        <el-form-item :label="$t('profile.form.extraArgs')">
          <el-input
            v-model="form.launch_args.extraArgs"
            :placeholder="$t('profile.form.extraArgsPlaceholder')"
            clearable
          />
          <div class="form-tip">{{ $t('profile.form.extraArgsTip') }}</div>
        </el-form-item>
      </template>

      <el-form-item :label="$t('profile.form.workspace')">
        <el-select
          v-model="form.workspace_id"
          :placeholder="$t('profile.form.workspacePlaceholder')"
          clearable
          style="width: 100%"
        >
          <el-option v-for="ws in workspaceStore.workspaces" :key="ws.id" :label="ws.name" :value="ws.id">
            <span class="ws-option">
              <span class="ws-dot" :style="{ background: ws.color || '#909399' }" />
              <span>{{ ws.icon || '📁' }}</span>
              <span>{{ ws.name }}</span>
            </span>
          </el-option>
        </el-select>
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">{{ $t('common.cancel') }}</el-button>
      <el-button type="primary" @click="handleSubmit" :loading="isSubmitting">
        {{ isEdit ? $t('common.save') : $t('common.confirm') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Camera, Aim } from '@element-plus/icons-vue';
import type { Profile, LaunchArgs, ProxyTestResult } from '../types';
import { api } from '../api';
import { MACARON_PALETTE, macaronColor, textColorFor } from '../utils/macaron';
import { useWorkspaceStore } from '../store/workspace';
import { useProfileStore } from '../store/profile';

const emptyLaunchArgs = (): LaunchArgs => ({
  proxy: '',
  userAgent: '',
  windowSize: '',
  windowPosition: '',
  language: '',
  extraArgs: ''
});

/** 后端可能返回缺字段的对象，补成空串避免 el-input 收到 undefined */
const mergeLaunchArgs = (args?: LaunchArgs | null): LaunchArgs => {
  const merged = emptyLaunchArgs();
  if (!args) return merged;

  (Object.keys(merged) as (keyof LaunchArgs)[]).forEach(key => {
    merged[key] = args[key] ?? '';
  });

  return merged;
};

const props = defineProps<{
  modelValue: boolean;
  profile?: Profile | null;
  existingTags: string[];
  defaultWorkspaceId?: string | null;
}>();

const workspaceStore = useWorkspaceStore();
const profileStore = useProfileStore();

/** 已配置过的 Chrome 路径，去重后供下拉选择（也允许手动输入新路径） */
const chromePathOptions = computed(() => {
  const set = new Set<string>();

  profileStore.profiles.forEach(p => {
    const path = p.chrome_path?.trim();
    if (path) set.add(path);
  });

  return Array.from(set).sort();
});

/** 已配置过的主页地址，去重后供下拉选择（也允许手动输入新地址） */
const homepageOptions = computed(() => {
  const set = new Set<string>();

  profileStore.profiles.forEach(p => {
    const url = p.homepage?.trim();
    if (url) set.add(url);
  });

  return Array.from(set).sort();
});

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
  success: [];
}>();

const { t } = useI18n();

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
});

const isEdit = computed(() => !!props.profile);

const formRef = ref();
const fileInput = ref<HTMLInputElement>();
const isSubmitting = ref(false);

const form = ref({
  name: '',
  chrome_path: '',
  homepage: '',
  icon_base64: '',
  icon_color: '',
  tags: '',
  notes: '',
  forward_port: undefined as number | undefined,
  disable_extensions: true,
  debugger_mode: true,
  debug_port_config: undefined as number | undefined,
  workspace_id: null as string | null,
  template_id: '' as string,
  template_mode: 'extensions' as string,
  launch_args: emptyLaunchArgs(),
});

/** 端口占用情况：unknown = 还没查 / 没填 */
const portStatus = ref<'unknown' | 'free' | 'used'>('unknown');
const pickingPort = ref(false);
let portTimer: number | undefined;

watch(
  () => form.value.debug_port_config,
  (port) => {
    portStatus.value = 'unknown';

    if (!port) return;

    // 输入过程中别每个数字都查一次
    window.clearTimeout(portTimer);
    portTimer = window.setTimeout(async () => {
      try {
        const status = await api.checkPort(port);
        portStatus.value = status.in_use ? 'used' : 'free';
      } catch {
        portStatus.value = 'unknown';
      }
    }, 350);
  }
);

const assignFreePort = async () => {
  pickingPort.value = true;

  try {
    const port = await api.findFreePort(form.value.debug_port_config || 9222);

    if (port) {
      form.value.debug_port_config = port;
      portStatus.value = 'free';
      ElMessage.success(t('profile.form.portAssigned', { port }));
    } else {
      ElMessage.warning(t('profile.form.noFreePort'));
    }
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  } finally {
    pickingPort.value = false;
  }
};

/** 代理自检：按当前填的代理发一次请求，把出口 IP 显示出来 */
const testingProxy = ref(false);
const proxyResult = ref<ProxyTestResult | null>(null);

const handleTestProxy = async () => {
  testingProxy.value = true;
  proxyResult.value = null;

  try {
    proxyResult.value = await api.testProxy(form.value.launch_args.proxy || '');

    if (proxyResult.value.ok) {
      ElMessage.success(t('profile.messages.proxyOk'));
    } else {
      ElMessage.error(t('profile.messages.proxyFail'));
    }
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  } finally {
    testingProxy.value = false;
  }
};

/** 启动参数区块默认收起，已有配置时才展开 */
const showLaunchArgs = ref(false);

const hasLaunchArgs = (args: LaunchArgs) =>
  Boolean(
    args.proxy || args.userAgent || args.windowSize || args.windowPosition || args.language || args.extraArgs
  );

/** 可选模板：由 store 中被标记的配置提供 */
const templateProfiles = computed(() => profileStore.templateProfiles);

/** 选中模板后必须放开扩展，否则复制过去的扩展不会加载 */
const handleTemplateChange = () => {
  if (form.value.template_id) {
    form.value.disable_extensions = false;
  }
};

const tagList = ref<string[]>([]);

const rules = {
  name: [
    { required: true, message: t('profile.form.namePlaceholder'), trigger: 'blur' },
    { min: 1, max: 50, message: t('profile.form.namePlaceholder'), trigger: 'blur' },
  ],
  homepage: [
    { 
      pattern: /^(https?:\/\/)?([\da-z.-]+)\.([a-z.]{2,6})([\/\w .-]*)*\/?$/, 
      message: t('profile.form.homepagePlaceholder'), 
      trigger: 'blur' 
    }
  ]
};

watch(
  () => props.profile,
  (profile) => {
    if (profile) {
      const launchArgs = mergeLaunchArgs(profile.launch_args);

      form.value = {
        name: profile.name,
        chrome_path: profile.chrome_path || '',
        homepage: profile.homepage || '',
        icon_base64: profile.icon_base64 || '',
        icon_color: profile.icon_color || '',
        tags: profile.tags || '',
        notes: profile.notes || '',
        forward_port: profile.forward_port,
        disable_extensions: profile.disable_extensions ?? true,
        debugger_mode: profile.debugger_mode ?? true,
        debug_port_config: profile.debug_port_config,
        workspace_id: profile.workspace_id ?? null,
        template_id: '',
        template_mode: 'extensions',
        launch_args: launchArgs,
      };
      showLaunchArgs.value = hasLaunchArgs(launchArgs);
      tagList.value = profile.tags?.split(',').map(t => t.trim()).filter(Boolean) || [];
    } else {
      form.value = {
        name: '',
        chrome_path: '',
        homepage: '',
        icon_base64: '',
        icon_color: '',
        tags: '',
        notes: '',
        forward_port: undefined,
        disable_extensions: true,
        debugger_mode: true,
        debug_port_config: undefined,
        workspace_id: props.defaultWorkspaceId ?? null,
        template_id: '',
        template_mode: 'extensions',
        launch_args: emptyLaunchArgs(),
      };
      showLaunchArgs.value = false;
      tagList.value = [];
    }
  },
  { immediate: true }
);

const triggerFileInput = () => {
  fileInput.value?.click();
};

/** 可选色板，与卡片默认取色用的是同一套 */
const macaronPalette = MACARON_PALETTE;

const toggleColor = (bg: string) => {
  form.value.icon_color = form.value.icon_color === bg ? '' : bg;
};

/** 预览色块：选了颜色就用选的，没选就按名字先算一个，和卡片上的效果一致 */
const previewColor = computed(() => {
  const custom = form.value.icon_color;

  return custom ? { bg: custom, fg: textColorFor(custom) } : macaronColor(form.value.name || 'preview');
});

const previewStyle = computed(() => ({
  background: previewColor.value.bg,
  color: previewColor.value.fg,
}));

const previewInitial = computed(() => (form.value.name || '?').trim().slice(0, 1).toUpperCase());

const handleFileChange = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0];
  
  if (file) {
    if (file.size > 2 * 1024 * 1024) {
      ElMessage.error(t('common.error'));
      return;
    }
    
    const reader = new FileReader();
    reader.onload = (e) => {
      form.value.icon_base64 = e.target?.result as string;
    };
    reader.readAsDataURL(file);
  }
  
  // Reset input
  target.value = '';
};

const handleSubmit = async () => {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;

  isSubmitting.value = true;
  
  try {
    const tags = tagList.value.join(',');
    
    if (isEdit.value && props.profile) {
      await api.updateProfile(
        props.profile.id,
        form.value.name,
        form.value.chrome_path || undefined,
        form.value.homepage || undefined,
        form.value.icon_base64 || undefined,
        // 空串表示清空，回到按 id 自动取色
        form.value.icon_color,
        tags || undefined,
        form.value.notes,
        form.value.forward_port ?? null,
        form.value.disable_extensions,
        form.value.debugger_mode,
        form.value.debug_port_config ?? null,
        form.value.launch_args
      );

      const targetWorkspace = form.value.workspace_id ?? null;
      if (targetWorkspace !== (props.profile.workspace_id ?? null)) {
        await api.moveProfileToWorkspace(props.profile.id, targetWorkspace);
      }

      ElMessage.success(t('common.success'));
    } else {
      await api.createProfile(
        form.value.name,
        form.value.chrome_path || undefined,
        form.value.homepage || undefined,
        form.value.icon_base64 || undefined,
        form.value.icon_color || undefined,
        tags || undefined,
        form.value.notes || undefined,
        form.value.forward_port || undefined,
        form.value.disable_extensions,
        form.value.debugger_mode,
        form.value.debug_port_config || undefined,
        form.value.workspace_id ?? null,
        form.value.template_id || undefined,
        form.value.template_id ? form.value.template_mode : undefined,
        form.value.launch_args
      );
      ElMessage.success(t('common.success'));
    }
    
    emit('success');
    visible.value = false;
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  } finally {
    isSubmitting.value = false;
  }
};
</script>

<style scoped lang="scss">
// 窄窗口下不要把弹窗顶出屏幕
.profile-form-dialog {
  max-width: 92vw;
}

:deep(.el-dialog__header) {
  background: linear-gradient(135deg, var(--primary-500) 0%, var(--secondary-500) 100%);
  padding: var(--space-3) var(--space-5);
  margin-right: 0;
  border-radius: var(--radius-2xl) var(--radius-2xl) 0 0;
}

:deep(.el-dialog__title) {
  color: white;
  font-weight: var(--font-semibold);
  font-size: var(--text-base);
}

:deep(.el-dialog__headerbtn) {
  top: 50%;
  transform: translateY(-50%);

  .el-dialog__close {
    color: white;
    font-size: 18px;

    &:hover {
      color: rgba(255, 255, 255, 0.8);
    }
  }
}

:deep(.el-dialog__body) {
  padding: var(--space-4) var(--space-5);
}

:deep(.el-dialog__footer) {
  padding: var(--space-3) var(--space-5);
  border-top: 1px solid var(--border-light);
}

// Form layout - compact
.profile-form {
  :deep(.el-form-item) {
    margin-bottom: var(--space-3);
  }

  :deep(.el-form-item__label) {
    font-weight: var(--font-medium);
    color: var(--text-secondary);
    font-size: var(--text-xs);
    padding-bottom: var(--space-1);
    line-height: 1.4;
  }

  :deep(.el-input__wrapper) {
    border-radius: var(--radius-lg);
    padding: var(--space-1) var(--space-3);

    &.is-focus {
      box-shadow: 0 0 0 3px rgba(33, 150, 243, 0.1);
    }
  }

  :deep(.el-input-number.full-width) {
    width: 100%;
  }
}

// Two-column row layout
.form-row {
  display: flex;
  gap: var(--space-3);
  align-items: flex-start;

  &.align-center {
    align-items: center;
  }

  .flex-1 {
    flex: 1;
    min-width: 0;
  }

  // Toggle items - keep label + switch inline, no stacking
  .toggle-item {
    flex: 1;

    :deep(.el-form-item__content) {
      display: flex;
      align-items: center;
      min-height: 32px;
    }
  }

  .toggle-row {
    display: flex;
    align-items: center;
    height: 32px;
  }

  .icon-item {
    flex-shrink: 0;
    margin-bottom: 0;

    :deep(.el-form-item__content) {
      line-height: 1;
    }
  }
}

.icon-selector {
  display: flex;
  justify-content: center;
}

.current-icon {
  position: relative;
  width: 64px;
  height: 64px;
  border-radius: var(--radius-full);
  cursor: pointer;
  overflow: hidden;
  border: 2px solid var(--border-light);
  transition: all var(--transition-fast);
  box-shadow: var(--shadow-sm);

  &:hover {
    border-color: var(--primary-400);
    transform: scale(1.05);
    box-shadow: var(--shadow-md);
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: var(--radius-full);
  }

  .current-icon-fallback {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    font-size: 26px;
    font-weight: var(--font-semibold);
  }
}

.icon-color-item {
  margin-top: var(--space-1);

  :deep(.el-form-item__content) {
    line-height: 1;
  }
}

.notes-item {
  :deep(.el-textarea__inner) {
    font-size: var(--text-sm);
  }
}

.port-row,
.proxy-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
}

.port-input,
.proxy-input {
  flex: 1;
  min-width: 0;
}

.port-btn {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-medium);
  background: var(--bg-primary);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);

  &:hover:not(:disabled) {
    border-color: var(--primary-400);
    color: var(--primary-600);
    background: var(--primary-50);
  }

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

.port-tip,
.proxy-result {
  margin-top: 4px;
  font-size: var(--text-xs);
  line-height: 1.4;
  word-break: break-all;

  &.warn,
  &.fail {
    color: var(--danger-600);
  }

  &.ok {
    color: var(--success-600);
  }
}

.color-picker {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}

.color-swatch {
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-full);
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease;

  &:hover {
    transform: scale(1.12);
  }

  // 选中：底色留一圈缝隙，再加一层主色环，避免和色块本身糊在一起
  &.active {
    border-color: transparent;
    box-shadow: 0 0 0 2px var(--bg-primary), 0 0 0 4px var(--primary-400);
  }

  // 「自动」：不指定颜色，按配置 id 取色
  &.auto {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: var(--font-semibold);
    color: var(--text-tertiary);
    background: var(--bg-secondary);
    border-style: dashed;
  }
}

.icon-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(0, 0, 0, 0.6), rgba(0, 0, 0, 0.4));
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  opacity: 0;
  transition: all var(--transition-fast);
  border-radius: var(--radius-full);

  .el-icon {
    font-size: 20px;
  }
}

.current-icon:hover .icon-overlay {
  opacity: 1;
}

.form-tip {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  margin-top: var(--space-1);
  line-height: 1.4;
}

.ws-option {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.ws-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}
</style>
