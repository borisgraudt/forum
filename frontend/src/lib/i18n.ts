/** Minimal EN/RU strings for chrome UI. Content stays user-authored. */

export type Locale = 'en' | 'ru';

const dict = {
  en: {
    skip: 'Skip to content',
    brand: 'Forum',
    ask: 'Ask',
    browse: 'Browse',
    search: 'Search',
    alerts: 'Alerts',
    unread: 'unread',
    signIn: 'Sign in',
    profile: 'Profile',
    settings: 'Settings',
    moderation: 'Moderation',
    admin: 'Admin',
    signOut: 'Sign out',
    theme: 'Toggle color theme',
    themeLight: 'Light',
    themeDark: 'Dark',
    status: 'Status',
    searchTitle: 'Search',
    searchLead: 'Find threads by title or post content.',
    searchPlaceholder: 'Search topics and posts…',
    searchBtn: 'Search',
    filterAuthor: 'Author',
    filterCategory: 'Category slug',
    filterSince: 'Since (YYYY-MM-DD)',
    resultsFor: 'results for',
    noMatches: 'No matches. Try different keywords.',
    previous: 'Previous',
    next: 'Next',
    page: 'Page',
    of: 'of',
  },
  ru: {
    skip: 'К содержимому',
    brand: 'Форум',
    ask: 'Спросить',
    browse: 'Обзор',
    search: 'Поиск',
    alerts: 'Уведомления',
    unread: 'непрочитанных',
    signIn: 'Войти',
    profile: 'Профиль',
    settings: 'Настройки',
    moderation: 'Модерация',
    admin: 'Админ',
    signOut: 'Выйти',
    theme: 'Переключить тему',
    themeLight: 'Светлая',
    themeDark: 'Тёмная',
    status: 'Статус',
    searchTitle: 'Поиск',
    searchLead: 'Ищите темы по заголовку и тексту постов.',
    searchPlaceholder: 'Поиск тем и постов…',
    searchBtn: 'Найти',
    filterAuthor: 'Автор',
    filterCategory: 'Slug категории',
    filterSince: 'С даты (ГГГГ-ММ-ДД)',
    resultsFor: 'результатов по запросу',
    noMatches: 'Ничего не найдено. Попробуйте другие слова.',
    previous: 'Назад',
    next: 'Далее',
    page: 'Стр.',
    of: 'из',
  },
} as const;

export type MsgKey = keyof (typeof dict)['en'];

export function resolveLocale(
  cookieHeader: string | null | undefined,
  acceptLanguage: string | null | undefined,
  queryLang?: string | null,
): Locale {
  const q = (queryLang || '').toLowerCase();
  if (q === 'ru' || q === 'en') return q;
  if (cookieHeader) {
    for (const part of cookieHeader.split(';')) {
      const [k, ...rest] = part.trim().split('=');
      if (k === 'lang') {
        const v = rest.join('=').toLowerCase();
        if (v === 'ru' || v === 'en') return v;
      }
    }
  }
  const al = (acceptLanguage || '').toLowerCase();
  if (al.startsWith('ru') || al.includes('ru-')) return 'ru';
  return 'en';
}

export function t(locale: Locale, key: MsgKey): string {
  return dict[locale][key] || dict.en[key] || key;
}
