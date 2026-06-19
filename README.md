# uuBot · 单词收藏

一个用 Rust 编写的单词收藏网站：

- **微信注册登录**：通过 [mapay.cn 聚合登录](./api.md)（`type=wx`）注册/登录，首次登录自动创建账号。
- **绑定邮箱**：登录后可在「账户设置」绑定邮箱并可选设置密码。
- **邮箱登录**：绑定后支持「验证码」或「密码」两种方式登录。
- **单词收藏**：单词 / 音标 / 释义 / 例句 / 备注 / 标签，可用 DeeplX 快速取义，也可调用后台配置的 LLM 生成学习型 Markdown 讲解。
- **微信 Bot**：通过 provider-neutral Webhook 接收 `/trans`、`/add`、`/list` 指令；用户可在 Web 端申请绑定微信并查看会话。
- **后台管理面板**：独立管理员账号；可配置 SMTP 发信、微信聚合登录凭据、词典接口，并查看用户与统计、测试发信。
- **多数据库**：SQLite / MySQL / PostgreSQL（改连接串即可切换）。
- **单文件部署**：前端（React + shadcn/ui）在编译期内嵌进二进制（rust-embed）。

## 技术栈

| 层 | 技术 |
|----|------|
| Web | axum + tokio |
| 存储 | SeaORM（sqlite/mysql/postgres） |
| 认证 | JWT（httpOnly Cookie）+ Argon2 |
| 邮件 | lettre（SMTP） |
| 前端 | React + Vite + Tailwind v4 + shadcn/ui（官方 CLI，radix-nova，Geist 字体） |

## 快速开始

```bash
# 1. 构建（编译前端 + 内嵌进二进制）
./build.sh

# 2. 配置（可选，全部有默认值）
cp .env.example .env   # 按需编辑

# 3. 运行
./target/release/uuBot
# 打开 http://localhost:8080
```

首次启动会自动建表并创建管理员账号（默认 `admin` / `admin123`，见 `.env.example`）。
后台地址：`/admin/login`。

## 开发模式

```bash
# 后端（默认 sqlite，监听 8080）
cargo run

# 前端（Vite dev server，自动代理 /api → 8080）
cd frontend && npm install && npm run dev
```

## 配置说明

- **启动配置**（数据库、监听地址、JWT 密钥、初始管理员）：环境变量 / `.env`，见 `.env.example`。
- **运行时配置**（SMTP、微信 appid/appkey、DeeplX、AI 翻译、微信 Bot）：存数据库，在**后台管理面板**中修改，无需重启。

切换数据库只需修改 `DATABASE_URL`：

```
sqlite://data.db?mode=rwc
mysql://user:pass@localhost:3306/uubot
postgres://user:pass@localhost:5432/uubot
```

## 微信登录配置

1. 在后台「系统配置 → 微信聚合登录」填入从 mapay.cn 获取的 **AppID** 与 **AppKey**。
2. 确保 `PUBLIC_BASE_URL` 是浏览器可访问的地址；回调地址为 `${PUBLIC_BASE_URL}/api/auth/wechat/callback`。
3. 前台「微信登录」会跳转到扫码页，扫码后回调自动建号并登录。

## 词典与 AI 翻译

- 「取义」使用 DeeplX Endpoint，默认 `https://api.deeplx.org/translate`，可在后台「系统配置 → 词典查询」改为自建 DeeplX 服务。
- 「AI 翻译」会返回基础摘要字段和完整 Markdown 学习讲解；后台可配置 provider、endpoint、key、模型和系统提示词。
- 新版词条会保存 `input_type`、`difficulty`、`content_markdown`、`source`、`raw_json` 等扩展字段；旧数据库启动时会自动补齐新增列。

## wx-bot 绑定与指令

wx-bot 使用 [`wx-bot-sdk`](https://crates.io/crates/wx-bot-sdk) 接入：用户在网页端申请绑定后，页面会显示微信登录二维码；二维码会过期，可点击“更换二维码”重新生成。扫码确认后，后端会保存 bot 账号凭据并启动长轮询监听，收到消息后自动回复。

1. 管理员进入「系统配置 → 微信 Bot」，启用 wx-bot，设置每个用户最大绑定数。
2. 用户进入「微信 Bot」页面创建绑定申请。
3. 点击“显示二维码”，用手机微信扫码；二维码过期时点击“更换二维码”。
4. 绑定成功后，用户可在同一页面查看微信对话记录。

wx-bot-sdk 默认把账号状态保存在 `.weixin-bot/`，该目录已加入 `.gitignore`，不要提交其中的 token 文件。

支持指令：

- `/trans example`：调用后台配置的 LLM 翻译。
- `/add (example)`：翻译并添加 `example` 到词库；也支持 `/add example`。
- `/add`：把当前会话上一次 `/trans` 的词加入词库；若没有上一次翻译会返回错误。
- `/list (-n 10)`：列出最近词条，默认 10 条，最多 50 条。

### 可选：Webhook 适配

如果后续还要接入其他 wx-bot 平台，也可以继续使用 provider-neutral Webhook：

```http
POST /api/bot/webhook
x-uubot-bot-secret: <后台配置的 Secret>
Content-Type: application/json
```

请求 JSON：

```json
{
  "external_user_id": "wx-bot-account-id",
  "display_name": "微信昵称",
  "avatar": "https://example.com/avatar.png",
  "conversation_id": "private-or-room-id",
  "text": "/trans example"
}
```

响应 JSON：

```json
{
  "reply": "example /ɪɡˈzæmpəl/\n例子；示例\n例句：..."
}
```
