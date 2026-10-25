/**
 * 主题色（强调色）。
 *
 * 界面里所有蓝色系元素都走 --primary-* / --secondary-* 变量，
 * 这里把用户选的一个颜色展开成完整色阶写回 :root，
 * 同时同步 Element Plus 自己的 --el-color-primary，
 * 这样 el-switch / el-progress / el-color-picker 这些没被 global.scss
 * 覆盖到的组件也会跟着换色。
 *
 * 写在 <html> 的 inline style 上，优先级高于 variables.scss 里的
 * :root 和 html.dark 规则，所以深色模式下也能盖住。
 */

export interface AccentPreset {
  /** i18n 键：settings.accentColors.<key> */
  key: string;
  /** 色阶 500 档，也就是用户看到的主色 */
  color: string;
}

export const ACCENT_PRESETS: AccentPreset[] = [
  { key: 'blue', color: '#0c9df8' },
  { key: 'indigo', color: '#3b2fc7' },
  { key: 'cyan', color: '#14b6dc' },
  { key: 'green', color: '#00b42a' },
  { key: 'teal', color: '#0fc6c2' },
  { key: 'purple', color: '#7c4dff' },
  { key: 'pink', color: '#f7659c' },
  { key: 'orange', color: '#ff7d00' },
];

export const DEFAULT_ACCENT = ACCENT_PRESETS[0].color;

export const ACCENT_STORAGE_KEY = 'profilex.accent';

/** 各档位的目标亮度，取自原飞鼠蓝色阶 */
const LEVELS = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900] as const;
const LEVEL_LIGHTNESS: Record<number, number> = {
  50: 96, 100: 93, 200: 85, 300: 74, 400: 60, 500: 51, 600: 42, 700: 34, 800: 29, 900: 25,
};
/** 各档位的饱和度系数，浅色档降一点饱和度才不会刺眼 */
const LEVEL_SATURATION: Record<number, number> = {
  50: 0.7, 100: 0.9, 200: 0.95, 300: 0.98, 400: 1, 500: 1, 600: 0.98, 700: 0.95, 800: 0.9, 900: 0.85,
};

interface Hsl {
  h: number;
  s: number;
  l: number;
}
interface Rgb {
  r: number;
  g: number;
  b: number;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function hexToRgb(hex: string): Rgb {
  const raw = hex.replace('#', '').trim();
  const full =
    raw.length === 3
      ? raw
          .split('')
          .map((c) => c + c)
          .join('')
      : raw;

  return {
    r: parseInt(full.slice(0, 2), 16) || 0,
    g: parseInt(full.slice(2, 4), 16) || 0,
    b: parseInt(full.slice(4, 6), 16) || 0,
  };
}

function rgbToHex({ r, g, b }: Rgb): string {
  const toHex = (v: number) => clamp(Math.round(v), 0, 255).toString(16).padStart(2, '0');

  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

function rgbToHsl({ r, g, b }: Rgb): Hsl {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;

  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const delta = max - min;

  let h = 0;
  if (delta !== 0) {
    if (max === rn) h = ((gn - bn) / delta) % 6;
    else if (max === gn) h = (bn - rn) / delta + 2;
    else h = (rn - gn) / delta + 4;
  }

  h = Math.round(h * 60);
  if (h < 0) h += 360;

  const l = (max + min) / 2;
  const s = delta === 0 ? 0 : delta / (1 - Math.abs(2 * l - 1));

  return { h, s: Math.round(s * 100), l: Math.round(l * 100) };
}

function hslToHex({ h, s, l }: Hsl): string {
  const sn = clamp(s, 0, 100) / 100;
  const ln = clamp(l, 0, 100) / 100;
  const a = sn * Math.min(ln, 1 - ln);
  const k = (n: number) => (n + h / 30) % 12;
  const f = (n: number) => ln - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));

  return rgbToHex({ r: f(0) * 255, g: f(8) * 255, b: f(4) * 255 });
}

function mix(from: string, to: string, ratio: number): string {
  const a = hexToRgb(from);
  const b = hexToRgb(to);
  const m = (x: number, y: number) => x + (y - x) * ratio;

  return rgbToHex({ r: m(a.r, b.r), g: m(a.g, b.g), b: m(a.b, b.b) });
}

function rgba(hex: string, alpha: number): string {
  const { r, g, b } = hexToRgb(hex);

  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * 由一个主色展开成 50~900 色阶。
 * 亮度用「目标值 + 主色亮度的偏移量」来算，
 * 这样选深色（如靛蓝）或浅色都不会算出明显跑偏的浅色档。
 */
function buildScale(baseHex: string, hueShift = 0): Record<number, string> {
  const base = rgbToHsl(hexToRgb(baseHex));
  const scale: Record<number, string> = {};

  for (const level of LEVELS) {
    // 500 档直接用主色本身，保证选什么就显示什么
    if (level === 500) {
      if (hueShift === 0) {
        scale[level] = baseHex;
        continue;
      }

      // 副色比主色稍暗、稍淡一点，避免渐变里抢过主色
      scale[level] = hslToHex({
        h: (base.h + hueShift + 360) % 360,
        s: clamp(base.s * 0.9, 0, 100),
        l: clamp(base.l - 4, 6, 97),
      });
      continue;
    }

    const l = clamp(LEVEL_LIGHTNESS[level] + (base.l - 51) * 0.25, 6, 97);
    const s = clamp(base.s * LEVEL_SATURATION[level], 0, 100);

    scale[level] = hslToHex({ h: (base.h + hueShift + 360) % 360, s, l });
  }

  return scale;
}

/** 设置主题色。深色模式下浅色档换成半透明、主色换成更亮的那一档 */
export function applyAccent(color: string, isDark: boolean): void {
  const root = document.documentElement;
  const primary = buildScale(color);
  // 副色跟着主色走，保持原来「主蓝 + 副青」的色相关系（约 -12°）
  const secondary = buildScale(color, -12);

  for (const level of LEVELS) {
    root.style.setProperty(`--primary-${level}`, primary[level]);
    root.style.setProperty(`--secondary-${level}`, secondary[level]);
  }

  if (isDark) {
    root.style.setProperty('--primary-50', rgba(primary[500], 0.1));
    root.style.setProperty('--primary-100', rgba(primary[500], 0.2));
    root.style.setProperty('--primary-200', rgba(primary[500], 0.3));
    // 深色背景上主色要提亮，否则按钮文字看不清
    root.style.setProperty('--primary-600', primary[400]);

    root.style.setProperty('--secondary-50', rgba(secondary[500], 0.12));
    root.style.setProperty('--secondary-100', rgba(secondary[500], 0.22));
    root.style.setProperty('--secondary-200', rgba(secondary[500], 0.32));
    root.style.setProperty('--secondary-600', secondary[400]);
  }

  // Element Plus：按钮/输入框已由 global.scss 接管，这里补上开关、进度条等组件的配色
  const epPrimary = isDark ? primary[400] : primary[500];
  // 浅色模式往白色混，深色模式往深底混，hover/背景色才不会突兀
  const epTarget = isDark ? '#1d2129' : '#ffffff';

  root.style.setProperty('--el-color-primary', epPrimary);
  root.style.setProperty('--el-color-primary-light-3', mix(epPrimary, epTarget, 0.3));
  root.style.setProperty('--el-color-primary-light-5', mix(epPrimary, epTarget, 0.5));
  root.style.setProperty('--el-color-primary-light-7', mix(epPrimary, epTarget, 0.7));
  root.style.setProperty('--el-color-primary-light-8', mix(epPrimary, epTarget, 0.8));
  root.style.setProperty('--el-color-primary-light-9', mix(epPrimary, epTarget, 0.9));
  root.style.setProperty(
    '--el-color-primary-dark-2',
    mix(epPrimary, isDark ? '#ffffff' : '#000000', 0.2)
  );
}
