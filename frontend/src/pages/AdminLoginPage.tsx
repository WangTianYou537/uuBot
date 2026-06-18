import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import { ArrowLeft, Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ThemeToggle } from "@/components/ThemeToggle";
import { Eyebrow } from "@/components/brand";
import { api } from "@/lib/api";

export default function AdminLoginPage() {
  const navigate = useNavigate();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const login = useMutation({
    mutationFn: () => api.post("/api/admin/login", { username, password }),
    onSuccess: () => navigate("/admin"),
    onError: (e: Error) => toast.error(e.message),
  });

  return (
    <div className="relative flex min-h-screen items-center justify-center px-6 py-12">
      <div className="absolute top-4 right-4">
        <ThemeToggle />
      </div>

      <div className="w-full max-w-sm">
        <Eyebrow>管理后台 / restricted</Eyebrow>
        <h1 className="headword mt-1 text-4xl">控制台</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          仅限管理员。请输入后台账号。
        </p>

        <hr className="rule my-7" />

        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            login.mutate();
          }}
        >
          <div className="flex flex-col gap-1.5">
            <label className="eyebrow">用户名</label>
            <Input
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <label className="eyebrow">密码</label>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </div>
          <Button type="submit" disabled={login.isPending} className="mt-1">
            {login.isPending && <Loader2 className="animate-spin" />}
            进入控制台
          </Button>
        </form>

        <a
          href="/"
          className="group mt-8 inline-flex items-center gap-1.5 font-mono text-xs uppercase tracking-[0.15em] text-muted-foreground transition-colors hover:text-foreground"
        >
          <ArrowLeft className="size-3 transition-transform group-hover:-translate-x-0.5" />
          返回前台
        </a>
      </div>
    </div>
  );
}
