<template>
  <el-dialog
    v-model="visible"
    :title="isEdit ? $t('workspace.form.titleEdit') : $t('workspace.form.titleCreate')"
    width="440px"
    destroy-on-close
    append-to-body
  >
    <el-form :model="form" label-position="top" class="workspace-form">
      <el-form-item :label="$t('workspace.form.name')" required>
        <el-input
          v-model="form.name"
          :placeholder="$t('workspace.form.namePlaceholder')"
          maxlength="30"
          show-word-limit
        />
      </el-form-item>

      <div class="form-row">
        <el-form-item :label="$t('workspace.form.icon')" class="flex-1">
          <el-select v-model="form.icon" style="width: 100%">
            <el-option v-for="ic in ICONS" :key="ic" :label="ic" :value="ic" />
          </el-select>
        </el-form-item>

        <el-form-item :label="$t('workspace.form.color')" class="color-item">
          <el-color-picker v-model="form.color" />
        </el-form-item>
      </div>

      <el-form-item :label="$t('workspace.form.description')">
        <el-input
          v-model="form.description"
          type="textarea"
          :rows="2"
          maxlength="120"
          show-word-limit
          :placeholder="$t('workspace.form.descriptionPlaceholder')"
        />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">{{ $t('common.cancel') }}</el-button>
      <el-button type="primary" @click="handleSubmit" :loading="submitting">
        {{ isEdit ? $t('common.save') : $t('common.confirm') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { Workspace } from '../types';
import { useWorkspaceStore } from '../store/workspace';

const ICONS = ['📁', '💼', '🚀', '🎯', '🧪', '🎨', '🏢', '🌐', '🔬', '📊', '🛠️', '🧩'];

const props = defineProps<{
  modelValue: boolean;
  workspace?: Workspace | null;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
  success: [];
}>();

const { t } = useI18n();
const workspaceStore = useWorkspaceStore();

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
});

const isEdit = computed(() => !!props.workspace);
const submitting = ref(false);

const form = ref({
  name: '',
  icon: ICONS[0],
  color: '#6366f1',
  description: '',
});

watch(
  () => props.workspace,
  (workspace) => {
    if (workspace) {
      form.value = {
        name: workspace.name,
        icon: workspace.icon || ICONS[0],
        color: workspace.color || '#6366f1',
        description: workspace.description || '',
      };
    } else {
      form.value = {
        name: '',
        icon: ICONS[0],
        color: '#6366f1',
        description: '',
      };
    }
  },
  { immediate: true }
);

const handleSubmit = async () => {
  if (!form.value.name.trim()) {
    ElMessage.warning(t('workspace.messages.emptyName'));
    return;
  }

  submitting.value = true;

  try {
    const payload = {
      name: form.value.name.trim(),
      icon: form.value.icon || null,
      color: form.value.color || null,
      description: form.value.description.trim() || null,
    };

    if (isEdit.value && props.workspace) {
      await workspaceStore.updateWorkspace(props.workspace.id, payload);
    } else {
      await workspaceStore.createWorkspace(payload);
    }

    ElMessage.success(t('common.success'));
    emit('success');
    visible.value = false;
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  } finally {
    submitting.value = false;
  }
};
</script>

<style scoped lang="scss">
.workspace-form {
  .form-row {
    display: flex;
    gap: var(--space-4);
  }

  .flex-1 {
    flex: 1;
    min-width: 0;
  }

  .color-item {
    flex: 0 0 auto;
  }
}
</style>
