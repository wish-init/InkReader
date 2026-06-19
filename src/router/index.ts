import { createRouter, createWebHashHistory } from 'vue-router'
import RepositoryPage from '@/pages/RepositoryPage.vue'
import LibraryPage from '@/pages/LibraryPage.vue'
import FavoritesPage from '@/pages/FavoritesPage.vue'
import HistoryPage from '@/pages/HistoryPage.vue'
import MetadataHealthPage from '@/pages/MetadataHealthPage.vue'
import AggregationPage from '@/pages/AggregationPage.vue'
import BookDetailPage from '@/pages/BookDetailPage.vue'
import ReaderPage from '@/pages/ReaderPage.vue'
import SettingsPage from '@/pages/SettingsPage.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/library' },
    { path: '/repositories', component: RepositoryPage },
    { path: '/library', component: LibraryPage },
    { path: '/books/:bookId', component: BookDetailPage, props: true },
    { path: '/favorites', component: FavoritesPage },
    { path: '/authors', component: AggregationPage, props: { mode: 'authors' } },
    { path: '/tags', component: AggregationPage, props: { mode: 'tags' } },
    { path: '/history', component: HistoryPage },
    { path: '/health', component: MetadataHealthPage },
    { path: '/reader/:bookId', component: ReaderPage, props: true },
    { path: '/settings', component: SettingsPage },
  ],
})
