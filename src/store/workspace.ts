import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { api } from '../api';
import type { Workspace } from '../types';
import { ElMessage } from 'element-plus';
import i18n from '../i18n';
import { useProfileStore } from './profile';

/** 特殊标记：未归入任何工作区的配置 */
export const UNASSIGNED = '__unassigned__';

interface BackupProgressPayload {
    current: number;
    total: number;
    name: string;
}

export const useWorkspaceStore = defineStore('workspace', () => {
    const profileStore = useProfileStore();

    const workspaces = ref<Workspace[]>([]);
    /** null = 全部配置；UNASSIGNED = 未分组；其余为具体工作区 id */
    const activeWorkspaceId = ref<string | null>(null);
    const launching = ref(false);
    const backing = ref(false);
    /** 备份进度：当前第几个 / 共几个 / 正在打包哪个配置 */
    const backupProgress = ref<BackupProgressPayload | null>(null);

    const activeWorkspace = computed(() =>
        workspaces.value.find(w => w.id === activeWorkspaceId.value) || null
    );

    /** 搜索 + 工作区过滤后的最终列表 */
    const visibleProfiles = computed(() => {
        const list = profileStore.filteredProfiles;

        if (activeWorkspaceId.value === null) return list;
        if (activeWorkspaceId.value === UNASSIGNED) {
            return list.filter(p => !p.workspace_id);
        }
        return list.filter(p => p.workspace_id === activeWorkspaceId.value);
    });

    /** 每个工作区（含未分组）下的配置数量 */
    const counts = computed<Record<string, number>>(() => {
        const map: Record<string, number> = { [UNASSIGNED]: 0 };
        workspaces.value.forEach(w => {
            map[w.id] = 0;
        });

        profileStore.profiles.forEach(p => {
            if (p.workspace_id && map[p.workspace_id] !== undefined) {
                map[p.workspace_id] += 1;
            } else {
                map[UNASSIGNED] += 1;
            }
        });

        return map;
    });

    const loadWorkspaces = async () => {
        try {
            workspaces.value = await api.getWorkspaces();
        } catch (error) {
            console.error(error);
        }
    };

    const createWorkspace = async (payload: {
        name: string;
        icon?: string | null;
        color?: string | null;
        description?: string | null;
    }) => {
        const workspace = await api.createWorkspace(
            payload.name,
            payload.icon ?? undefined,
            payload.color ?? undefined,
            payload.description ?? undefined
        );
        await loadWorkspaces();
        return workspace;
    };

    const updateWorkspace = async (
        id: string,
        payload: { name?: string; icon?: string | null; color?: string | null; description?: string | null }
    ) => {
        await api.updateWorkspace(
            id,
            payload.name,
            payload.icon ?? undefined,
            payload.color ?? undefined,
            payload.description ?? undefined
        );
        await loadWorkspaces();
    };

    const removeWorkspace = async (id: string) => {
        await api.deleteWorkspace(id);
        if (activeWorkspaceId.value === id) {
            activeWorkspaceId.value = null;
        }
        await loadWorkspaces();
        await profileStore.loadProfiles();
    };

    const moveProfile = async (profileId: string, workspaceId: string | null) => {
        await api.moveProfileToWorkspace(profileId, workspaceId);
        await profileStore.loadProfiles();
    };

    /** 批量把多个配置移动到同一工作区，一次调用完成 */
    const moveProfiles = async (profileIds: string[], workspaceId: string | null) => {
        if (profileIds.length === 0) return;

        await api.moveProfilesToWorkspace(profileIds, workspaceId);

        await profileStore.loadProfiles();
        await loadWorkspaces();
    };

    /** 一键启动当前工作区内所有未运行的配置 */
    const launchActiveWorkspace = async () => {
        const targets = visibleProfiles.value.filter(p => !p.is_running);

        if (targets.length === 0) {
            ElMessage.info(i18n.global.t('workspace.messages.nothingToLaunch'));
            return;
        }

        launching.value = true;
        ElMessage.info(i18n.global.t('workspace.messages.launching', { count: targets.length }));

        try {
            for (const profile of targets) {
                await profileStore.handleLaunch(profile);
                // 错开启动，避免 CPU 瞬时打满
                await new Promise(r => setTimeout(r, 500));
            }
        } finally {
            launching.value = false;
        }
    };

    /** 备份当前工作区（或全部/未分组视图）下的所有配置 */
    const backupActiveWorkspace = async () => {
        // 备份很慢，避免重复点击叠加任务
        if (backing.value) return;

        const targets = visibleProfiles.value;
        if (targets.length === 0) return;

        backing.value = true;
        backupProgress.value = null;

        // 工作区备份由后端逐个打包并上报，这里只负责接收
        const unlisten = await listen<BackupProgressPayload>('backup-progress', event => {
            backupProgress.value = event.payload;
        });

        try {
            const backupDir = await api.getBackupDir();
            const id = activeWorkspaceId.value;

            let success = 0;

            if (id && id !== UNASSIGNED) {
                const results = await api.backupWorkspace(id, backupDir);
                success = results.filter(r => r.success).length;
            } else {
                // 全部/未分组：交给后端一次打包，进度同样来自 backup-progress 事件
                const ids = targets.map(p => p.id);
                const results = await api.backupProfiles(ids, backupDir);
                success = results.filter(r => r.success).length;
            }

            const failed = targets.length - success;

            if (failed === 0) {
                ElMessage.success(i18n.global.t('workspace.messages.backupDone', { count: success }));
            } else {
                ElMessage.warning(
                    i18n.global.t('workspace.messages.backupPartial', { success, failed })
                );
            }
        } catch (error) {
            ElMessage.error(i18n.global.t('common.error'));
            console.error(error);
        } finally {
            unlisten();
            backupProgress.value = null;
            backing.value = false;
        }
    };

    return {
        workspaces,
        activeWorkspaceId,
        activeWorkspace,
        visibleProfiles,
        counts,
        launching,
        backing,
        backupProgress,
        loadWorkspaces,
        createWorkspace,
        updateWorkspace,
        removeWorkspace,
        moveProfile,
        moveProfiles,
        launchActiveWorkspace,
        backupActiveWorkspace,
    };
});
