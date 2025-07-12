import { createRouter, createWebHistory } from 'vue-router';
import Login from '../components/Login.vue';
import WorkOrder from '../service/workorder.vue';

const routes = [
  {
    path: '/',
    redirect: '/login',
  },
  {
    path: '/login',
    name: 'Login',
    component: Login,
  },
  {
    path: '/workorder',
    name: 'WorkOrder',
    component: WorkOrder,
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;