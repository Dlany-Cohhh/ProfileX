import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { api } from '../api';
import type { Profile, ViewMode } from '../types';
import { ElMessage } from 'element-plus';
import { listen } from '@tauri-apps/api/event';
import i18n from '../i18n';

export const useProfileStore = defineStore('profile', () => {
    const profiles = ref<Profile[]>([]);
    const loading = ref(false);
    const searchQuery = ref('');
    const viewMode = ref<ViewMode>('grid');
    const launchingProfiles = ref<Set<string>>(new Set());
    const selectedIds = ref<Set<string>>(new Set());
    /** profile id -> 磁盘占用字节数 */
    const sizes = ref<Record<string, number>>({});
    const loadingSizes = ref(false);
    /** 正在关闭的配置，避免重复点 */
    const closingProfiles = ref<Set<string>>(new Set());

    /** 列表排序方式。custom = 界面上手动拖出来的顺序 */
    const sortBy = ref<'custom' | 'name' | 'created' | 'opened' | 'size'>('custom');
    const sortDir = ref<'asc' | 'desc'>('asc');
    /** 运行态筛选 */
    const statusFilter = ref<'all' | 'running' | 'stopped'>('all');
    /** 标签筛选，空串 = 不过滤 */
    const tagFilter = ref('');
    /** 被标记为模板的配置 id，新建配置时可套用 */
    const templateIds = ref<string[]>([]);

    /** 模板配置列表（按当前排序） */
    const templateProfiles = computed(() =>
        profiles.value.filter(p => templateIds.value.includes(p.id))
    );

    const loadTemplates = async () => {
        try {
            templateIds.value = await api.getTemplateIds();
        } catch (error) {
            console.error(error);
        }
    };

    const toggleTemplate = async (id: string) => {
        const next = !templateIds.value.includes(id);
        await api.setProfileTemplate(id, next);
        await loadTemplates();
    };

    /** 搜索 + 运行态/标签筛选 + 排序后的列表 */
    const filteredProfiles = computed(() => {
        let list = profiles.value;

        const query = searchQuery.value.trim().toLowerCase();
        if (query) {
            list = list.filter(
                p =>
                    p.name.toLowerCase().includes(query) ||
                    p.tags?.toLowerCase().includes(query) ||
                    p.notes?.toLowerCase().includes(query)
            );
        }

        if (statusFilter.value === 'running') {
            list = list.filter(p => p.is_running);
        } else if (statusFilter.value === 'stopped') {
            list = list.filter(p => !p.is_running);
        }

        if (tagFilter.value) {
            list = list.filter(p =>
                p.tags?.split(',').some(t => t.trim() === tagFilter.value)
            );
        }

        if (sortBy.value === 'custom') return list;

        const factor = sortDir.value === 'asc' ? 1 : -1;
        const valueOf = (p: Profile): string | number => {
            switch (sortBy.value) {
                case 'name':
                    return p.name;
                case 'created':
                    return p.created_at;
                case 'opened':
                    return p.last_opened_at || '';
                case 'size':
                    return sizes.value[p.id] ?? 0;
                default:
                    return 0;
            }
        };

        // 复制一份再排，避免就地改动 Pinia 里的数组
        return [...list].sort((a, b) => {
            const va = valueOf(a);
            const vb = valueOf(b);

            if (typeof va === 'number' && typeof vb === 'number') {
                return (va - vb) * factor;
            }

            return String(va).localeCompare(String(vb), 'zh-Hans-CN') * factor;
        });
    });

    /** 正在运行的配置数，决定要不要显示「全部关闭」 */
    const runningCount = computed(() => profiles.value.filter(p => p.is_running).length);

    const existingTags = computed(() => {
        const tags = new Set<string>();
        profiles.value.forEach(p => {
            p.tags?.split(',').forEach(t => {
                const tag = t.trim();
                if (tag) tags.add(tag);
            });
        });
        return Array.from(tags);
    });

    const loadProfiles = async () => {
        loading.value = true;
        try {
            profiles.value = await api.getProfiles();
        } catch (error) {
            ElMessage.error(i18n.global.t('common.error'));
            console.error(error);
        } finally {
            loading.value = false;
        }
    };

    /** 统计所有配置的磁盘占用。文件多，放后台算，不阻塞界面 */
    const loadSizes = async () => {
        if (loadingSizes.value) return;
        loadingSizes.value = true;
        try {
            const entries = await api.getProfileSizes();
            const map: Record<string, number> = {};
            entries.forEach(([id, bytes]) => {
                map[id] = bytes;
            });
            sizes.value = map;
        } catch (error) {
            console.error(error);
        } finally {
            loadingSizes.value = false;
        }
    };

    const sizeOf = (id: string) => sizes.value[id];

    const handleLaunch = async (profile: Profile) => {
        if (launchingProfiles.value.has(profile.id)) return;

        try {
            launchingProfiles.value.add(profile.id);
            const result = await api.launchChrome(profile.id);

            if (result.success) {
                ElMessage.success(i18n.global.t('common.success'));
                await loadProfiles();
            } else {
                ElMessage.error(result.error || i18n.global.t('common.error'));
            }
        } catch (error) {
            ElMessage.error(i18n.global.t('common.error'));
            console.error(error);
        } finally {
            launchingProfiles.value.delete(profile.id);
        }
    };

    const toggleSelection = (id: string) => {
        if (selectedIds.value.has(id)) {
            selectedIds.value.delete(id);
        } else {
            selectedIds.value.add(id);
        }
    };

    const clearSelection = () => {
        selectedIds.value.clear();
    };

    /** 全选/取消全选当前可见的配置 */
    const toggleSelectAll = (ids: string[]) => {
        const allSelected = ids.length > 0 && ids.every(id => selectedIds.value.has(id));
        selectedIds.value = allSelected ? new Set<string>() : new Set(ids);
    };

    /** 关掉一个配置对应的浏览器 */
    const handleClose = async (profile: Profile) => {
        if (closingProfiles.value.has(profile.id)) return;

        closingProfiles.value.add(profile.id);

        try {
            await api.closeProfile(profile.id);
            await loadProfiles();
            ElMessage.success(i18n.global.t('profile.messages.closed', { name: profile.name }));
        } catch (error) {
            ElMessage.error(i18n.global.t('common.error'));
            console.error(error);
        } finally {
            closingProfiles.value.delete(profile.id);
        }
    };

    /** 一键关闭所有运行中的配置 */
    const handleCloseAll = async () => {
        try {
            const count = await api.closeAllProfiles();
            await loadProfiles();

            if (count === 0) {
                ElMessage.info(i18n.global.t('profile.messages.nothingRunning'));
            } else {
                ElMessage.success(i18n.global.t('profile.messages.closedAll', { count }));
            }
        } catch (error) {
            ElMessage.error(i18n.global.t('common.error'));
            console.error(error);
        }
    };

    /** 批量关闭选中的配置 */
    const handleBatchClose = async () => {
        const ids = Array.from(selectedIds.value);
        if (ids.length === 0) return;

        for (const id of ids) {
            const profile = profiles.value.find(p => p.id === id);
            if (profile?.is_running) {
                await api.closeProfile(id).catch(console.error);
            }
        }

        await loadProfiles();
        clearSelection();
    };

    const handleBatchLaunch = async () => {
        const ids = Array.from(selectedIds.value);
        if (ids.length === 0) return;

        ElMessage.info(i18n.global.t('main.batchLaunching', { count: ids.length }));

        for (const id of ids) {
            const profile = profiles.value.find(p => p.id === id);
            if (profile && !profile.is_running) {
                handleLaunch(profile);
                // Stagger launch to avoid heavy CPU spike
                await new Promise(r => setTimeout(r, 500));
            }
        }
        clearSelection();
    };

    const initStatusListener = async () => {
        await listen('browser-status-update', (event: any) => {
            const { id, is_running } = event.payload;
            const profile = profiles.value.find(p => p.id === id);
            if (profile) {
                profile.is_running = is_running;
            }
        });
    };

    return {
        profiles,
        loading,
        searchQuery,
        viewMode,
        launchingProfiles,
        closingProfiles,
        selectedIds,
        sizes,
        loadingSizes,
        templateIds,
        templateProfiles,
        filteredProfiles,
        runningCount,
        sortBy,
        sortDir,
        statusFilter,
        tagFilter,
        existingTags,
        loadProfiles,
        loadTemplates,
        toggleTemplate,
        loadSizes,
        sizeOf,
        handleLaunch,
        handleClose,
        handleCloseAll,
        handleBatchClose,
        toggleSelection,
        clearSelection,
        toggleSelectAll,
        handleBatchLaunch,
        initStatusListener,
    };
});
