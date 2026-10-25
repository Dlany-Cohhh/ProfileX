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
 */
export function macaronColor(seed: string): MacaronColor {
  let hash = 0;

  for (let i = 0; i < seed.length; i += 1) {
    hash = (hash * 31 + seed.charCodeAt(i)) % 2147483647;
  }

  return MACARON_PALETTE[hash % MACARON_PALETTE.length];
}
