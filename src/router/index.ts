import { createRouter, createWebHistory } from 'vue-router'
import DevicesView from '../views/DevicesView.vue'
import DeviceDetailView from '../views/DeviceDetailView.vue'
import DiscoverView from '../views/DiscoverView.vue'
import CommissionByCodeView from '../views/CommissionByCodeView.vue'
import LogsView from '../views/LogsView.vue'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/devices' },
    { path: '/devices', component: DevicesView },
    { path: '/devices/:nodeId', component: DeviceDetailView, props: true },
    { path: '/discover', component: DiscoverView },
    { path: '/commission', component: CommissionByCodeView },
    { path: '/logs', component: LogsView },
  ],
})
