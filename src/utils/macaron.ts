/**
 * 马卡龙配色：低饱和、高明度的纯色底 + 同色系深一点的文字色。
 * 底色太浅，白色首字会看不清，所以首字用同色系的深色调。
 */
export interface MacaronColor {
  bg: string;
  fg: string;
}

export const MACARON_PALETTE: MacaronColor[] = [
  { bg: '#FFB5C5', fg: '#A8385A' }, // 樱花粉
  { bg: '#FFCFA8', fg: '#9A5A22' }, // 蜜桃橙
  { bg: '#F7E1A0', fg: '#8A6A1E' }, // 柠檬黄
  { bg: '#C7E9A8', fg: '#4C7A26' }, // 抹茶绿
  { bg: '#A8E3D0', fg: '#1F7360' }, // 薄荷绿
  { bg: '#A9D8F0', fg: '#2A6A9E' }, // 天青蓝
  { bg: '#C3CFF2', fg: '#46559A' }, // 雾霾蓝
  { bg: '#D9C2F0', fg: '#63409E' }, // 香芋紫
  { bg: '#F5C0E6', fg: '#9A3D80' }, // 藕荷粉
  { bg: '#E9C6AE', fg: '#8A5330' }, // 焦糖
];

/**
 * 按 id 稳定取色：同一个配置每次拿到的颜色都一样，
 * 不同 id 尽量散开，避免相邻卡片撞色
 *
 * 这里用 FNV-1a + 末尾混合（lowbias32）。简单的 hash*31 对
 * 前缀相同、尾部递增的 id（uuid、时间戳命名）分布很差，
 * 会出现一片卡片同色。
 */
export function macaronColor(seed: string): MacaronColor {
  let hash = 0x811c9dc5;

  for (let i = 0; i < seed.length; i += 1) {
    hash ^= seed.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }

  // 打散低位，否则取模后容易落在相邻的桶里
  hash ^= hash >>> 16;
  hash = Math.imul(hash, 0x7feb352d);
  hash ^= hash >>> 15;
  hash = Math.imul(hash, 0x846ca68b);
  hash ^= hash >>> 16;

  return MACARON_PALETTE[(hash >>> 0) % MACARON_PALETTE.length];
}

function hexToHsl(hex: string): { h: number; s: number; l: number } {
  const raw = hex.replace('#', '');
  const full = raw.length === 3 ? raw.split('').map(c => c + c).join('') : raw;

  const r = parseInt(full.slice(0, 2), 16) / 255;
  const g = parseInt(full.slice(2, 4), 16) / 255;
  const b = parseInt(full.slice(4, 6), 16) / 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;

  let h = 0;
  if (delta !== 0) {
    if (max === r) h = ((g - b) / delta) % 6;
    else if (max === g) h = (b - r) / delta + 2;
    else h = (r - g) / delta + 4;
  }

  h = Math.round(h * 60);
  if (h < 0) h += 360;

  const l = (max + min) / 2;
  const s = delta === 0 ? 0 : delta / (1 - Math.abs(2 * l - 1));

  return { h, s: Math.round(s * 100), l: Math.round(l * 100) };
}

/**
 * 底色是浅色，首字必须用同色系的深色才看得清。
 * 自定义颜色也走这里，省得每个色都要手工配一个文字色。
 */
export function textColorFor(hex: string): string {
  const { h, s } = hexToHsl(hex);

  return `hsl(${h} ${Math.max(s, 45)}% 34%)`;
}
