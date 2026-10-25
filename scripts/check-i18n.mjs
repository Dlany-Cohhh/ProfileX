// 校验中英文案 key 是否一致：漏一个 key 在界面上就是空白或原始 key
// 用法：node scripts/check-i18n.mjs
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const localesDir = join(root, 'src', 'i18n', 'locales');

const flatten = (obj, prefix = '') =>
  Object.entries(obj).flatMap(([key, value]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return value && typeof value === 'object' ? flatten(value, path) : [path];
  });

const files = readdirSync(localesDir).filter((f) => f.endsWith('.json'));

if (files.length < 2) {
  console.error(`locale 文件不足：${files.join(', ') || '空'}`);
  process.exit(1);
}

const tables = new Map(
  files.map((file) => [file.replace('.json', ''), JSON.parse(readFileSync(join(localesDir, file), 'utf8'))])
);

const keySets = new Map([...tables].map(([lang, obj]) => [lang, new Set(flatten(obj))]));

// 以第一个语言为基准，其余语言逐一比对
const [baseLang, baseKeys] = [...keySets][0];
let problems = 0;

for (const [lang, keys] of keySets) {
  if (lang === baseLang) continue;

  const missing = [...baseKeys].filter((k) => !keys.has(k));
  const extra = [...keys].filter((k) => !baseKeys.has(k));

  for (const key of missing) {
    console.error(`[缺失] ${lang}.${key}（${baseLang} 有）`);
    problems += 1;
  }

  for (const key of extra) {
    console.error(`[多余] ${lang}.${key}（${baseLang} 没有）`);
    problems += 1;
  }
}

if (problems > 0) {
  console.error(`\n共 ${problems} 处不一致`);
  process.exit(1);
}

console.log(`i18n 校验通过：${[...keySets].map(([l, s]) => `${l}=${s.size}`).join(', ')}`);
