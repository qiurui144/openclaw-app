import { useRouter } from "vue-router";
import { useWizardStore } from "@/stores/wizard";

const INSTALL_ROUTES = [
  "welcome", "check", "activation", "source", "install", "service", "ai-token", "platform", "deploy", "finish",
];

const INSTALL_ONLINE_ROUTES = [
  "welcome", "check", "activation", "source", "clash-disclaimer", "clash-config",
  "install", "service", "ai-token", "platform", "deploy", "finish",
];

const UPDATE_ROUTES = ["welcome", "update", "finish"];

const UNINSTALL_ROUTES = ["welcome", "uninstall"];

export function useWizardNav() {
  const router = useRouter();
  const wizard = useWizardStore();

  function routeOrder(): string[] {
    if (wizard.wizardMode === "update") return UPDATE_ROUTES;
    if (wizard.wizardMode === "uninstall") return UNINSTALL_ROUTES;
    // install mode
    if (wizard.sourceMode === "online") return INSTALL_ONLINE_ROUTES;
    return INSTALL_ROUTES;
  }

  function currentIndex() {
    const name = router.currentRoute.value.name as string;
    return routeOrder().indexOf(name);
  }

  function next() {
    const order = routeOrder();
    const idx = currentIndex();
    if (idx < order.length - 1) {
      router.push({ name: order[idx + 1] });
    }
  }

  function back() {
    const order = routeOrder();
    const idx = currentIndex();
    if (idx > 0) {
      router.push({ name: order[idx - 1] });
    }
  }

  function goTo(name: string) {
    router.push({ name });
  }

  return { next, back, goTo, currentIndex, routeOrder };
}
