import { createRouter, createWebHistory } from 'vue-router'
import HomePage from '../views/HomePage.vue'
import MainPage from '../views/MainPage.vue'
import LogExpressionPage from '../views/LogExpressionPage.vue'
import CertQueryPage from '../views/CertQueryPage.vue'
import RobotFeedbackPage from '../views/RobotFeedbackPage.vue'

const routes = [
 {
 path: '/',
 name: 'Home',
 component: HomePage
 },
 {
 path: '/encrypt',
 name: 'EncryptTool',
 component: MainPage
 },
 {
 path: '/log-expression',
 name: 'LogExpressionTool',
 component: LogExpressionPage
 },
 {
 path: '/cert-query',
 name: 'CertQueryTool',
 component: CertQueryPage
 },
 {
 path: '/robot-feedback',
 name: 'RobotFeedbackTool',
 component: RobotFeedbackPage
 }
]

const router = createRouter({
 history: createWebHistory(),
 routes
})

export default router
