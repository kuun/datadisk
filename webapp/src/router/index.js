import {createRouter, createWebHistory} from 'vue-router'
import HomeView from '../views/HomeView.vue'
import LoginView from '../views/LoginView.vue'
import SettingsView from '../views/SettingsView.vue'
import UserSettings from '../views/settings/UserSettings.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: HomeView,
      children: [
        {
          path: 'ui/file',
          name: 'file',
          component: () => import('../views/FileView.vue'),
          redirect: { name: 'recent' },  // 添加重定向
          children: [
            {
              path: 'mydocs',  // 修改路径
              name: 'mydocs',
              component: () => import('../views/file/MyDocsView.vue')
            },
            {
              path: 'recent',  // 修改路径
              name: 'recent',
              component: () => import('../views/file/RecentFileView.vue')
            },
            {
              path: 'star',
              name: 'star',
              component: () => import('../views/file/FileStarView.vue')
            },
            {
              path: 'tags',
              name: 'tags',
              component: () => import('../views/file/FileTagsView.vue')
            }
          ]
        },
        {
          path: '/ui/contacts',
          name: 'contacts',
          component: () => import('../views/ContactsView.vue')
        },
        {
          path: "/ui/audit",
          name: 'audit',
          component: () => import('../views/AuditView.vue')
        },
        {
          path: '/ui/group',
          name: 'group',
          component: () => import('../views/GroupView.vue')
        },
        {
          path: '/ui/settings',
          name: 'settings',
          component: SettingsView,
          redirect: '/ui/settings/user',
          children: [
            {
              path: 'user',
              name: 'userSettings',
              component: UserSettings
            }
          ]
        }
      ]
    },
    {
      path: '/ui/login',
      name: 'login',
      component: LoginView
    }
  ]
})

export default router
