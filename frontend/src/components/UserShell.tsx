import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { BookMarked, MessageCircle, SlidersHorizontal } from "lucide-react";
import type { ReactNode } from "react";

import { AppShell, type NavItem } from "@/components/AppShell";
import { cn } from "@/lib/utils";
import { api, type User } from "@/lib/api";
import { useInvalidateUser } from "@/lib/auth";

const USER_NAV: NavItem[] = [
  { to: "/app", label: "词库", hint: "lexicon", icon: BookMarked },
  { to: "/bot", label: "微信 Bot", hint: "wx-bot", icon: MessageCircle },
  { to: "/settings", label: "设置", hint: "account", icon: SlidersHorizontal },
];

/** Shell for signed-in user pages, with the lexicon/account sidebar. */
export function UserShell({
  user,
  contentClassName,
  children,
}: {
  user: User;
  contentClassName?: string;
  children: ReactNode;
}) {
  const navigate = useNavigate();
  const invalidate = useInvalidateUser();

  const logout = useMutation({
    mutationFn: () => api.post("/api/auth/logout"),
    onSuccess: () => {
      invalidate();
      navigate("/");
    },
  });

  return (
    <AppShell
      subtitle={user.nickname}
      brandTo="/app"
      nav={USER_NAV}
      onLogout={() => logout.mutate()}
      contentClassName={cn("max-w-4xl", contentClassName)}
    >
      {children}
    </AppShell>
  );
}
