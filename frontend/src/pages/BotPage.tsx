import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, Link2, MessageCircle, Plus, RefreshCw, Trash2 } from "lucide-react";
import { toast } from "sonner";

import { UserShell } from "@/components/UserShell";
import { Eyebrow, Masthead } from "@/components/brand";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  api,
  type BotBindingList,
  type BotConversation,
  type BotConversationList,
  type BotMessageList,
  type BotQrInfo,
  type WxBinding,
} from "@/lib/api";
import { useCurrentUser } from "@/lib/auth";
import { cn } from "@/lib/utils";

export default function BotPage() {
  const { data: user } = useCurrentUser();
  const qc = useQueryClient();
  const [displayName, setDisplayName] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [qrByBinding, setQrByBinding] = useState<Record<number, BotQrInfo>>({});

  const bindings = useQuery({
    queryKey: ["bot-bindings"],
    queryFn: () => api.get<BotBindingList>("/api/bot/bindings"),
  });

  const conversations = useQuery({
    queryKey: ["bot-conversations"],
    queryFn: () => api.get<BotConversationList>("/api/bot/conversations"),
  });

  const selectedConversation = useMemo(() => {
    const items = conversations.data?.items ?? [];
    return items.find((c) => c.id === selectedId) ?? items[0] ?? null;
  }, [conversations.data?.items, selectedId]);

  const messages = useQuery({
    queryKey: ["bot-messages", selectedConversation?.id],
    enabled: !!selectedConversation,
    queryFn: () =>
      api.get<BotMessageList>(
        `/api/bot/conversations/${selectedConversation!.id}/messages?page=1&page_size=100`
      ),
  });

  const createBinding = useMutation({
    mutationFn: () => api.post<WxBinding>("/api/bot/bindings", { display_name: displayName }),
    onSuccess: (binding) => {
      setDisplayName("");
      toast.success("已创建绑定申请");
      qc.invalidateQueries({ queryKey: ["bot-bindings"] });
      qrcode.mutate({ id: binding.id, force: false });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const qrcode = useMutation({
    mutationFn: ({ id, force }: { id: number; force: boolean }) =>
      api.post<BotQrInfo>(`/api/bot/bindings/${id}/qrcode?force=${force}`),
    onSuccess: (data, vars) => {
      setQrByBinding((prev) => ({ ...prev, [vars.id]: data }));
      toast.success(vars.force ? "二维码已更新" : "二维码已生成");
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const revokeBinding = useMutation({
    mutationFn: (id: number) => api.del(`/api/bot/bindings/${id}`),
    onSuccess: () => {
      toast.success("已撤销绑定");
      qc.invalidateQueries({ queryKey: ["bot-bindings"] });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const pendingBindings = bindings.data?.items.filter((b) => b.status === "pending") ?? [];

  useQuery({
    queryKey: ["bot-bindings-poll", pendingBindings.map((b) => b.id).join(",")],
    enabled: pendingBindings.length > 0,
    refetchInterval: 3000,
    queryFn: async () => {
      const data = await api.get<BotBindingList>("/api/bot/bindings");
      qc.setQueryData(["bot-bindings"], data);
      return data;
    },
  });

  if (!user) return null;

  return (
    <UserShell user={user} contentClassName="max-w-6xl">
      <Masthead
        eyebrow="微信 Bot / command line"
        title="微信助手"
        aside={
          <div className="flex flex-col items-end">
            <span className="headword text-2xl text-primary">
              {bindings.data?.total ?? "—"}/{bindings.data?.max_bindings ?? 3}
            </span>
            <Eyebrow>已申请绑定</Eyebrow>
          </div>
        }
      />

      <div className="mt-6 grid gap-4 lg:grid-cols-[22rem_1fr]">
        <section className="flex flex-col gap-4">
          <div className="rounded-md border border-border p-4">
            <Eyebrow>绑定微信</Eyebrow>
            <p className="mt-2 text-sm text-muted-foreground">
              创建申请后点击“显示二维码”，用手机微信扫码完成绑定。二维码会过期，过期后可更换二维码。
            </p>
            <div className="mt-4 flex gap-2">
              <Input
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                placeholder="备注名（可选）"
              />
              <Button
                onClick={() => createBinding.mutate()}
                disabled={createBinding.isPending}
              >
                <Plus />
                申请
              </Button>
            </div>
          </div>

          <div className="rounded-md border border-border p-4">
            <Eyebrow>绑定列表</Eyebrow>
            <div className="mt-3 flex flex-col gap-3">
              {bindings.data?.items.map((b) => {
                const qr = qrByBinding[b.id];
                const isPending = b.status === "pending";
                return (
                  <div key={b.id} className="rounded-sm bg-secondary p-3 text-sm">
                    <div className="flex items-center justify-between gap-2">
                      <div>
                        <p className="font-medium">{b.display_name || "未命名微信"}</p>
                        <p className="font-mono text-xs text-muted-foreground">
                          {b.status === "active" ? "已绑定" : b.status}
                        </p>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        title="撤销绑定"
                        onClick={() => revokeBinding.mutate(b.id)}
                      >
                        <Trash2 />
                      </Button>
                    </div>
                    <div className="mt-2 flex items-center gap-2 font-mono text-xs text-muted-foreground">
                      <Link2 className="size-3" />
                      <span className="truncate">{b.binding_code}</span>
                      <button
                        className="text-primary"
                        onClick={() => {
                          navigator.clipboard.writeText(b.binding_code);
                          toast.success("绑定码已复制");
                        }}
                      >
                        <Copy className="size-3" />
                      </button>
                    </div>
                    {isPending && (
                      <div className="mt-3 rounded-sm border border-border bg-background p-3">
                        {qr?.svg ? (
                          <div
                            className="mx-auto flex max-w-full justify-center overflow-auto rounded-sm bg-white p-2"
                            dangerouslySetInnerHTML={{ __html: qr.svg }}
                          />
                        ) : (
                          <p className="py-10 text-center text-xs text-muted-foreground">
                            生成二维码后，用手机微信扫码完成绑定。
                          </p>
                        )}
                        <div className="mt-3 grid grid-cols-2 gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => qrcode.mutate({ id: b.id, force: false })}
                            disabled={qrcode.isPending}
                          >
                            <MessageCircle />
                            显示二维码
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => qrcode.mutate({ id: b.id, force: true })}
                            disabled={qrcode.isPending}
                          >
                            <RefreshCw />
                            更换二维码
                          </Button>
                        </div>
                        <p className="mt-2 text-xs text-muted-foreground">
                          二维码约 5 分钟过期；过期或扫码失败时点击“更换二维码”。
                        </p>
                      </div>
                    )}
                  </div>
                );
              })}
              {bindings.data?.items.length === 0 && (
                <p className="py-6 text-center text-sm text-muted-foreground">
                  暂无绑定申请
                </p>
              )}
            </div>
          </div>

          <div className="rounded-md border border-border p-4 text-sm">
            <Eyebrow>指令</Eyebrow>
            <ul className="mt-3 flex flex-col gap-2 font-mono text-xs">
              <li>/trans example</li>
              <li>/add (example)</li>
              <li>/add</li>
              <li>/list (-n 10)</li>
            </ul>
          </div>
        </section>

        <section className="grid min-h-[36rem] gap-4 lg:grid-cols-[18rem_1fr]">
          <div className="rounded-md border border-border p-4">
            <Eyebrow>会话</Eyebrow>
            <div className="mt-3 flex flex-col gap-1">
              {conversations.data?.items.map((c) => (
                <ConversationButton
                  key={c.id}
                  conversation={c}
                  active={selectedConversation?.id === c.id}
                  onClick={() => setSelectedId(c.id)}
                />
              ))}
              {conversations.data?.items.length === 0 && (
                <p className="py-12 text-center text-sm text-muted-foreground">
                  微信发来第一条指令后，会话会出现在这里。
                </p>
              )}
            </div>
          </div>

          <div className="rounded-md border border-border p-4">
            <Eyebrow>对话记录</Eyebrow>
            <div className="mt-4 flex max-h-[34rem] flex-col gap-3 overflow-y-auto pr-1">
              {messages.data?.items.map((m) => (
                <div
                  key={m.id}
                  className={cn(
                    "max-w-[85%] rounded-md px-3 py-2 text-sm",
                    m.direction === "inbound"
                      ? "self-end bg-primary text-primary-foreground"
                      : "self-start bg-secondary text-secondary-foreground"
                  )}
                >
                  <p className="whitespace-pre-wrap leading-relaxed">{m.content}</p>
                  <p className="mt-1 font-mono text-[0.65rem] opacity-70">
                    {new Date(m.created_at).toLocaleString()}
                  </p>
                </div>
              ))}
              {!selectedConversation && (
                <div className="flex flex-1 flex-col items-center justify-center gap-3 py-24 text-center text-muted-foreground">
                  <MessageCircle className="size-8" />
                  <p className="text-sm">选择一个会话查看消息。</p>
                </div>
              )}
            </div>
          </div>
        </section>
      </div>
    </UserShell>
  );
}

function ConversationButton({
  conversation,
  active,
  onClick,
}: {
  conversation: BotConversation;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "rounded-sm px-3 py-2 text-left text-sm transition-colors hover:bg-secondary",
        active && "bg-secondary text-secondary-foreground"
      )}
    >
      <p className="truncate font-medium">
        {conversation.external_conversation_id || `会话 ${conversation.id}`}
      </p>
      <p className="mt-1 truncate font-mono text-xs text-muted-foreground">
        last: {conversation.last_translated_term || "—"}
      </p>
    </button>
  );
}
