<template>
  <main class="app-main" :class="{ 'has-batch-bar': profileStore.selectedIds.size > 0 }">
    <!-- 骨架屏用与网格同构的卡片，避免加载完成时布局跳动 -->
    <div
      v-if="profileStore.loading"
      :class="['profiles-container', profileStore.viewMode === 'grid' ? 'grid-view' : 'list-view']"
    >
      <div v-for="n in 6" :key="n" class="skeleton-card">
        <el-skeleton-item variant="image" class="skeleton-avatar" />
        <div class="skeleton-lines">
          <el-skeleton-item variant="p" class="skeleton-line w60" />
          <el-skeleton-item variant="text" class="skeleton-line w40" />
        </div>
      </div>
    </div>

    <template v-else>
      <!-- 只要库里还有配置就保留工具栏：筛选把列表筛空时，条件还能在这里改回来 -->
      <div v-if="profileStore.profiles.length > 0" class="list-toolbar">
        <el-checkbox
          :model-value="allSelected"
          :indeterminate="partiallySelected"
          @change="profileStore.toggleSelectAll(visibleIds)"
        >
          {{ $t('main.selectAll') }}
        </el-checkbox>

        <!-- 右侧一组：全部关闭 + 排序筛选 + 视图切换 + 导入 + 新建 -->
        <div class="toolbar-actions">
          <el-button
            v-if="profileStore.runningCount > 0"
            class="close-all-btn"
            plain
            :loading="closingAll"
            @click="handleCloseAll"
          >
            <el-icon><SwitchButton /></el-icon>
            {{ $t('main.closeAll', { n: profileStore.runningCount }) }}
          </el-button>

          <!-- 排序 / 筛选：胶囊外壳 + 无边框下拉，紧贴视图切换左边 -->
          <div class="filter-group">
            <div class="filter-chip" :class="{ 'is-active': profileStore.sortBy !== 'custom' }">
              <ArrowUpDown :size="13" :stroke-width="1.75" class="chip-icon" />
              <el-select
                v-model="profileStore.sortBy"
                class="chip-select"
                size="small"
                popper-class="app-filter-popper"
              >
                <el-option
                  v-for="opt in sortOptions"
                  :key="opt.value"
                  :label="opt.label"
                  :value="opt.value"
                />
              </el-select>
            </div>

            <!-- 手动顺序下没有升降序的概念，直接不显示 -->
            <el-tooltip
              v-if="profileStore.sortBy !== 'custom'"
              :content="profileStore.sortDir === 'asc' ? $t('main.sortAsc') : $t('main.sortDesc')"
              placement="bottom"
              :show-after="300"
            >
              <button class="chip-btn" @click="toggleSortDir">
                <ArrowUp v-if="profileStore.sortDir === 'asc'" :size="14" :stroke-width="2" />
                <ArrowDown v-else :size="14" :stroke-width="2" />
              </button>
            </el-tooltip>

            <div class="filter-chip" :class="{ 'is-active': profileStore.statusFilter !== 'all' }">
              <Activity :size="13" :stroke-width="1.75" class="chip-icon" />
              <el-select
                v-model="profileStore.statusFilter"
                class="chip-select"
                size="small"
                popper-class="app-filter-popper"
              >
                <el-option
                  v-for="opt in statusOptions"
                  :key="opt.value"
                  :label="opt.label"
                  :value="opt.value"
                />
              </el-select>
            </div>

            <div class="filter-chip" :class="{ 'is-active': !!profileStore.tagFilter }">
              <Tag :size="13" :stroke-width="1.75" class="chip-icon" />
              <el-select
                v-model="profileStore.tagFilter"
                class="chip-select"
                size="small"
                clearable
                :placeholder="$t('main.filterTag')"
                popper-class="app-filter-popper"
              >
                <el-option
                  v-for="tag in profileStore.existingTags"
                  :key="tag"
                  :label="tag"
                  :value="tag"
                />
              </el-select>
            </div>
          </div>

          <!-- 视图切换放在导入备份左侧，跟列表内容属于同一层 -->
          <div class="view-toggle">
            <el-tooltip :content="$t('main.gridView')" placement="bottom" :show-after="300">
              <button
                class="view-btn"
                :class="{ active: profileStore.viewMode === 'grid' }"
                @click="profileStore.viewMode = 'grid'"
              >
                <LayoutGrid :size="15" :stroke-width="1.75" />
              </button>
            </el-tooltip>
            <el-tooltip :content="$t('main.listView')" placement="bottom" :show-after="300">
              <button
                class="view-btn"
                :class="{ active: profileStore.viewMode === 'list' }"
                @click="profileStore.viewMode = 'list'"
              >
                <List :size="15" :stroke-width="1.75" />
              </button>
            </el-tooltip>
          </div>

          <el-button class="toolbar-import" plain :loading="importing" @click="handleImport">
            <Import :size="15" :stroke-width="1.75" />
            {{ $t('main.import') }}
          </el-button>

          <el-button type="primary" class="toolbar-create" @click="$emit('create')">
            <Plus :size="16" :stroke-width="2" />
            {{ $t('header.newProfile') }}
          </el-button>
        </div>
      </div>

      <!-- 筛选筛空的，给一键清除，而不是让人以为库是空的 -->
      <el-empty
        v-if="workspaceStore.visibleProfiles.length === 0"
        :description="emptyDescription"
        class="empty-state"
      >
        <el-button v-if="hasActiveFilter" @click="handleClearFilters">
          {{ $t('main.clearFilters') }}
        </el-button>
        <template v-else>
          <el-button type="primary" @click="$emit('create')">
            {{ $t('header.newProfile') }}
          </el-button>
          <el-button plain @click="handleImport">
            <Import :size="15" :stroke-width="1.75" />
            {{ $t('main.import') }}
          </el-button>
        </template>
      </el-empty>

      <div
        v-else
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
            :workspace-color="colorOf(profile)"
            :class="{ 'is-dragging': draggingId === profile.id }"
            :style="{ animationDelay: `${Math.min(index, 8) * 40}ms` }"
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

          <el-button plain :loading="closingBatch" @click="handleBatchClose">
            <el-icon v-if="!closingBatch"><SwitchButton /></el-icon>
            {{ $t('profile.actions.close') }}
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
import { VideoPlay, Brush, Folder, SwitchButton } from '@element-plus/icons-vue';
import {
  Import,
  LayoutGrid,
  List,
  Plus,
  ArrowUp,
  ArrowDown,
  ArrowUpDown,
  Activity,
  Tag,
} from 'lucide-vue-next';
import { open } from '@tauri-apps/plugin-dialog';
import { formatSize } from '../../utils/format';
import { PROFILE_REORDER_TYPE } from '../../utils/dnd';
import { UNASSIGNED } from '../../store/workspace';
import type { Profile } from '../../types';

defineEmits(['create', 'edit', 'backup', 'performance']);
const profileStore = useProfileStore();
const workspaceStore = useWorkspaceStore();
const { t } = useI18n();

const clearingCache = ref(false);
const importing = ref(false);

/** 选择备份包 → 确认内容 → 导入成配置（软件设置与浏览器数据一起还原） */
const handleImport = async () => {
  const picked = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'ProfileX', extensions: ['zip'] }],
  });

  // 取消选择时 open 返回 null
  if (!picked || Array.isArray(picked)) return;

  const info = await api.inspectBackup(picked);

  const message = info.has_meta
    ? t('main.importConfirm', {
        name: info.profile_name,
        workspace: info.workspace_name || t('workspace.unassigned'),
      })
    : t('main.importConfirmLegacy', { name: info.profile_name });

  try {
    await ElMessageBox.confirm(message, t('main.import'), {
      confirmButtonText: t('main.import'),
      cancelButtonText: t('common.cancel'),
      type: 'warning',
    });
  } catch {
    return;
  }

  importing.value = true;

  try {
    // 传 null：让后端按备份包里记录的工作区还原，而不是塞进当前视图
    const profile = await api.importProfileBackup(picked, null);

    await profileStore.loadProfiles();
    await workspaceStore.loadWorkspaces();

    ElMessage.success(t('main.imported', { name: profile.name }));
  } catch (error) {
    ElMessage.error(typeof error === 'string' ? error : t('common.error'));
    console.error(error);
  } finally {
    importing.value = false;
  }
};

const visibleIds = computed(() => workspaceStore.visibleProfiles.map(p => p.id));

const allSelected = computed(
  () => visibleIds.value.length > 0 && visibleIds.value.every(id => profileStore.selectedIds.has(id))
);

const partiallySelected = computed(
  () => profileStore.selectedIds.size > 0 && !allSelected.value
);

const sortOptions = computed(() => [
  { value: 'custom', label: t('main.sortCustom') },
  { value: 'name', label: t('main.sortName') },
  { value: 'created', label: t('main.sortCreated') },
  { value: 'opened', label: t('main.sortOpened') },
  { value: 'size', label: t('main.sortSize') },
]);

const statusOptions = computed(() => [
  { value: 'all', label: t('main.filterAll') },
  { value: 'running', label: t('main.filterRunning') },
  { value: 'stopped', label: t('main.filterStopped') },
]);

const toggleSortDir = () => {
  profileStore.sortDir = profileStore.sortDir === 'asc' ? 'desc' : 'asc';
};

const closingAll = ref(false);

const handleCloseAll = async () => {
  closingAll.value = true;

  try {
    await profileStore.handleCloseAll();
  } finally {
    closingAll.value = false;
  }
};

const closingBatch = ref(false);

const handleBatchClose = async () => {
  closingBatch.value = true;

  try {
    await profileStore.handleBatchClose();
    ElMessage.success(t('common.success'));
  } catch (error) {
    ElMessage.error(t('common.error'));
    console.error(error);
  } finally {
    closingBatch.value = false;
  }
};

/** 工作区颜色表，卡片左侧色条用它标识归属 */
const colorOf = (profile: Profile) =>
  profile.workspace_id
    ? workspaceStore.workspaces.find(ws => ws.id === profile.workspace_id)?.color || null
    : null;

const gridRef = ref<HTMLElement | null>(null);
/** 插入位置下标（0 ~ 列表长度） */
const dropIndex = ref<number | null>(null);
const draggingId = ref<string | null>(null);

/** 非手动顺序下拖拽排序没有意义（排完会被排序规则立刻覆盖），直接不启用 */
const canReorder = computed(() => profileStore.sortBy === 'custom');

const onDragStart = (event: DragEvent) => {
  if (!canReorder.value) return;

  draggingId.value = event.dataTransfer?.getData(PROFILE_REORDER_TYPE) || null;
};

/** 按阅读顺序判断指针落在哪个卡片之前 */
const onDragOver = (event: DragEvent) => {
  const container = gridRef.value;
  if (!container || !canReorder.value) return;

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

  if (!canReorder.value) return;
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

/** 搜索或筛选是否正把列表筛掉（决定空态给「清除筛选」还是「新建配置」） */
const hasActiveFilter = computed(
  () =>
    !!profileStore.searchQuery.trim() ||
    profileStore.statusFilter !== 'all' ||
    !!profileStore.tagFilter,
);

const emptyDescription = computed(() => {
  if (hasActiveFilter.value) return t('main.noMatchingProfiles');
  if (workspaceStore.activeWorkspaceId !== null) return t('workspace.messages.noProfiles');
  return t('main.noProfiles');
});

/** 一键回到「全部状态 + 无标签 + 无搜索」 */
const handleClearFilters = () => {
  profileStore.searchQuery = '';
  profileStore.statusFilter = 'all';
  profileStore.tagFilter = '';
};

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

  // 底部浮着批量操作栏，给它留出空间才不会遮住最后一行
  &.has-batch-bar {
    padding-bottom: 96px;
  }
}

// 卡片骨架屏：结构与 ProfileCard 对齐
.skeleton-card {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-4);
  background: var(--bg-primary);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-lg);
  min-height: 92px;
}

.skeleton-avatar {
  width: 52px;
  height: 52px;
  border-radius: var(--radius-lg);
  flex: 0 0 auto;
}

.skeleton-lines {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  flex: 1;
}

.skeleton-line {
  height: 12px;

  &.w60 {
    width: 60%;
  }

  &.w40 {
    width: 40%;
  }
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
  // 窗口太窄时让筛选胶囊换到下一行，而不是把右边的按钮挤出去
  flex-wrap: wrap;
  row-gap: var(--space-2);

  :deep(.el-checkbox__label) {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
}

// 筛选胶囊跟按钮同组，靠 toolbar-actions 的 margin-left:auto 一起推到右边
.filter-group {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  min-width: 0;
  // 和左边那组按钮拉开一点距离，视觉上分成「筛选」和「动作」
  margin-left: var(--space-1);
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-left: auto;
}

// 胶囊外壳负责边框和底色，里面的 el-select 完全透明，
// 这样下拉框看起来就是一整块，而不是「输入框套输入框」
.filter-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 30px;
  padding: 0 6px 0 8px;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-full);
  background: var(--bg-primary);
  transition: background var(--transition-fast), border-color var(--transition-fast);

  &:hover {
    background: var(--bg-hover);
    border-color: var(--border-medium);
  }

  // 非默认值才点亮，一眼能看出「当前有筛选」
  &.is-active {
    background: var(--primary-50);
    border-color: var(--primary-200);

    .chip-icon,
    :deep(.el-select__selected-item) {
      color: var(--primary-600);
    }

    :deep(.el-select__suffix) {
      color: var(--primary-500);
    }
  }

  // 让下拉宽度跟着文字走：EP 的 selection 是 flex:1（basis 0），
  // 父级宽度 auto 时会被压成 0，只剩图标和箭头，所以这里把 basis 改回 auto
  :deep(.el-select) {
    width: auto;
  }

  :deep(.el-select__selection) {
    flex: 0 1 auto;
  }

  // 占位文字是绝对定位的，不参与撑宽；空值时 EP 会挂 is-transparent，
  // 默认半透明压在下层，这里统一改成正常流，空态才看得见文字
  :deep(.el-select__placeholder) {
    position: static;
    transform: none;
    z-index: auto;
    width: auto;
    color: var(--text-tertiary);
  }

  :deep(.el-select__wrapper) {
    min-height: 24px;
    min-width: 48px;
    padding: 0;
    background: transparent;
    box-shadow: none !important;
    font-size: var(--text-sm);
  }

  :deep(.el-select__selected-item) {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  // 空值时的占位符比正常选中值淡一档（EP 用 is-transparent 标出这一状态）
  :deep(.el-select__placeholder.is-transparent) {
    color: var(--text-tertiary);
  }

  :deep(.el-select__suffix),
  :deep(.el-select__caret) {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  :deep(.el-select__placeholder) {
    font-size: var(--text-sm);
    color: var(--text-tertiary);
  }
}

.chip-icon {
  flex-shrink: 0;
  color: var(--text-tertiary);
}

// 升降序按钮跟胶囊同一套外观
.chip-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  padding: 0;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-full);
  background: var(--bg-primary);
  color: var(--text-secondary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);

  &:hover {
    background: var(--bg-hover);
    color: var(--primary-600);
  }
}

.close-all-btn {
  color: var(--danger-600);
  border-color: var(--danger-200);

  &:hover {
    background: var(--danger-50);
    border-color: var(--danger-300);
    color: var(--danger-700);
  }
}

// 视图切换：靠右贴住导入按钮，和它算一组
.view-toggle {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}

.view-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 24px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  border-radius: calc(var(--radius-md) - 2px);
  cursor: pointer;
  transition: all var(--transition-fast);

  svg {
    display: block;
  }

  &:hover {
    color: var(--text-primary);
  }

  &.active {
    background: var(--bg-primary);
    color: var(--primary-500);
    box-shadow: var(--shadow-xs);
  }
}

.toolbar-import {
  margin-left: 0;
}

.toolbar-create {
  // Element 会给相邻按钮加 12px 左外边距，这里已经有 gap 了
  &.el-button {
    margin-left: 0;
  }

  svg {
    display: block;
  }
}

.profiles-container {
  display: grid;
  gap: var(--space-4);

  &.grid-view {
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  }

  &.list-view {
    grid-template-columns: 1fr;
  }

  > * {
    animation: fadeInUp 0.4s ease-out backwards;
  }
}

// 拖拽排序：插入位置指示线。用细线代替大方块，避免整行被挤开产生跳动
.drop-indicator {
  position: relative;
  height: 2px;
  border-radius: var(--radius-full);
  background: var(--primary-500);
  box-shadow: 0 0 6px var(--primary-400);

  // 两端加个小圆头，视觉上更清楚
  &::before,
  &::after {
    content: '';
    position: absolute;
    top: 50%;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--primary-500);
    transform: translateY(-50%);
  }

  &::before {
    left: -3px;
  }

  &::after {
    right: -3px;
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
