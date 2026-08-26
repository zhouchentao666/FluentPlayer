import { createWebHashHistory, createRouter, type RouteRecordRaw, type RouterOptions } from 'vue-router'
import { ref, nextTick } from 'vue'

export const routeDirection = ref<'forward' | 'backward'>('forward')

const routeDepths: Record<string, number> = {
  find: 0,
  songlist: 1,
  local: 2,
  download: 3,
  search: 2,
  recognize: 2,
  recent: 2,
  list: 4,
  singer: 4,
  'local-tag-editor': 5,
  profile: 2,
}

const appRouter: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'welcome',
    component: () => import('@/views/welcome/index.vue')
  },
  {
    path: '/home',
    name: 'home',
    redirect: '/home/find',
    component: () => import('@/views/home/index.vue'),
    children: [
      { path: 'find', name: 'find', component: () => import('@/views/music/find.vue') },
      { path: 'songlist', name: 'songlist', component: () => import('@/views/music/songlist.vue') },
      { path: 'local', name: 'local', component: () => import('@/views/music/local.vue') },
      { path: 'recent', name: 'recent', component: () => import('@/views/music/recent.vue') },
      { path: 'search', name: 'search', component: () => import('@/views/music/search.vue') },
      { path: 'recognize', name: 'recognize', component: () => import('@/views/music/recognize.vue') },
      { path: 'list/:id', name: 'list', component: () => import('@/views/music/list.vue') },
      { path: 'singer/:id', name: 'singer', component: () => import('@/views/music/singer.vue') },
      { path: 'download', name: 'download', component: () => import('@/views/download/index.vue') },
      { path: 'local/edit-tag', name: 'local-tag-editor', component: () => import('@/views/music/LocalTagEditorPage.vue') },
      { path: 'profile', name: 'profile', component: () => import('@/views/user/Profile.vue') }
    ]
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('@/views/settings/index.vue')
  }
]

const routes: RouteRecordRaw[] = [
  { path: '/', children: appRouter },
  { path: '/desktop-lyric', name: 'desktop-lyric', component: () => import('@/views/DeskTopLyric/DeskTopLyric.vue') },
  { path: '/recognition-worker', name: 'recognition-worker', component: () => import('@/views/music/RecognitionWorker.vue') }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
} as RouterOptions)

// View Transition API integration for route changes
let pendingTransitionResolve: (() => void) | null = null
let activeTransition: ViewTransition | null = null
let transitionTimeoutId: ReturnType<typeof setTimeout> | null = null

function cleanupRouteTransition() {
  delete document.documentElement.dataset.routeTransition
  activeTransition = null
  if (transitionTimeoutId) {
    clearTimeout(transitionTimeoutId)
    transitionTimeoutId = null
  }
}

router.beforeEach((to, from) => {
  if (pendingTransitionResolve) {
    pendingTransitionResolve()
    pendingTransitionResolve = null
  }

  const toDepth = routeDepths[to.name as string] ?? 0
  const fromDepth = routeDepths[from.name as string] ?? 0
  routeDirection.value = toDepth >= fromDepth ? 'forward' : 'backward'

  // Sub-route changes within /home use Vue <Transition> instead of
  // View Transition API to avoid ::view-transition pseudo-elements
  // breaking backdrop-filter on the fixed player bar.
  const isHomeSubRoute = (p: string) => p.startsWith('/home/') && p !== '/home'
  if (isHomeSubRoute(to.path) && isHomeSubRoute(from.path)) {
    return
  }

  if (
    !from.name ||
    !('startViewTransition' in document) ||
    to.path === from.path ||
    to.path.includes('desktop-lyric') ||
    to.path.includes('recognition-worker') ||
    to.path.includes('settings') ||
    from.path.includes('settings')
  ) {
    return
  }

  document.documentElement.dataset.routeTransition = routeDirection.value

  activeTransition = document.startViewTransition(() =>
    new Promise<void>((resolve) => {
      pendingTransitionResolve = resolve
    })
  )

  transitionTimeoutId = setTimeout(() => {
    if (activeTransition) {
      try { activeTransition.skipTransition() } catch {}
      cleanupRouteTransition()
    }
  }, 800)

  activeTransition.finished.finally(cleanupRouteTransition)
})

router.afterEach(() => {
  nextTick(() => {
    if (pendingTransitionResolve) {
      pendingTransitionResolve()
      pendingTransitionResolve = null
    }
  })
})

router.onError(() => {
  if (pendingTransitionResolve) {
    pendingTransitionResolve()
    pendingTransitionResolve = null
  }
  cleanupRouteTransition()
})

const getRoutePreloadEnabled = (): boolean => {
  try {
    const saved = localStorage.getItem('appSettings')
    if (saved) {
      const parsed = JSON.parse(saved) as { routePreloadEnabled?: boolean }
      if (typeof parsed.routePreloadEnabled === 'boolean') return parsed.routePreloadEnabled
    }
  } catch {}
  return true
}

function flattenRoutes(rs: RouteRecordRaw[]): RouteRecordRaw[] {
  const result: RouteRecordRaw[] = []
  for (const r of rs) {
    result.push(r)
    if (r.children) result.push(...flattenRoutes(r.children))
  }
  return result
}

const startPreload = () => {
  if (!getRoutePreloadEnabled()) return
  const idleCb = window.requestIdleCallback || ((cb: IdleRequestCallback) => window.setTimeout(cb, 200))
  const queue = flattenRoutes(routes).filter((r): r is RouteRecordRaw & { component: () => Promise<any> } =>
    !!r.component && typeof r.component === 'function'
  )

  const runBatch = () => {
    if (!getRoutePreloadEnabled()) return
    const route = queue.shift()
    if (!route) return
    try { route.component() } catch {}
    idleCb(runBatch)
  }

  const schedule = () => idleCb(runBatch)
  if (document.readyState === 'complete') setTimeout(schedule, 1500)
  else window.addEventListener('load', () => setTimeout(schedule, 1500), { once: true })
}
startPreload()

export default router
