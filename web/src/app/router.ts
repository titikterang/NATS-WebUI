import { createRouter, createWebHistory } from 'vue-router'

// Every screen has a URL, so a bug report can link the exact view (e.g. /servers/3/jetstream).
// The backend serves index.html for unknown paths, so deep links work on reload.
export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/servers' },
    { path: '/servers', name: 'servers', component: () => import('@/features/servers/ServerListPage.vue') },
    {
      path: '/servers/:id(\\d+)/:tab(overview|jetstream|subscriptions)?',
      name: 'server',
      component: () => import('@/features/servers/ServerDetailPage.vue'),
      props: (r) => ({ id: Number(r.params.id), tab: (r.params.tab as string) || 'overview' }),
    },
    { path: '/clients', name: 'clients', component: () => import('@/features/clients/ClientListPage.vue') },
    {
      path: '/clients/:id(\\d+)',
      name: 'client',
      component: () => import('@/features/clients/ClientPage.vue'),
      props: (r) => ({ id: Number(r.params.id) }),
    },
    { path: '/:pathMatch(.*)*', redirect: '/servers' },
  ],
})
