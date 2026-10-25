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

    const filteredProfiles = computed(() => {
        if (!searchQuery.value) return profiles.value;
        const query = searchQuery.value.toLowerCase();
        return profiles.value.filter(
            p =>
                p.name.toLowerCase().includes(query) ||
                p.tags?.toLowerCase().includes(query)
        );
    });

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
        selectedIds,
        sizes,
        loadingSizes,
        templateIds,
        templateProfiles,
        filteredProfiles,
        existingTags,
        loadProfiles,
        loadTemplates,
        toggleTemplate,
        loadSizes,
        sizeOf,
        handleLaunch,
        toggleSelection,
        clearSelection,
        toggleSelectAll,
        handleBatchLaunch,
        initStatusListener,
    };
});
