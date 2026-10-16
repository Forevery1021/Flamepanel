//! 用户类命令：user list / reset-password / reset-2fa（离线直写数据库，面板宕机或忘记密码时可用）。

use std::io::Read;

use serde_json::json;

use crate::domain::repository::UserRepository;
use crate::utils::password::PasswordUtils;

use super::args::Parsed;
use super::context::Ctx;
use super::db::{pick_user, Db};
use super::output::{CliError, Outcome};
use super::system::table;

pub async fn list(ctx: &Ctx) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let db = Db::open(ctx).await?;
    let users = db.users().list().await;
    db.close().await;
    let mut users = users?;
    users.sort_by_key(|u| u.id);
    let rows: Vec<Vec<String>> = users
        .iter()
        .map(|u| {
            vec![
                u.id.to_string(),
                u.username.clone(),
                u.role.clone(),
                if u.must_change_password {
                    "yes".into()
                } else {
                    "no".into()
                },
                u.created_at.format("%Y-%m-%d %H:%M").to_string(),
            ]
        })
        .collect();
    let data: Vec<_> = users
        .iter()
        .map(|u| {
            json!({
                "id": u.id,
                "username": u.username,
                "role": u.role,
                "must_change_password": u.must_change_password,
                "created_at": u.created_at,
            })
        })
        .collect();
    let human = if rows.is_empty() {
        "(no users — finish the setup wizard in the browser)\n".to_string()
    } else {
        table(&["ID", "USERNAME", "ROLE", "MUST_CHANGE", "CREATED"], &rows)
    };
    Ok(Outcome::new(json!({ "users": data }), human))
}

/// 无歧义字符集（去掉 0/O、1/l/I）。
const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789";
const GENERATED_LEN: usize = 16;

/// 用系统 CSPRNG 生成随机密码（拒绝采样消除取模偏差），并保证通过强度校验。
pub fn generate_password(username: &str) -> Result<String, CliError> {
    use ring::rand::{SecureRandom, SystemRandom};
    let rng = SystemRandom::new();
    let limit = (256 / CHARSET.len() * CHARSET.len()) as u8;
    for _ in 0..16 {
        let mut out = String::with_capacity(GENERATED_LEN);
        while out.len() < GENERATED_LEN {
            let mut buf = [0u8; 32];
            rng.fill(&mut buf)
                .map_err(|_| CliError::general("system random generator unavailable"))?;
            for b in buf {
                if b < limit && out.len() < GENERATED_LEN {
                    out.push(CHARSET[(b as usize) % CHARSET.len()] as char);
                }
            }
        }
        if PasswordUtils::validate_strength(&out, Some(username)).is_ok() {
            return Ok(out);
        }
    }
    Err(CliError::general(
        "could not generate a password meeting the policy",
    ))
}

fn read_stdin_password() -> Result<String, CliError> {
    let mut s = String::new();
    std::io::stdin()
        .take(4096)
        .read_to_string(&mut s)
        .map_err(|e| CliError::io("read password from stdin", &e))?;
    let pw = s.strip_suffix('\n').unwrap_or(&s);
    let pw = pw.strip_suffix('\r').unwrap_or(pw);
    if pw.is_empty() {
        return Err(CliError::usage("empty password on stdin"));
    }
    Ok(pw.to_string())
}

pub async fn reset_password(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let db = Db::open(ctx).await?;
    let result = reset_inner(&db, p).await;
    db.close().await;
    result
}

async fn reset_inner(db: &Db, p: &Parsed) -> Result<Outcome, CliError> {
    let repo = db.users();
    let mut user = pick_user(&repo, p.value("user")).await?;
    let force_change = !p.has("no-force-change");
    let from_stdin = p.has("password-stdin");

    if p.dry_run() {
        return Ok(Outcome::new(
            json!({
                "dry_run": true,
                "user": user.username,
                "user_id": user.id,
                "actions": [
                    format!("set a new {} password", if from_stdin { "stdin-provided" } else { "random" }),
                    format!("must_change_password = {force_change}"),
                    "password_changed_at = now (revokes refresh tokens and websocket sessions)",
                ],
            }),
            format!(
                "would reset the password of '{}' (id {}); must change at next login: {force_change}",
                user.username, user.id
            ),
        ));
    }

    let (password, generated) = if from_stdin {
        let pw = read_stdin_password()?;
        PasswordUtils::validate_strength(&pw, Some(&user.username))?;
        (pw, false)
    } else {
        (generate_password(&user.username)?, true)
    };
    user.password_hash = PasswordUtils::hash(&password)?;
    user.must_change_password = force_change;
    repo.update(&user).await?;
    let now = chrono::Utc::now().timestamp();
    repo.update_password_changed_at(user.id, now).await?;

    let mut data = json!({
        "user": user.username,
        "user_id": user.id,
        "generated": generated,
        "must_change_password": force_change,
        "revoked": "refresh tokens and websocket sessions; open access tokens expire within 15 minutes",
    });
    let mut human = format!("password of '{}' has been reset\n", user.username);
    if generated {
        data["password"] = json!(password);
        human.push_str(&format!("new password: {password}\n"));
    }
    if force_change {
        human.push_str("a password change is required at next login\n");
    }
    Ok(Outcome::new(data, human)
        .warn("existing sessions: refresh tokens revoked now; access tokens expire within 15 minutes")
        .warn("the new password works immediately; a running panel applies the must-change flag within 15 s (auth cache)"))
}

/// 离线移除用户的两步验证（认证器与恢复码都丢失时的最后手段）。
pub async fn reset_two_factor(ctx: &Ctx, p: &Parsed) -> Result<Outcome, CliError> {
    ctx.require_install()?;
    let db = Db::open(ctx).await?;
    let result = reset_two_factor_inner(&db, p).await;
    db.close().await;
    result
}

async fn reset_two_factor_inner(db: &Db, p: &Parsed) -> Result<Outcome, CliError> {
    use crate::domain::two_factor::TwoFactorRepository;
    let user = pick_user(&db.users(), p.value("user")).await?;
    let repo = crate::infrastructure::sqlite::SqliteTwoFactorRepository::new(db.pool.clone());
    let enabled = repo.get(user.id).await?.is_some_and(|r| r.enabled);
    if p.dry_run() {
        return Ok(Outcome::new(
            json!({
                "dry_run": true,
                "user": user.username,
                "user_id": user.id,
                "two_factor_enabled": enabled,
                "actions": ["delete the TOTP secret and recovery codes"],
            }),
            format!(
                "would remove two-factor authentication of '{}' (id {}); currently enabled: {enabled}",
                user.username, user.id
            ),
        ));
    }
    let existed = repo.delete(user.id).await?;
    let human = if existed {
        format!(
            "two-factor authentication of '{}' has been removed\n",
            user.username
        )
    } else {
        format!(
            "'{}' had no two-factor authentication configured\n",
            user.username
        )
    };
    Ok(Outcome::new(
        json!({ "user": user.username, "user_id": user.id, "removed": existed }),
        human,
    )
    .warn("a running panel caches 2FA status in memory: restart it (fpctl restart) for the change to take effect"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_passwords_meet_policy_and_differ() {
        let a = generate_password("admin").unwrap();
        let b = generate_password("admin").unwrap();
        assert_eq!(a.len(), GENERATED_LEN);
        assert_ne!(a, b);
        assert!(a.bytes().all(|c| CHARSET.contains(&c)));
        PasswordUtils::validate_strength(&a, Some("admin")).unwrap();
    }
}
