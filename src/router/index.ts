import { createRouter, createWebHistory } from 'vue-router'
import DevicesView from '../views/DevicesView.vue'
import DeviceDetailView from '../views/DeviceDetailView.vue'
import CommissionView from '../views/CommissionView.vue'
import ControlView from '../views/ControlView.vue'
import LogsView from '../views/LogsView.vue'
import SettingsView from '../views/SettingsView.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/devices' },
    { path: '/devices', component: DevicesView },
    { path: '/devices/:nodeId', component: DeviceDetailView, props: true },
    { path: '/commission', component: CommissionView },
    { path: '/control', component: ControlView },
    { path: '/logs', component: LogsView },
    { path: '/settings', component: SettingsView },
  ],
})
