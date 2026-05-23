use bcrypt::{hash, DEFAULT_COST};
use clap::{Parser, Subcommand};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::infrastructure::{SqliteUserRepository, UserRepository};
use crate::domain::User;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BANNER: &str = r#"
   ___ _                       ___         _
  / __\ | __ _ _ __ ___   ___ / _ \___ ___| |
 / _\ | |/ _` | '_ ` _ \ / _ \ /_)/ __/ _ \ |
/ /   | | (_| | | | | | |  __/ __/ (_|  __/ |
\/    |_|\__,_|_| |_| |_|\___\/   \___\___|_|
"#;

#[derive(Parser)]
#[command(
    name = "flamepanel",
    about = "Flamepanel - Server Operations Management Panel CLI",
    long_about = None,
    disable_help_subcommand = true,
    version = VERSION,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 显示版本信息
    Version,
    /// 显示当前配置
    Config,
    /// 查看用户信息
    UserInfo {
        /// 用户名 (默认: admin)
        #[arg(default_value = "admin")]
        username: String,
    },
    /// 列出所有用户
    UserList,
    /// 重置用户密码
    ResetPassword {
        /// 用户名
        username: String,
        /// 新密码 (留空则自动生成随机密码)
        password: Option<String>,
    },
    /// 检查服务运行状态
    Status,
}

pub async fn run(cli: Cli) {
    match cli.command {
        None => {
            // 无子命令时启动服务器
            crate::start_server().await;
        }
        Some(cmd) => match cmd {
            Commands::Version => cmd_version(),
            Commands::Config => cmd_config(),
            Commands::UserInfo { username } => cmd_user_info(username).await,
            Commands::UserList => cmd_user_list().await,
            Commands::ResetPassword { username, password } => {
                cmd_reset_password(username, password).await;
            }
            Commands::Status => cmd_status().await,
        },
    }
}

// ─── version ───────────────────────────────────────────────────────────────────

fn cmd_version() {
    println!("{BANNER}");
    println!("Flamepanel v{VERSION}");
    println!("Rust 运维管理面板");
    println!();
    println!("Repository: https://github.com/Forevery1021/Flamepanel");
}

// ─── config ────────────────────────────────────────────────────────────────────

fn cmd_config() {
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("配置加载失败: {e}");
            std::process::exit(1);
        }
    };

    println!("{BANNER}");
    println!("当前配置:");
    println!("  ─────────────────────────────────");
    println!("  监听端口:     {}", cfg.port);
    println!("  数据库路径:   {}", cfg.database_url);
    println!("  管理员用户名: {}", cfg.admin_username);
    println!("  ─────────────────────────────────");
    println!();

    if cfg.jwt_secret == "your-super-secret-jwt-key-change-in-production" {
        println!("  ⚠ 警告: JWT 密钥为默认值，请通过 OP_JWT_SECRET 环境变量修改！");
    }
    if cfg.admin_password == "admin123" {
        println!("  ⚠ 警告: 管理员密码为默认值，请及时修改！");
    }
}

// ─── user-info ─────────────────────────────────────────────────────────────────

async fn cmd_user_info(username: String) {
    let (db, _cfg) = connect_db().await;
    let repo = SqliteUserRepository::new(db);

    match repo.find_by_username(&username).await {
        Ok(Some(user)) => print_user(&user),
        Ok(None) => {
            eprintln!("用户 '{}' 不存在", username);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("查询失败: {e}");
            std::process::exit(1);
        }
    }
}

// ─── user-list ─────────────────────────────────────────────────────────────────

async fn cmd_user_list() {
    let (db, _cfg) = connect_db().await;
    let repo = SqliteUserRepository::new(db);

    match repo.list().await {
        Ok(users) => {
            if users.is_empty() {
                println!("暂无用户");
                return;
            }
            println!("{0: <5} {1: <20} {2: <10} {3: <20}", "ID", "用户名", "角色", "创建时间");
            println!("{}", "-".repeat(60));
            for u in &users {
                println!(
                    "{0: <5} {1: <20} {2: <10} {3: <20}",
                    u.id,
                    u.username,
                    u.role,
                    u.created_at.format("%Y-%m-%d %H:%M:%S")
                );
            }
            if let Some(last) = users.last() {
                println!();
                if let Some(ref login) = last.last_login {
                    println!("最近登录: {}", login.format("%Y-%m-%d %H:%M:%S"));
                }
            }
        }
        Err(e) => {
            eprintln!("查询失败: {e}");
            std::process::exit(1);
        }
    }
}

// ─── reset-password ────────────────────────────────────────────────────────────

async fn cmd_reset_password(username: String, password: Option<String>) {
    let (db, _cfg) = connect_db().await;
    let repo = SqliteUserRepository::new(db);

    let user = match repo.find_by_username(&username).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            eprintln!("用户 '{}' 不存在", username);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("查询失败: {e}");
            std::process::exit(1);
        }
    };

    let new_password = password.unwrap_or_else(|| generate_password());

    if new_password.len() < 6 {
        eprintln!("密码长度不能少于 6 位");
        std::process::exit(1);
    }

    let pw_for_hash = new_password.clone();
    let password_hash = tokio::task::spawn_blocking(move || hash(&pw_for_hash, DEFAULT_COST))
        .await
        .expect("密码哈希线程失败")
        .expect("密码哈希失败");

    match repo.update_password(user.id, &password_hash).await {
        Ok(()) => {
            println!("用户 '{}' 的密码已重置", username);
            println!("新密码: {}", new_password);
            println!();
            println!("请妥善保管新密码，建议首次登录后修改。");
        }
        Err(e) => {
            eprintln!("密码重置失败: {e}");
            std::process::exit(1);
        }
    }
}

// ─── status ────────────────────────────────────────────────────────────────────

async fn cmd_status() {
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("配置加载失败: {e}");
            std::process::exit(1);
        }
    };

    // 检查端口是否在监听
    let addr = format!("127.0.0.1:{}", cfg.port);
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(_) => {
            // 端口未被占用，服务未运行
            println!("Flamepanel 服务未运行 (端口 {} 空闲)", cfg.port);
        }
        Err(_) => {
            // 端口已被占用，服务可能正在运行
            println!("Flamepanel 服务正在运行 (端口 {} 已监听)", cfg.port);

            // 尝试请求 API 确认
            if let Ok(resp) = reqwest::get(format!("http://{}/api/system/info", addr)).await {
                if resp.status().is_success() {
                    println!("API 响应正常");
                } else {
                    println!("端口已占用但非 Flamepanel 服务 (HTTP {})", resp.status());
                }
            } else {
                println!("端口已占用但无法确认是否为 Flamepanel");
            }
        }
    }
}

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

async fn connect_db() -> (SqlitePool, Config) {
    let cfg = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("配置加载失败: {e}");
            std::process::exit(1);
        }
    };

    std::fs::create_dir_all("data").ok();

    let db = match sqlx::SqlitePool::connect(&cfg.database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("数据库连接失败: {e}");
            std::process::exit(1);
        }
    };

    // 确保表存在（运行迁移）
    if let Err(e) = sqlx::migrate!().run(&db).await {
        eprintln!("数据库迁移失败: {e}");
        std::process::exit(1);
    }

    // 确保管理员存在
    if let Err(e) = crate::application::seed_admin(&db, &cfg).await {
        eprintln!("初始化管理员失败: {e}");
        std::process::exit(1);
    }

    (db, cfg)
}

fn generate_password() -> String {
    use rand::Rng;
    let chars: Vec<char> = "abcdefghjkmnpqrstuvwxyzABCDEFGHJKMNPQRSTUVWXYZ23456789!@#$%"
        .chars()
        .collect();
    let mut rng = rand::thread_rng();
    (0..16).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

fn print_user(user: &User) {
    println!("用户信息:");
    println!("  ID:         {}", user.id);
    println!("  用户名:     {}", user.username);
    println!("  角色:       {}", user.role);
    println!(
        "  创建时间:   {}",
        user.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    if let Some(ref login) = user.last_login {
        println!("  最近登录:   {}", login.format("%Y-%m-%d %H:%M:%S"));
    }
    println!(
        "  密码哈希:   {}...",
        &user.password_hash.chars().take(20).collect::<String>()
    );
}
