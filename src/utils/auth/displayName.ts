import i18n from '@/locales'

export default function displayName(user: { name?: string | null; username?: string | null; nickname?: string | null } | null): string {
  const t = i18n.global.t
  if (!user) return t('common.notLoggedIn')
  return user.name || user.nickname || user.username || t('common.notLoggedIn')
}
