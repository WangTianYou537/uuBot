import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Loader2, Pencil, Plus, Search, Sparkles, Trash2 } from "lucide-react";

import { UserShell } from "@/components/UserShell";
import { Masthead, Eyebrow } from "@/components/brand";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  api,
  type AiTranslationResult,
  type Word,
  type WordList,
} from "@/lib/api";
import { useCurrentUser } from "@/lib/auth";

export default function WordsPage() {
  const { data: user } = useCurrentUser();
  const qc = useQueryClient();
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Word | null>(null);
  const [creating, setCreating] = useState(false);

  const words = useQuery({
    queryKey: ["words", query],
    queryFn: () =>
      api.get<WordList>(
        `/api/words?page=1&page_size=100&q=${encodeURIComponent(query)}`
      ),
  });

  const del = useMutation({
    mutationFn: (id: number) => api.del(`/api/words/${id}`),
    onSuccess: () => {
      toast.success("已从词库移除");
      qc.invalidateQueries({ queryKey: ["words"] });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  if (!user) return null;
  const total = words.data?.total ?? 0;

  return (
    <UserShell user={user} contentClassName="max-w-5xl">
      <Masthead
        eyebrow="我的词库 / my lexicon"
        title="词库"
        aside={
          <div className="flex flex-col items-end">
            <span className="headword text-2xl text-primary">{total}</span>
            <Eyebrow>已收录词条</Eyebrow>
          </div>
        }
      />

      {/* Look-up bar */}
      <div className="mt-6 flex items-center gap-2">
        <form
          className="relative flex-1"
          onSubmit={(e) => {
            e.preventDefault();
            setQuery(search);
          }}
        >
          <Search className="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="pl-9"
            placeholder="检索词条、释义或标签…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </form>
        <Button onClick={() => setCreating(true)} className="shrink-0">
          <Plus />
          <span className="hidden sm:inline">收录新词</span>
          <span className="sm:hidden">收录</span>
        </Button>
      </div>

      {/* Entries */}
      <div className="mt-2">
        {words.isLoading ? (
          <p className="py-20 text-center text-sm text-muted-foreground">
            正在翻检词库…
          </p>
        ) : words.data && words.data.items.length > 0 ? (
          <ul className="divide-y divide-border">
            {words.data.items.map((w) => (
              <Entry
                key={w.id}
                word={w}
                onEdit={() => setEditing(w)}
                onDelete={() => {
                  if (confirm(`从词库移除「${w.term}」？`)) del.mutate(w.id);
                }}
              />
            ))}
          </ul>
        ) : (
          <EmptyState
            searching={query.length > 0}
            onAdd={() => setCreating(true)}
          />
        )}
      </div>

      {creating && (
        <WordDialog
          onClose={() => setCreating(false)}
          onSaved={() => {
            setCreating(false);
            qc.invalidateQueries({ queryKey: ["words"] });
          }}
        />
      )}
      {editing && (
        <WordDialog
          word={editing}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            qc.invalidateQueries({ queryKey: ["words"] });
          }}
        />
      )}
    </UserShell>
  );
}

function Entry({
  word,
  onEdit,
  onDelete,
}: {
  word: Word;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const senses = word.definition
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  const tags = word.tags
    .split(",")
    .map((t) => t.trim())
    .filter(Boolean);

  return (
    <li className="group grid grid-cols-[1fr_auto] gap-x-4 py-6">
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h2 className="headword text-2xl sm:text-3xl">{word.term}</h2>
        {word.phonetic && <span className="phonetic">{word.phonetic}</span>}
      </div>

      {/* Quiet actions, revealed on hover/focus */}
      <div className="flex items-start gap-0.5 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
        <Button variant="ghost" size="icon-sm" onClick={onEdit} title="编辑">
          <Pencil />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onDelete}
          title="移除"
          className="text-muted-foreground hover:text-destructive"
        >
          <Trash2 />
        </Button>
      </div>

      <div className="col-span-2 mt-3 flex flex-col gap-3">
        {senses.length > 1 ? (
          <ol className="senses text-[0.95rem]">
            {senses.map((s, i) => (
              <li key={i}>{s}</li>
            ))}
          </ol>
        ) : senses.length === 1 ? (
          <p className="text-[0.95rem] leading-relaxed">{senses[0]}</p>
        ) : (
          <p className="text-sm text-muted-foreground">（暂无释义）</p>
        )}

        {word.example && (
          <p className="border-l-2 border-primary/50 pl-3 font-display text-[0.95rem] italic text-foreground/80">
            {word.example}
          </p>
        )}

        {word.note && (
          <p className="text-sm text-muted-foreground">
            <span className="eyebrow mr-2">注</span>
            {word.note}
          </p>
        )}

        {word.content_markdown && (
          <details className="rounded-md border border-border bg-secondary/40 p-3">
            <summary className="cursor-pointer text-sm font-medium">完整讲解</summary>
            <div className="mt-3 whitespace-pre-wrap font-mono text-xs leading-relaxed text-foreground/85">
              {word.content_markdown}
            </div>
          </details>
        )}

        {(word.input_type || word.difficulty || word.source) && (
          <div className="flex flex-wrap gap-1.5">
            {[word.input_type, word.difficulty, word.source].filter(Boolean).map((t) => (
              <span
                key={t}
                className="rounded-sm border border-border px-2 py-0.5 font-mono text-xs text-muted-foreground"
              >
                {t}
              </span>
            ))}
          </div>
        )}

        {tags.length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {tags.map((t) => (
              <span
                key={t}
                className="rounded-sm bg-secondary px-2 py-0.5 font-mono text-xs text-secondary-foreground"
              >
                {t}
              </span>
            ))}
          </div>
        )}
      </div>
    </li>
  );
}

function EmptyState({
  searching,
  onAdd,
}: {
  searching: boolean;
  onAdd: () => void;
}) {
  return (
    <div className="flex flex-col items-center gap-4 py-24 text-center">
      <p className="headword text-3xl text-muted-foreground">
        {searching ? "查无此词" : "空白的扉页"}
      </p>
      <p className="max-w-sm text-sm text-muted-foreground">
        {searching
          ? "换个关键词试试，或直接把它收录进词库。"
          : "你收录的第一个词会出现在这里，从此由你来定义它。"}
      </p>
      <Button onClick={onAdd} variant="outline" className="mt-2">
        <Plus />
        收录新词
      </Button>
    </div>
  );
}

function WordDialog({
  word,
  onClose,
  onSaved,
}: {
  word?: Word;
  onClose: () => void;
  onSaved: () => void;
}) {
  const isEdit = !!word;
  const [term, setTerm] = useState(word?.term ?? "");
  const [phonetic, setPhonetic] = useState(word?.phonetic ?? "");
  const [definition, setDefinition] = useState(word?.definition ?? "");
  const [example, setExample] = useState(word?.example ?? "");
  const [note, setNote] = useState(word?.note ?? "");
  const [tags, setTags] = useState(word?.tags ?? "");
  const [inputType, setInputType] = useState(word?.input_type ?? "");
  const [difficulty, setDifficulty] = useState(word?.difficulty ?? "");
  const [contentMarkdown, setContentMarkdown] = useState(word?.content_markdown ?? "");
  const [source, setSource] = useState(word?.source ?? "manual");
  const [rawJson, setRawJson] = useState(word?.raw_json ?? "");

  const aiTranslate = useMutation({
    mutationFn: () =>
      api.post<AiTranslationResult>("/api/words/ai-translate", { term }),
    onSuccess: (d) => {
      if (d.phonetic) setPhonetic(d.phonetic);
      if (d.definition) setDefinition(d.definition);
      if (d.example) setExample(d.example);
      if (d.note) setNote(d.note);
      if (d.tags) setTags(d.tags);
      if (d.input_type) setInputType(d.input_type);
      if (d.difficulty) setDifficulty(d.difficulty);
      if (d.content_markdown) setContentMarkdown(d.content_markdown);
      if (d.raw_json) setRawJson(d.raw_json);
      setSource("ai");
      toast.success("AI 翻译已填入词条");
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const save = useMutation({
    mutationFn: () => {
      const payload = {
        term,
        phonetic,
        definition,
        example,
        note,
        tags,
        input_type: inputType,
        difficulty,
        content_markdown: contentMarkdown,
        source,
        raw_json: rawJson,
      };
      return isEdit
        ? api.put(`/api/words/${word!.id}`, payload)
        : api.post("/api/words", { ...payload, auto_lookup: false });
    },
    onSuccess: () => {
      toast.success(isEdit ? "已更新词条" : "已收录");
      onSaved();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <Eyebrow>{isEdit ? "编辑词条" : "收录新词"}</Eyebrow>
          <DialogTitle className="headword text-2xl">
            {term || "新词条"}
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <Field label="词条">
            <div className="flex gap-2">
              <Input
                value={term}
                onChange={(e) => setTerm(e.target.value)}
                className="font-display text-base"
                autoFocus
              />
              <Button
                type="button"
                variant="outline"
                title="调用后台配置的 LLM 翻译"
                onClick={() => aiTranslate.mutate()}
                disabled={!term || aiTranslate.isPending}
              >
                {aiTranslate.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <Sparkles />
                )}
                AI 翻译
              </Button>
            </div>
          </Field>

          <Field label="音标">
            <Input
              value={phonetic}
              onChange={(e) => setPhonetic(e.target.value)}
              className="font-mono"
              placeholder="ˈeksəmpl"
            />
          </Field>

          <Field label="释义（每行一条义项）">
            <Textarea
              value={definition}
              onChange={(e) => setDefinition(e.target.value)}
              rows={3}
            />
          </Field>

          <Field label="例句">
            <Textarea
              value={example}
              onChange={(e) => setExample(e.target.value)}
              rows={2}
              className="font-display italic"
            />
          </Field>

          <Field label="备注">
            <Input value={note} onChange={(e) => setNote(e.target.value)} />
          </Field>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <Field label="类型">
              <Input value={inputType} onChange={(e) => setInputType(e.target.value)} placeholder="word" />
            </Field>
            <Field label="难度">
              <Input value={difficulty} onChange={(e) => setDifficulty(e.target.value)} placeholder="中级" />
            </Field>
            <Field label="来源">
              <Input value={source} onChange={(e) => setSource(e.target.value)} className="font-mono" />
            </Field>
          </div>

          <Field label="完整讲解 Markdown">
            <Textarea
              value={contentMarkdown}
              onChange={(e) => setContentMarkdown(e.target.value)}
              rows={6}
              className="max-h-72 min-h-36 resize-y overflow-y-auto font-mono text-xs"
            />
          </Field>

          <Field label="标签（逗号分隔）">
            <Input
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              className="font-mono"
              placeholder="四级, 动词"
            />
          </Field>
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            取消
          </Button>
          <Button onClick={() => save.mutate()} disabled={!term || save.isPending}>
            {save.isPending && <Loader2 className="animate-spin" />}
            {isEdit ? "保存" : "收录"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
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
