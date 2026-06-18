import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import { ArrowRight, Loader2, QrCode } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ThemeToggle } from "@/components/ThemeToggle";
import { Eyebrow } from "@/components/brand";
import { api } from "@/lib/api";
import { useCurrentUser, useInvalidateUser } from "@/lib/auth";

export default function LoginPage() {
  const navigate = useNavigate();
  const { data: user } = useCurrentUser();
  const invalidate = useInvalidateUser();

  if (user) navigate("/app", { replace: true });

  const wechat = useMutation({
    mutationFn: () =>
      api.get<{ url: string; qrcode: string }>("/api/auth/wechat/login"),
    onSuccess: (data) => {
      const target = data.qrcode || data.url;
      if (target) window.location.href = target;
      else toast.error("没拿到登录地址，请稍后再试");
    },
    onError: (e: Error) => toast.error(e.message),
  });

  return (
    <div className="min-h-screen lg:grid lg:grid-cols-[1.1fr_1fr]">
      {/* Left — the brand, set as a dictionary entry. */}
      <section className="relative flex flex-col justify-between overflow-hidden border-b border-border bg-card px-6 py-12 sm:px-12 lg:border-r lg:border-b-0 lg:py-16">
        <div className="flex items-center justify-between">
          <Eyebrow>词 · 典 / a self-compiled dictionary</Eyebrow>
          <div className="lg:hidden">
            <ThemeToggle />
          </div>
        </div>

        <article className="my-10 max-w-lg animate-in fade-in slide-in-from-bottom-2 duration-700">
          <div className="flex items-baseline gap-3">
            <h1 className="headword marked text-6xl sm:text-7xl">uuBot</h1>
            <span className="phonetic text-lg">ˈjuːbɒt</span>
          </div>
          <p className="mt-3 font-mono text-xs uppercase tracking-[0.2em] text-muted-foreground">
            noun · 名词
          </p>
          <ol className="senses mt-6 text-[0.95rem] text-foreground/90">
            <li>你亲手编纂的单词本：收录、释义、例句，皆由你定义。</li>
            <li>用微信一键登录，绑定邮箱后亦可凭邮箱出入。</li>
            <li>录入生词时自动取词典释义与音标，省去誊抄。</li>
          </ol>
        </article>

        <p className="font-mono text-xs text-muted-foreground">
          collected by you · since 2026
        </p>
      </section>

      {/* Right — the act of entering. */}
      <section className="flex items-center justify-center px-6 py-12 sm:px-12">
        <div className="w-full max-w-sm">
          <div className="mb-8 hidden items-center justify-between lg:flex">
            <Eyebrow>登录 / sign in</Eyebrow>
            <ThemeToggle />
          </div>

          <Tabs defaultValue="wechat">
            <TabsList className="w-full">
              <TabsTrigger value="wechat">微信</TabsTrigger>
              <TabsTrigger value="email">邮箱</TabsTrigger>
            </TabsList>

            <TabsContent value="wechat" className="pt-6">
              <p className="text-sm leading-relaxed text-muted-foreground">
                用微信扫码登录。首次登录将为你新建词库，无需填写任何信息。
              </p>
              <Button
                className="mt-6 w-full"
                onClick={() => wechat.mutate()}
                disabled={wechat.isPending}
              >
                {wechat.isPending ? <Loader2 className="animate-spin" /> : <QrCode />}
                微信扫码登录
              </Button>
            </TabsContent>

            <TabsContent value="email" className="pt-6">
              <EmailLogin
                onLoggedIn={() => {
                  invalidate();
                  navigate("/app");
                }}
              />
            </TabsContent>
          </Tabs>

          <hr className="rule my-8" />

          <a
            href="/admin/login"
            className="group inline-flex items-center gap-1.5 font-mono text-xs uppercase tracking-[0.15em] text-muted-foreground transition-colors hover:text-foreground"
          >
            管理后台
            <ArrowRight className="size-3 transition-transform group-hover:translate-x-0.5" />
          </a>
        </div>
      </section>
    </div>
  );
}

function EmailLogin({ onLoggedIn }: { onLoggedIn: () => void }) {
  const [mode, setMode] = useState<"code" | "password">("code");
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [password, setPassword] = useState("");

  const sendCode = useMutation({
    mutationFn: () => api.post("/api/auth/email/send-code", { email }),
    onSuccess: () => toast.success("验证码已发送，请查收邮件"),
    onError: (e: Error) => toast.error(e.message),
  });

  const login = useMutation({
    mutationFn: () =>
      api.post("/api/auth/email/login", {
        email,
        code: mode === "code" ? code : undefined,
        password: mode === "password" ? password : undefined,
      }),
    onSuccess: () => {
      toast.success("欢迎回来");
      onLoggedIn();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        login.mutate();
      }}
    >
      <div className="flex flex-col gap-1.5">
        <label className="eyebrow" htmlFor="email">
          邮箱
        </label>
        <Input
          id="email"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="you@example.com"
          required
        />
      </div>

      <Tabs value={mode} onValueChange={(v) => setMode(v as "code" | "password")}>
        <TabsList variant="line" className="w-full justify-start">
          <TabsTrigger value="code">验证码</TabsTrigger>
          <TabsTrigger value="password">密码</TabsTrigger>
        </TabsList>
        <TabsContent value="code" className="pt-4">
          <div className="flex gap-2">
            <Input
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder="6 位验证码"
              inputMode="numeric"
              className="font-mono tracking-widest"
            />
            <Button
              type="button"
              variant="outline"
              onClick={() => sendCode.mutate()}
              disabled={!email || sendCode.isPending}
            >
              {sendCode.isPending && <Loader2 className="animate-spin" />}
              发送
            </Button>
          </div>
        </TabsContent>
        <TabsContent value="password" className="pt-4">
          <Input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="账号密码"
          />
        </TabsContent>
      </Tabs>

      <Button type="submit" disabled={login.isPending} className="mt-1">
        {login.isPending && <Loader2 className="animate-spin" />}
        进入词库
      </Button>
    </form>
  );
}
