import { ref, onMounted } from "vue";
import type { ThemeMode } from "@/types";

const STORAGE_KEY = "theme";

export function useTheme() {
  const theme = ref<ThemeMode>(
    (localStorage.getItem(STORAGE_KEY) as ThemeMode) || "system"
  );

  function applyTheme(mode: ThemeMode) {
    const root = document.documentElement;

    if (mode === "system") {
      root.removeAttribute("data-theme");
      // Check system preference for .dark class
      const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      if (prefersDark) {
        root.classList.add("dark");
      } else {
        root.classList.remove("dark");
      }
    } else {
      root.setAttribute("data-theme", mode);
      // Add/remove .dark class for shadcn-vue components
      if (mode === "dark") {
        root.classList.add("dark");
      } else {
        root.classList.remove("dark");
      }
    }
  }

  function setTheme(mode: ThemeMode) {
    theme.value = mode;
    localStorage.setItem(STORAGE_KEY, mode);
    applyTheme(mode);
  }

  onMounted(() => {
    applyTheme(theme.value);
  });

  return {
    theme,
    setTheme,
  };
}
