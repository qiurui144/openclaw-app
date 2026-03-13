import { createRouter, createWebHistory } from "vue-router";

const routes = [
  { path: "/",                  name: "welcome",           component: () => import("@/pages/WelcomePage.vue") },
  { path: "/check",             name: "check",             component: () => import("@/pages/SystemCheckPage.vue") },
  { path: "/source",            name: "source",            component: () => import("@/pages/SourcePage.vue") },
  { path: "/clash-disclaimer",  name: "clash-disclaimer",  component: () => import("@/pages/ClashDisclaimerPage.vue") },
  { path: "/clash-config",      name: "clash-config",      component: () => import("@/pages/ClashConfigPage.vue") },
  { path: "/install",           name: "install",           component: () => import("@/pages/InstallConfigPage.vue") },
  { path: "/service",           name: "service",           component: () => import("@/pages/ServiceConfigPage.vue") },
  { path: "/platform",          name: "platform",          component: () => import("@/pages/PlatformIntegrationPage.vue") },
  { path: "/deploy",            name: "deploy",            component: () => import("@/pages/DeploymentPage.vue") },
  { path: "/finish",            name: "finish",            component: () => import("@/pages/FinishPage.vue") },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});
