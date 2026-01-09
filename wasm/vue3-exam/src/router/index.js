import { createRouter, createWebHistory } from 'vue-router'
import Home from '../views/Home.vue'
import Simple from '../views/Simple.vue'
import Worklet from '../views/Worklet.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: Home
    },
    {
      path: '/simple',
      name: 'simple',
      component: Simple
    },
    {
      path: '/worklet',
      name: 'worklet',
      component: Worklet
    }
  ]
})

export default router

