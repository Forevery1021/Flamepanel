<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · auth

> 12 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/auth/2fa` | 两步验证状态 | 登录即可 |
| POST | `/api/auth/2fa/disable` | 停用两步验证 | 登录即可 |
| POST | `/api/auth/2fa/enable` | 确认启用：校验待绑定密钥的验证码，返回一次性恢复码 | 登录即可 |
| POST | `/api/auth/2fa/recovery-codes` | 重新生成恢复码（旧码全部作废） | 登录即可 |
| POST | `/api/auth/2fa/setup` | 开始绑定：生成待验证密钥与 otpauth URI（已启用时不影响旧密钥，确认后才替换） | 登录即可 |
| POST | `/api/auth/change-password` | 修改当前用户密码 | 登录即可 |
| POST | `/api/auth/login` | 登录 | 公开（无需登录） |
| POST | `/api/auth/login/2fa` | 登录第二步：提交挑战令牌 + TOTP 验证码（或一次性恢复码）换取会话令牌。 | 公开（无需登录） |
| POST | `/api/auth/logout` | 退出登录：吊销当前 refresh token（加入黑名单），使其无法再换取新 access token。 M4：logout 端点此前被中间件引用但路由缺失（404）；现补全并实现吊销。 | 公开（无需登录） |
| GET | `/api/auth/me` | 当前登录用户信息（前端刷新页面恢复身份） | 登录即可 |
| POST | `/api/auth/refresh` | 刷新令牌：必须使用 Refresh Token（Cookie 或 Bearer），换取新的 Access Token + 轮换 Refresh Token 剩余寿命不足一半时重新签发 Refresh Token（滑动过期）；否则复用原 Refresh Token 刷新令牌（Refresh Token 换新 Access Token + 轮换 Refresh Token） | 公开（无需登录） |
| POST | `/api/auth/rotate-secret` | 轮换 JWT 签名密钥（admin only）：更新后旧 access token 立即失效（短宽限期由客户端刷新换取新 token） | `user:update` |

### GET `/api/auth/2fa`

两步验证状态

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`status`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 当前用户的两步验证状态 | TwoFactorStatusResponse |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "enabled": false,
  "enabled_at": "2026-01-01T00:00:00Z",
  "pending": false,
  "recovery_codes_remaining": 0,
  "required": false
}
```

</details>

```bash
fpctl api GET '/api/auth/2fa' --json
```

### POST `/api/auth/2fa/disable`

停用两步验证

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`disable`

**请求体**（`application/json`，必填）：TwoFactorConfirmRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `code` | string | 是 | 6 位验证码（停用时也可填恢复码） |
| `password` | string | 是 | 当前密码 |

```json
{
  "code": "string",
  "password": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已停用 |  |
| 400 | 密码或验证码错误 |  |
| 401 | 未认证 |  |
| 409 | 面板强制两步验证，不可停用 |  |
| 429 | 尝试过于频繁 |  |

```bash
fpctl api POST '/api/auth/2fa/disable' -d '{"code":"string","password":"string"}' --json
```

### POST `/api/auth/2fa/enable`

确认启用：校验待绑定密钥的验证码，返回一次性恢复码

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`enable`

**请求体**（`application/json`，必填）：TwoFactorEnableRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `code` | string | 是 | 认证器 App 显示的 6 位验证码 |

```json
{
  "code": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已启用；恢复码仅此一次返回 | RecoveryCodesResponse |
| 400 | 未开始绑定或验证码错误 |  |
| 401 | 未认证 |  |
| 429 | 尝试过于频繁 |  |

<details><summary>响应示例</summary>

```json
{
  "recovery_codes": [
    "string"
  ]
}
```

</details>

```bash
fpctl api POST '/api/auth/2fa/enable' -d '{"code":"string"}' --json
```

### POST `/api/auth/2fa/recovery-codes`

重新生成恢复码（旧码全部作废）

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`regenerate_recovery_codes`

**请求体**（`application/json`，必填）：TwoFactorConfirmRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `code` | string | 是 | 6 位验证码（停用时也可填恢复码） |
| `password` | string | 是 | 当前密码 |

```json
{
  "code": "string",
  "password": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 新恢复码（仅此一次返回） | RecoveryCodesResponse |
| 400 | 密码或验证码错误 / 未启用 |  |
| 401 | 未认证 |  |
| 429 | 尝试过于频繁 |  |

<details><summary>响应示例</summary>

```json
{
  "recovery_codes": [
    "string"
  ]
}
```

</details>

```bash
fpctl api POST '/api/auth/2fa/recovery-codes' -d '{"code":"string","password":"string"}' --json
```

### POST `/api/auth/2fa/setup`

开始绑定：生成待验证密钥与 otpauth URI（已启用时不影响旧密钥，确认后才替换）

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`setup`

**请求体**（`application/json`，必填）：TwoFactorSetupRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `password` | string | 是 | 当前密码（二次确认身份） |

```json
{
  "password": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 待绑定密钥 | TwoFactorSetup |
| 400 | 密码错误 |  |
| 401 | 未认证 |  |
| 429 | 尝试过于频繁 |  |

<details><summary>响应示例</summary>

```json
{
  "otpauth_uri": "string",
  "secret": "string"
}
```

</details>

```bash
fpctl api POST '/api/auth/2fa/setup' -d '{"password":"string"}' --json
```

### POST `/api/auth/change-password`

修改当前用户密码

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`change_password`

**请求体**（`application/json`，必填）：ChangePasswordBody

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `new_password` | string | 是 | 需满足密码强度策略，且不得与用户名相同 |
| `old_password` | string | 是 |  |

```json
{
  "new_password": "string",
  "old_password": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 修改成功；该用户其他会话的 refresh token 失效 |  |
| 400 | 旧密码错误或新密码不满足强度策略 |  |
| 401 | 未认证 |  |

```bash
fpctl api POST '/api/auth/change-password' -d '{"new_password":"string","old_password":"string"}' --json
```

### POST `/api/auth/login`

登录

- 认证：无
- 权限：公开（无需登录）
- operationId：`login`

**请求体**（`application/json`，必填）：LoginRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `password` | string | 是 |  |
| `username` | string | 是 |  |

```json
{
  "password": "string",
  "username": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 登录成功 | LoginResponse |
| 401 | 凭据错误 |  |
| 429 | 尝试过于频繁 |  |

<details><summary>响应示例</summary>

```json
{
  "challenge": "string",
  "must_change_password": false,
  "must_setup_two_factor": false,
  "refresh_token": "string",
  "role": "string",
  "token": "string",
  "two_factor_required": false,
  "username": "string"
}
```

</details>

### POST `/api/auth/login/2fa`

登录第二步：提交挑战令牌 + TOTP 验证码（或一次性恢复码）换取会话令牌。

失败计入与密码登录相同的「用户名 + IP」锁定计数；挑战令牌签发后若用户改过密码则失效。

- 认证：无
- 权限：公开（无需登录）
- operationId：`login_two_factor`

**请求体**（`application/json`，必填）：LoginTwoFactorRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `challenge` | string | 是 | 第一步返回的挑战令牌 |
| `code` | string | 是 | 6 位 TOTP 验证码或一次性恢复码 |

```json
{
  "challenge": "string",
  "code": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 登录成功 | LoginResponse |
| 401 | 挑战令牌无效 / 过期，或验证码错误 |  |
| 429 | 尝试过于频繁 |  |

<details><summary>响应示例</summary>

```json
{
  "challenge": "string",
  "must_change_password": false,
  "must_setup_two_factor": false,
  "refresh_token": "string",
  "role": "string",
  "token": "string",
  "two_factor_required": false,
  "username": "string"
}
```

</details>

### POST `/api/auth/logout`

退出登录：吊销当前 refresh token（加入黑名单），使其无法再换取新 access token。
M4：logout 端点此前被中间件引用但路由缺失（404）；现补全并实现吊销。

- 认证：无
- 权限：公开（无需登录）
- operationId：`logout`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 退出成功 |  |
| 401 | 令牌无效/缺失 |  |

### GET `/api/auth/me`

当前登录用户信息（前端刷新页面恢复身份）

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`me`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 当前用户 | MeResponse |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "id": 0,
  "must_change_password": false,
  "role": "string",
  "username": "string"
}
```

</details>

```bash
fpctl api GET '/api/auth/me' --json
```

### POST `/api/auth/refresh`

刷新令牌：必须使用 Refresh Token（Cookie 或 Bearer），换取新的 Access Token + 轮换 Refresh Token
剩余寿命不足一半时重新签发 Refresh Token（滑动过期）；否则复用原 Refresh Token
刷新令牌（Refresh Token 换新 Access Token + 轮换 Refresh Token）

- 认证：无
- 权限：公开（无需登录）
- operationId：`refresh`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 刷新成功 | LoginResponse |
| 401 | 令牌无效/过期 |  |

<details><summary>响应示例</summary>

```json
{
  "challenge": "string",
  "must_change_password": false,
  "must_setup_two_factor": false,
  "refresh_token": "string",
  "role": "string",
  "token": "string",
  "two_factor_required": false,
  "username": "string"
}
```

</details>

### POST `/api/auth/rotate-secret`

轮换 JWT 签名密钥（admin only）：更新后旧 access token 立即失效（短宽限期由客户端刷新换取新 token）

- 认证：Bearer JWT
- 权限：`user:update`
- operationId：`rotate_secret`

**请求体**（`application/json`，必填）：RotateSecretBody

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `secret` | string | 是 | 新密钥（不少于 32 字节） |

```json
{
  "secret": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 轮换成功，旧 access token 立即失效 |  |
| 400 | 密钥过短 |  |
| 401 | 未认证 |  |
| 403 | 无权限 |  |

```bash
fpctl api POST '/api/auth/rotate-secret' -d '{"secret":"string"}' --json
```
