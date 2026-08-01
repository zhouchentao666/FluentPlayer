import enUS from '../Strings/en-US/Resources';
import zhCN from '../Strings/zh-CN/Resources';

const messages = {
  'en-US': enUS,
  'zh-CN': zhCN,
};

export type Locale = keyof typeof messages;

const getNested = (obj: any, path: string): any => {
  return path.split('.').reduce((acc, key) => (acc == null ? acc : acc[key]), obj);
};

export function useRes(key: string, locale: Locale = 'zh-CN'): string {
  const dict = messages[locale] || messages['zh-CN'];
  const val = getNested(dict, key);
  return typeof val === 'string' ? val : key;
}

export function useT(): (key: string) => string {
  return (key: string) => useRes(key, currentLocale);
}

let currentLocale: Locale = 'zh-CN';

export function setLocale(locale: Locale) {
  currentLocale = locale;
}

export function getLocale(): Locale {
  return currentLocale;
}

export { messages };
export default { useRes, useT, setLocale, getLocale, messages };
