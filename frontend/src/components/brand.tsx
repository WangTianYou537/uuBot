import { Link } from "react-router-dom";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/** Uppercase mono micro-label used to head sections. */
export function Eyebrow({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return <p className={cn("eyebrow", className)}>{children}</p>;
}

/** The brand, set as a compact dictionary headword. */
export function Wordmark({ to = "/app" }: { to?: string }) {
  return (
    <Link to={to} className="group flex items-baseline gap-2">
      <span className="headword text-xl">uuBot</span>
      <span className="phonetic hidden text-xs sm:inline">ˈjuːbɒt</span>
    </Link>
  );
}

/** Page masthead: an eyebrow over a large serif title, with optional aside. */
export function Masthead({
  eyebrow,
  title,
  aside,
}: {
  eyebrow: string;
  title: ReactNode;
  aside?: ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-end justify-between gap-4 border-b border-border pb-5">
      <div className="flex flex-col gap-1.5">
        <Eyebrow>{eyebrow}</Eyebrow>
        <h1 className="headword text-3xl sm:text-4xl">{title}</h1>
      </div>
      {aside && <div className="text-right">{aside}</div>}
    </div>
  );
}
