import {
  createContext, useCallback, useContext, useEffect, useMemo, useRef, useState,
  type ReactNode,
} from "react";
import {
  derive, getDatabase, type Database, type Derived, type Filters,
} from "../data/engine";
import { resolveRange, type DateRange, type PresetId } from "../lib/format";

type Theme = "light" | "dark";

interface AppState {
  theme: Theme;
  toggleTheme: () => void;
  route: string;
  navigate: (id: string) => void;
  collapsed: boolean;
  toggleCollapsed: () => void;
  mobileNav: boolean;
  setMobileNav: (v: boolean) => void;
  filters: Filters;
  setPreset: (p: PresetId) => void;
  setCustomRange: (from: Date, to: Date) => void;
  patchFilters: (p: Partial<Omit<Filters, "range">>) => void;
  resetFilters: () => void;
  db: Database;
  data: Derived;
  loading: boolean;
  reducedMotion: boolean;
  signedIn: boolean;
  signOut: () => void;
  signIn: () => void;
}

const Ctx = createContext<AppState | null>(null);

const defaultFilters = (): Filters => ({
  range: resolveRange("thisMonth"),
  branchId: "all",
  accountId: "all",
  categoryId: "all",
  txType: "all",
  userId: "all",
});

export function AppProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>(() => {
    if (typeof window === "undefined") return "light";
    const saved = localStorage.getItem("np-theme");
    if (saved === "dark" || saved === "light") return saved;
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  });
  const [route, setRoute] = useState("dashboard");
  const [collapsed, setCollapsed] = useState(
    () => typeof window !== "undefined" && window.innerWidth > 0 && window.innerWidth < 1400,
  );
  const [mobileNav, setMobileNav] = useState(false);
  const [filters, setFilters] = useState<Filters>(defaultFilters);
  const [loading, setLoading] = useState(true);
  const [signedIn, setSignedIn] = useState(true);
  const loadTimer = useRef<number | null>(null);

  const reducedMotion = useMemo(
    () => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    [],
  );

  const db = useMemo(() => getDatabase(), []);
  const data = useMemo(() => derive(db, filters), [db, filters]);

  // initial skeleton
  useEffect(() => {
    const t = window.setTimeout(() => setLoading(false), 750);
    return () => window.clearTimeout(t);
  }, []);

  const flashLoading = useCallback(() => {
    if (reducedMotion) return;
    if (loadTimer.current) window.clearTimeout(loadTimer.current);
    setLoading(true);
    loadTimer.current = window.setTimeout(() => setLoading(false), 420);
  }, [reducedMotion]);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem("np-theme", theme);
  }, [theme]);

  const toggleTheme = useCallback(() => setTheme((t) => (t === "light" ? "dark" : "light")), []);

  const navigate = useCallback((id: string) => {
    setRoute(id);
    setMobileNav(false);
    window.scrollTo({ top: 0, behavior: "instant" as ScrollBehavior });
  }, []);

  const toggleCollapsed = useCallback(() => setCollapsed((c) => !c), []);

  const setPreset = useCallback(
    (p: PresetId) => {
      if (p === "custom") return; // handled by setCustomRange popover
      setFilters((f) => ({ ...f, range: resolveRange(p) }));
      flashLoading();
    },
    [flashLoading],
  );

  const setCustomRange = useCallback(
    (from: Date, to: Date) => {
      setFilters((f) => ({ ...f, range: { preset: "custom", from, to: to < from ? from : to } as DateRange }));
      flashLoading();
    },
    [flashLoading],
  );

  const patchFilters = useCallback(
    (p: Partial<Omit<Filters, "range">>) => {
      setFilters((f) => ({ ...f, ...p }));
      flashLoading();
    },
    [flashLoading],
  );

  const resetFilters = useCallback(() => {
    setFilters(defaultFilters());
    flashLoading();
  }, [flashLoading]);

  const value: AppState = {
    theme, toggleTheme,
    route, navigate,
    collapsed, toggleCollapsed,
    mobileNav, setMobileNav,
    filters, setPreset, setCustomRange, patchFilters, resetFilters,
    db, data, loading, reducedMotion,
    signedIn, signOut: () => setSignedIn(false), signIn: () => setSignedIn(true),
  };

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useApp(): AppState {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useApp must be used inside AppProvider");
  return ctx;
}
