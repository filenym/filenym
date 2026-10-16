import { useEffect, useState } from "react";
export type Theme = "system" | "light" | "dark";
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(() => {
    try {
      const v = localStorage.getItem("filenym-theme");
      return v === "light" || v === "dark" ? v : "system";
    } catch {
      return "system";
    }
  });
  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme =
        theme === "system" ? (media?.matches ? "dark" : "light") : theme;
    };
    apply();
    media?.addEventListener("change", apply);
    try {
      localStorage.setItem("filenym-theme", theme);
    } catch {
      /* Theme still works for this session. */
    }
    return () => media?.removeEventListener("change", apply);
  }, [theme]);
  return { theme, setTheme };
}
