import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ACCENT_STORAGE_KEY, DEFAULT_ACCENT, applyAccent } from '../utils/accent';

export const useThemeStore = defineStore('theme', () => {
    const isDark = ref(localStorage.getItem('theme') === 'dark' ||
        (!localStorage.getItem('theme') && window.matchMedia('(prefers-color-scheme: dark)').matches));

    /** 主题色（强调色），存本机，不进后端设置 */
    const accent = ref(localStorage.getItem(ACCENT_STORAGE_KEY) || DEFAULT_ACCENT);

    const setAccent = (color: string) => {
        accent.value = color;
    };

    const toggleTheme = () => {
        isDark.value = !isDark.value;
    };

    watch(isDark, (val) => {
        const root = document.documentElement;
        const appWindow = getCurrentWindow();

        if (val) {
            root.classList.add('dark');
            localStorage.setItem('theme', 'dark');
            appWindow.setTheme('dark').catch(console.error);
        } else {
            root.classList.remove('dark');
            localStorage.setItem('theme', 'light');
            appWindow.setTheme('light').catch(console.error);
        }

        // 明暗切换后浅色档/主色的取值不一样，重新铺一遍色阶
        applyAccent(accent.value, val);
    }, { immediate: true });

    watch(accent, (val) => {
        localStorage.setItem(ACCENT_STORAGE_KEY, val);
        applyAccent(val, isDark.value);
    }, { immediate: true });

    return {
        isDark,
        accent,
        setAccent,
        toggleTheme
    };
});
