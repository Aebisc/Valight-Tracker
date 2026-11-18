"use client";

import { useEffect, useState, useCallback } from "react";

export function useTheme() {
  const [theme, setTheme] = useState<"dark" | "light">("dark");

  useEffect(() => {
    const saved = localStorage.getItem("theme");
    const isLight = saved === "light" || document.documentElement.classList.contains("light");
    const activeTheme: "dark" | "light" = isLight ? "light" : "dark";
    setTheme(activeTheme);
    document.documentElement.classList.toggle("light", isLight);
  }, []);

  const toggle = useCallback(() => {
    setTheme((prev) => {
      const next = prev === "dark" ? "light" : "dark";
      localStorage.setItem("theme", next);
      document.documentElement.classList.toggle("light", next === "light");
      return next;
    });
  }, []);

  return { theme, toggle };
}

export function useSettings() {
  const { theme, toggle: toggleTheme } = useTheme();

  return {
    theme,
    toggleTheme,
  };
}
