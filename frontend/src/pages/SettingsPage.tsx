import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import { Loader2 } from "lucide-react";

import { UserShell } from "@/components/UserShell";
import { Masthead, Eyebrow } from "@/components/brand";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import { useCurrentUser, useInvalidateUser } from "@/lib/auth";

export default function SettingsPage() {
  const { data: user } = useCurrentUser();
  const invalidate = useInvalidateUser();

  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [bindPassword, setBindPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");

  const sendCode = useMutation({
    mutationFn: () => api.post("/api/me/email/send-code", { email }),
    onSuccess: () => toast.success("验证码已发送"),
    onError: (e: Error) => toast.error(e.message),
  });

  const bind = useMutation({
    mutationFn: () =>
      api.post("/api/me/email/bind", {
        email,
        code,
        password: bindPassword || undefined,
      }),
    onSuccess: () => {
      toast.success("邮箱已绑定");
      setCode("");
      setBindPassword("");
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const setPassword = useMutation({
    mutationFn: () => api.post("/api/me/password", { password: newPassword }),
    onSuccess: () => {
      toast.success("密码已更新");
      setNewPassword("");
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  if (!user) return null;

  return (
    <UserShell user={user}>
      <Masthead eyebrow="账户 / account" title="设置" />

        {/* Identity */}
        <Section eyebrow="身份 / identity" title="账户资料">
          <div className="flex items-center gap-4">
            {user.avatar && (
              <img
                src={user.avatar}
                alt=""
                className="size-14 rounded-md object-cover ring-1 ring-border"
              />
            )}
            <div className="min-w-0">
              <p className="headword text-xl">{user.nickname}</p>
              <p className="font-mono text-xs text-muted-foreground">
                {user.email ? user.email : "未绑定邮箱"}
                {user.email_verified && " · 已验证"}
              </p>
            </div>
          </div>
        </Section>

        {/* Email binding */}
        <Section
          eyebrow="凭据 / email"
          title={user.email ? "更换绑定邮箱" : "绑定邮箱"}
          description="绑定后可用邮箱验证码或密码登录。"
        >
          <div className="flex flex-col gap-4">
            <Field label="邮箱">
              <Input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="you@example.com"
              />
            </Field>
            <Field label="验证码">
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
            </Field>
            <Field label="登录密码（可选）">
              <Input
                type="password"
                value={bindPassword}
                onChange={(e) => setBindPassword(e.target.value)}
                placeholder="至少 6 位，留空则不设置"
              />
            </Field>
            <Button
              onClick={() => bind.mutate()}
              disabled={!email || !code || bind.isPending}
              className="self-start"
            >
              {bind.isPending && <Loader2 className="animate-spin" />}
              绑定邮箱
            </Button>
          </div>
        </Section>

        {/* Password */}
        {user.email && (
          <Section
            eyebrow="安全 / password"
            title="设置 / 修改密码"
            description={user.has_password ? "当前已设置密码。" : "当前尚未设置密码。"}
          >
            <div className="flex flex-col gap-4">
              <Field label="新密码">
                <Input
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  placeholder="至少 6 位"
                />
              </Field>
              <Button
                onClick={() => setPassword.mutate()}
                disabled={newPassword.length < 6 || setPassword.isPending}
                className="self-start"
              >
                {setPassword.isPending && <Loader2 className="animate-spin" />}
                保存密码
              </Button>
            </div>
          </Section>
        )}
    </UserShell>
  );
}

function Section({
  eyebrow,
  title,
  description,
  children,
}: {
  eyebrow: string;
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="border-b border-border py-8">
      <Eyebrow>{eyebrow}</Eyebrow>
      <h2 className="headword mt-1 text-xl">{title}</h2>
      {description && (
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      )}
      <div className="mt-5">{children}</div>
    </section>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <label className="eyebrow">{label}</label>
      {children}
    </div>
  );
}
