import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { LayoutDashboard, Send, Settings, Users } from "lucide-react";

import { AppShell, type NavItem } from "@/components/AppShell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Eyebrow } from "@/components/brand";
import { api, ApiError, type AdminSettings, type User } from "@/lib/api";
import { Textarea } from "@/components/ui/textarea";

const ADMIN_NAV: NavItem[] = [
  { to: "/admin", label: "总览", hint: "overview", icon: LayoutDashboard },
  { to: "/admin?tab=settings", label: "系统配置", hint: "settings", icon: Settings },
  { to: "/admin?tab=users", label: "用户管理", hint: "users", icon: Users },
];

export default function AdminPage() {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const tab = new URLSearchParams(window.location.search).get("tab") ?? "settings";

  const me = useQuery({
    queryKey: ["admin-me"],
    queryFn: async () => {
      try {
        return await api.get<{ id: number; username: string }>("/api/admin/me");
      } catch (e) {
        if (e instanceof ApiError && e.status === 401) return null;
        throw e;
      }
    },
  });

  useEffect(() => {
    if (me.isFetched && !me.data) navigate("/admin/login", { replace: true });
  }, [me.isFetched, me.data, navigate]);

  const logout = useMutation({
    mutationFn: () => api.post("/api/admin/logout"),
    onSuccess: () => navigate("/admin/login"),
  });

  if (!me.data) {
    return (
      <div className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">
        正在核验身份…
      </div>
    );
  }

  return (
    <AppShell
      subtitle={`${me.data.username} · 管理员`}
      brandTo="/admin"
      nav={ADMIN_NAV}
      onLogout={() => logout.mutate()}
      footnote="admin console · 2026"
    >
      <Eyebrow>后台管理 / console</Eyebrow>
      <h1 className="headword mt-1 text-3xl sm:text-4xl">总览</h1>

      <Stats />

      <div className="mt-10">
        <Tabs
          value={tab === "users" ? "users" : "settings"}
          onValueChange={(value) =>
            navigate(value === "settings" ? "/admin?tab=settings" : "/admin?tab=users")
          }
        >
          <TabsList variant="line">
            <TabsTrigger value="settings">系统配置</TabsTrigger>
            <TabsTrigger value="users">用户管理</TabsTrigger>
          </TabsList>
          <TabsContent value="settings" className="pt-6">
            <SettingsForm onSaved={() => qc.invalidateQueries()} />
          </TabsContent>
          <TabsContent value="users" className="pt-6">
            <UsersTable />
          </TabsContent>
        </Tabs>
      </div>
    </AppShell>
  );
}

function Stats() {
  const stats = useQuery({
    queryKey: ["admin-stats"],
    queryFn: () =>
      api.get<{
        users: number;
        words: number;
        bot_bindings: number;
        bot_messages: number;
      }>("/api/admin/stats"),
  });
  return (
    <div className="mt-6 grid grid-cols-2 divide-x divide-y divide-border rounded-md border border-border sm:grid-cols-4 sm:divide-y-0">
      <Figure label="注册用户" value={stats.data?.users} />
      <Figure label="收录词条" value={stats.data?.words} />
      <Figure label="Bot 绑定" value={stats.data?.bot_bindings} />
      <Figure label="Bot 消息" value={stats.data?.bot_messages} />
    </div>
  );
}

function Figure({ label, value }: { label: string; value?: number }) {
  return (
    <div className="flex flex-col gap-1 px-5 py-6">
      <span className="headword text-4xl text-primary tabular-nums">
        {value ?? "—"}
      </span>
      <Eyebrow>{label}</Eyebrow>
    </div>
  );
}

function SettingsForm({ onSaved }: { onSaved: () => void }) {
  const { data, isLoading } = useQuery({
    queryKey: ["admin-settings"],
    queryFn: () => api.get<AdminSettings>("/api/admin/settings"),
  });

  const [form, setForm] = useState<AdminSettings | null>(null);
  const [testTo, setTestTo] = useState("");

  useEffect(() => {
    if (data) setForm(data);
  }, [data]);

  const save = useMutation({
    mutationFn: (payload: AdminSettings) =>
      api.put("/api/admin/settings", payload),
    onSuccess: () => {
      toast.success("配置已保存");
      onSaved();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const testSmtp = useMutation({
    mutationFn: () => api.post("/api/admin/test-smtp", { to: testTo }),
    onSuccess: () => toast.success("测试邮件已发送"),
    onError: (e: Error) => toast.error(e.message),
  });

  if (isLoading || !form) {
    return <p className="text-sm text-muted-foreground">正在载入配置…</p>;
  }

  const { smtp, oauth, dictionary: dict, ai, bot } = form;
  const aiHints = {
    claude: {
      endpoint: "https://api.anthropic.com/v1/messages",
      model: "claude-opus-4-8",
      description: "Anthropic Messages API。",
    },
    openai_compatible: {
      endpoint: "https://api.openai.com/v1/chat/completions",
      model: "gpt-4.1-mini",
      description: "OpenAI Chat Completions 兼容接口，如 OpenAI、DeepSeek、Ollama 或 vLLM。",
    },
    gemini: {
      endpoint: "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent",
      model: "gemini-2.5-flash",
      description: "Gemini generateContent REST API；Endpoint 通常已经包含模型名。",
    },
  }[ai.provider];

  return (
    <div className="flex flex-col">
      {/* SMTP */}
      <Block eyebrow="发信 / smtp" title="邮件服务" description="用于发送邮箱验证码与测试邮件。">
        <Check
          checked={smtp.enabled}
          onChange={(v) => setForm({ ...form, smtp: { ...smtp, enabled: v } })}
        >
          启用 SMTP
        </Check>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label="服务器地址">
            <Input
              className="font-mono"
              value={smtp.host}
              onChange={(e) => setForm({ ...form, smtp: { ...smtp, host: e.target.value } })}
              placeholder="smtp.example.com"
            />
          </Field>
          <Field label="端口">
            <Input
              type="number"
              className="font-mono"
              value={smtp.port}
              onChange={(e) =>
                setForm({ ...form, smtp: { ...smtp, port: Number(e.target.value) } })
              }
            />
          </Field>
          <Field label="用户名">
            <Input
              value={smtp.username}
              onChange={(e) =>
                setForm({ ...form, smtp: { ...smtp, username: e.target.value } })
              }
            />
          </Field>
          <Field label="密码 / 授权码">
            <Input
              type="password"
              value={smtp.password}
              onChange={(e) =>
                setForm({ ...form, smtp: { ...smtp, password: e.target.value } })
              }
            />
          </Field>
          <Field label="发件人邮箱">
            <Input
              className="font-mono"
              value={smtp.from_email}
              onChange={(e) =>
                setForm({ ...form, smtp: { ...smtp, from_email: e.target.value } })
              }
            />
          </Field>
          <Field label="发件人名称">
            <Input
              value={smtp.from_name}
              onChange={(e) =>
                setForm({ ...form, smtp: { ...smtp, from_name: e.target.value } })
              }
            />
          </Field>
        </div>
        <Check
          checked={smtp.use_implicit_tls}
          onChange={(v) =>
            setForm({ ...form, smtp: { ...smtp, use_implicit_tls: v } })
          }
        >
          使用隐式 TLS（端口 465；否则用 STARTTLS）
        </Check>
        <div className="flex flex-col gap-2 border-t border-border pt-4 sm:flex-row sm:items-end">
          <Field label="测试收件邮箱" className="flex-1">
            <Input
              className="font-mono"
              value={testTo}
              onChange={(e) => setTestTo(e.target.value)}
              placeholder="test@example.com"
            />
          </Field>
          <Button
            variant="outline"
            onClick={() => testSmtp.mutate()}
            disabled={!testTo || testSmtp.isPending}
          >
            <Send />
            测试发信
          </Button>
        </div>
      </Block>

      {/* OAuth */}
      <Block
        eyebrow="登录 / wechat"
        title="微信聚合登录"
        description="mapay.cn 聚合登录凭据。"
      >
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label="接口地址 (base_url)" className="sm:col-span-2">
            <Input
              className="font-mono"
              value={oauth.base_url}
              onChange={(e) =>
                setForm({ ...form, oauth: { ...oauth, base_url: e.target.value } })
              }
            />
          </Field>
          <Field label="AppID (clientId)">
            <Input
              className="font-mono"
              value={oauth.appid}
              onChange={(e) =>
                setForm({ ...form, oauth: { ...oauth, appid: e.target.value } })
              }
            />
          </Field>
          <Field label="AppKey (secret)">
            <Input
              type="password"
              className="font-mono"
              value={oauth.appkey}
              onChange={(e) =>
                setForm({ ...form, oauth: { ...oauth, appkey: e.target.value } })
              }
            />
          </Field>
        </div>
      </Block>

      {/* Dictionary */}
      <Block
        eyebrow="取义 / dictionary"
        title="词典查询"
        description="URL 模板需包含 {word} 占位符。"
      >
        <Check
          checked={dict.enabled}
          onChange={(v) =>
            setForm({ ...form, dictionary: { ...dict, enabled: v } })
          }
        >
          启用词典查询
        </Check>
        <Field label="URL 模板">
          <Input
            className="font-mono"
            value={dict.url_template}
            onChange={(e) =>
              setForm({
                ...form,
                dictionary: { ...dict, url_template: e.target.value },
              })
            }
          />
        </Field>
      </Block>

      {/* AI */}
      <Block
        eyebrow="翻译 / llm"
        title="AI 翻译"
        description="用于添加词条时生成中文释义、例句、备注与标签。支持 OpenAI 兼容、Claude 与 Gemini。"
      >
        <Check
          checked={ai.enabled}
          onChange={(v) => setForm({ ...form, ai: { ...ai, enabled: v } })}
        >
          启用 AI 翻译
        </Check>
        <Field label="调用方式">
          <select
            className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm shadow-xs outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50"
            value={ai.provider}
            onChange={(e) => {
              const provider = e.target.value as typeof ai.provider;
              const hints = {
                claude: {
                  endpoint: "https://api.anthropic.com/v1/messages",
                  model: "claude-opus-4-8",
                },
                openai_compatible: {
                  endpoint: "https://api.openai.com/v1/chat/completions",
                  model: "gpt-4.1-mini",
                },
                gemini: {
                  endpoint: "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent",
                  model: "gemini-2.5-flash",
                },
              }[provider];
              setForm({
                ...form,
                ai: {
                  ...ai,
                  provider,
                  api_endpoint: hints.endpoint,
                  model: hints.model,
                },
              });
            }}
          >
            <option value="claude">Claude / Anthropic</option>
            <option value="openai_compatible">OpenAI 兼容</option>
            <option value="gemini">Gemini</option>
          </select>
        </Field>
        <Field label="API Endpoint">
          <Input
            className="font-mono"
            value={ai.api_endpoint}
            onChange={(e) =>
              setForm({ ...form, ai: { ...ai, api_endpoint: e.target.value } })
            }
            placeholder={aiHints.endpoint}
          />
        </Field>
        <Field label="API Key">
          <Input
            type="password"
            className="font-mono"
            value={ai.api_key}
            onChange={(e) =>
              setForm({ ...form, ai: { ...ai, api_key: e.target.value } })
            }
          />
        </Field>
        <Field label="模型">
          <Input
            className="font-mono"
            value={ai.model}
            onChange={(e) =>
              setForm({ ...form, ai: { ...ai, model: e.target.value } })
            }
            placeholder={aiHints.model}
          />
          <p className="text-xs text-muted-foreground">{aiHints.description}</p>
        </Field>
        <Field label="翻译系统提示词">
          <Textarea
            value={ai.system_prompt}
            onChange={(e) =>
              setForm({ ...form, ai: { ...ai, system_prompt: e.target.value } })
            }
            rows={5}
          />
        </Field>
      </Block>

      {/* wx-bot */}
      <Block
        eyebrow="微信 / bot"
        title="微信 Bot"
        description="用于接收 wx-bot Webhook 指令，并把翻译、添加、列表结果写回会话。"
      >
        <Check
          checked={bot.enabled}
          onChange={(v) => setForm({ ...form, bot: { ...bot, enabled: v } })}
        >
          启用 wx-bot Webhook
        </Check>
        <Field label="每个用户最大绑定数">
          <Input
            type="number"
            className="font-mono"
            value={bot.max_bindings_per_user}
            onChange={(e) =>
              setForm({
                ...form,
                bot: {
                  ...bot,
                  max_bindings_per_user: Number(e.target.value),
                },
              })
            }
          />
        </Field>
        <Field label="Webhook Secret">
          <Input
            type="password"
            className="font-mono"
            value={bot.webhook_secret}
            onChange={(e) =>
              setForm({ ...form, bot: { ...bot, webhook_secret: e.target.value } })
            }
            placeholder="外部 wx-bot 请求 x-uubot-bot-secret"
          />
          <p className="text-xs text-muted-foreground">
            外部服务调用 /api/bot/webhook 时需要携带 x-uubot-bot-secret 请求头。
          </p>
        </Field>
      </Block>

      <div className="pt-6">
        <Button onClick={() => save.mutate(form)} disabled={save.isPending}>
          保存全部配置
        </Button>
      </div>
    </div>
  );
}

function UsersTable() {
  const { data, isLoading } = useQuery({
    queryKey: ["admin-users"],
    queryFn: () =>
      api.get<{ items: User[]; total: number }>(
        "/api/admin/users?page=1&page_size=100"
      ),
  });

  if (isLoading) return <p className="text-sm text-muted-foreground">正在载入…</p>;

  return (
    <div className="overflow-x-auto rounded-md border border-border">
      <table className="w-full min-w-[36rem] text-sm">
        <thead>
          <tr className="border-b border-border">
            <Th>ID</Th>
            <Th>昵称</Th>
            <Th>邮箱</Th>
            <Th>注册时间</Th>
          </tr>
        </thead>
        <tbody>
          {data?.items.map((u) => (
            <tr key={u.id} className="border-b border-border last:border-0">
              <td className="px-4 py-3 font-mono text-xs text-muted-foreground">
                {u.id}
              </td>
              <td className="px-4 py-3 font-medium">{u.nickname}</td>
              <td className="px-4 py-3 font-mono text-xs">
                {u.email ?? <span className="text-muted-foreground">未绑定</span>}
              </td>
              <td className="px-4 py-3 font-mono text-xs text-muted-foreground">
                {new Date(u.created_at).toLocaleString()}
              </td>
            </tr>
          ))}
          {data?.items.length === 0 && (
            <tr>
              <td colSpan={4} className="px-4 py-12 text-center text-sm text-muted-foreground">
                暂无用户
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

function Th({ children }: { children: React.ReactNode }) {
  return (
    <th className="px-4 py-3 text-left">
      <span className="eyebrow">{children}</span>
    </th>
  );
}

function Block({
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
    <section className="border-b border-border py-8 first:pt-0">
      <Eyebrow>{eyebrow}</Eyebrow>
      <h2 className="headword mt-1 text-xl">{title}</h2>
      {description && (
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      )}
      <div className="mt-5 flex flex-col gap-4">{children}</div>
    </section>
  );
}

function Field({
  label,
  className,
  children,
}: {
  label: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div className={`flex flex-col gap-1.5 ${className ?? ""}`}>
      <label className="eyebrow">{label}</label>
      {children}
    </div>
  );
}

function Check({
  checked,
  onChange,
  children,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  children: React.ReactNode;
}) {
  return (
    <label className="flex cursor-pointer items-center gap-2 text-sm">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="size-4 accent-primary"
      />
      {children}
    </label>
  );
}
