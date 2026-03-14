import { useRouter } from "vue-router";
import { useWizardStore } from "@/stores/wizard";

const ROUTE_ORDER_BASE = [
  "welcome", "check", "source", "install", "service", "ai-token", "platform", "deploy", "finish",
];

export function useWizardNav() {
  const router = useRouter();
  const wizard = useWizardStore();

  function routeOrder() {
    if (wizard.sourceMode === "online") {
      return [
        "welcome", "check", "source", "clash-disclaimer", "clash-config",
        "install", "service", "ai-token", "platform", "deploy", "finish",
      ];
    }
    return ROUTE_ORDER_BASE;
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
