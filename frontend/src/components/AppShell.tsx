import { useEffect, useState, type ReactNode } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { type LucideIcon, LogOut, Menu, X } from "lucide-react";

import { ThemeToggle } from "@/components/ThemeToggle";
import { cn } from "@/lib/utils";

export type NavItem = {
  to: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
};

type Props = {
  /** Shown under the wordmark, e.g. a nickname or "控制台". */
  subtitle?: string;
  brandTo?: string;
  nav: NavItem[];
  onLogout: () => void;
  footnote?: string;
  contentClassName?: string;
  children: ReactNode;
};

/**
 * App shell with a fixed sidebar on large screens and a slide-in drawer on
 * small/medium ones. The dictionary identity lives in the rail: wordmark set
 * as a headword, mono nav labels, a hairline-ruled footer.
 */
export function AppShell({
  subtitle,
  brandTo = "/app",
  nav,
  onLogout,
  footnote = "collected by you · 2026",
  contentClassName,
  children,
}: Props) {
  const [open, setOpen] = useState(false);
  const location = useLocation();

  // Close the drawer on route change.
  useEffect(() => setOpen(false), [location.pathname]);

  // Lock body scroll while the drawer is open.
  useEffect(() => {
    if (open) {
      document.body.style.overflow = "hidden";
      return () => {
        document.body.style.overflow = "";
      };
    }
  }, [open]);

  return (
    <div className="min-h-screen lg:grid lg:grid-cols-[16rem_1fr]">
      {/* ---- Sidebar (fixed on lg, drawer below) ---- */}
      <Sidebar
        subtitle={subtitle}
        brandTo={brandTo}
        nav={nav}
        onLogout={onLogout}
        footnote={footnote}
        drawerOpen={open}
        onClose={() => setOpen(false)}
      />

      {/* ---- Main column ---- */}
      <div className="flex min-h-screen flex-col">
        {/* Mobile/tablet top bar with the menu trigger */}
        <div className="sticky top-0 z-30 flex items-center justify-between border-b border-border bg-background/85 px-4 py-3 backdrop-blur lg:hidden">
          <button
            type="button"
            onClick={() => setOpen(true)}
            className="-ml-1 inline-flex size-9 items-center justify-center rounded-md hover:bg-accent"
            aria-label="打开菜单"
          >
            <Menu className="size-5" />
          </button>
          <Link to={brandTo} className="flex items-baseline gap-2">
            <span className="headword text-lg">uuBot</span>
          </Link>
          <ThemeToggle />
        </div>

        <main
          className={cn(
            "mx-auto w-full max-w-3xl flex-1 px-4 py-8 sm:px-8 sm:py-10",
            contentClassName
          )}
        >
          {children}
        </main>
      </div>
    </div>
  );
}

function Sidebar({
  subtitle,
  brandTo,
  nav,
  onLogout,
  footnote,
  drawerOpen,
  onClose,
}: {
  subtitle?: string;
  brandTo: string;
  nav: NavItem[];
  onLogout: () => void;
  footnote: string;
  drawerOpen: boolean;
  onClose: () => void;
}) {
  return (
    <>
      {/* Scrim for the drawer */}
      <div
        className={cn(
          "fixed inset-0 z-40 bg-black/40 transition-opacity lg:hidden",
          drawerOpen ? "opacity-100" : "pointer-events-none opacity-0"
        )}
        onClick={onClose}
        aria-hidden
      />

      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-50 flex w-64 flex-col border-r border-border bg-card transition-transform duration-200",
          "lg:sticky lg:top-0 lg:z-auto lg:h-screen lg:translate-x-0",
          drawerOpen ? "translate-x-0" : "-translate-x-full"
        )}
      >
        {/* Brand */}
        <div className="flex items-start justify-between px-5 pt-6">
          <Link to={brandTo} className="flex flex-col gap-0.5">
            <span className="headword text-2xl">uuBot</span>
            <span className="phonetic text-xs">ˈjuːbɒt</span>
          </Link>
          <button
            type="button"
            onClick={onClose}
            className="-mr-1 inline-flex size-8 items-center justify-center rounded-md hover:bg-accent lg:hidden"
            aria-label="关闭菜单"
          >
            <X className="size-4" />
          </button>
        </div>

        {subtitle && (
          <p className="mt-4 truncate px-5 font-mono text-xs uppercase tracking-[0.15em] text-muted-foreground">
            {subtitle}
          </p>
        )}

        {/* Nav */}
        <nav className="mt-4 flex flex-1 flex-col gap-1 px-3">
          {nav.map((item) => (
            <NavLink key={item.to} item={item} />
          ))}
        </nav>

        {/* Footer */}
        <div className="border-t border-border p-3">
          <div className="flex items-center justify-between px-2 pb-2">
            <span className="eyebrow">主题</span>
            <ThemeToggle />
          </div>
          <button
            type="button"
            onClick={onLogout}
            className="flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <LogOut className="size-4" />
            退出登录
          </button>
          <p className="px-2.5 pt-3 font-mono text-[0.625rem] text-muted-foreground">
            {footnote}
          </p>
        </div>
      </aside>
    </>
  );
}

function NavLink({ item }: { item: NavItem }) {
  const navigate = useNavigate();
  const { pathname, search } = useLocation();
  const active = `${pathname}${search}` === item.to || (!item.to.includes("?") && pathname === item.to);
  const Icon = item.icon;

  return (
    <button
      type="button"
      onClick={() => navigate(item.to)}
      aria-current={active ? "page" : undefined}
      className={cn(
        "group flex items-center gap-3 rounded-md px-2.5 py-2 text-left transition-colors",
        active
          ? "bg-accent text-foreground"
          : "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
      )}
    >
      <Icon
        className={cn("size-4 shrink-0", active && "text-primary")}
      />
      <span className="flex flex-col leading-tight">
        <span className="text-sm font-medium">{item.label}</span>
        {item.hint && (
          <span className="font-mono text-[0.625rem] uppercase tracking-wide text-muted-foreground">
            {item.hint}
          </span>
        )}
      </span>
      {/* active spine */}
      <span
        className={cn(
          "ml-auto h-4 w-0.5 rounded-full bg-primary transition-opacity",
          active ? "opacity-100" : "opacity-0"
        )}
      />
    </button>
  );
}
