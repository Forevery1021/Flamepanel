//! 通用辅助：路径组件白名单、表单校验、RAII 互斥锁守卫、系统资源探测。

use super::*;

/// F-05: 校验将拼入文件系统路径的用户输入（package_key / container_name）。
/// 白名单：字母数字、`_`、`-`、`.`；首字符必须为字母数字；拒绝 `..` 片段；长度 <= 63。
/// 路径组件字符白名单（F-05 / M-A 同一威胁模型）：被应用备份还原等多个编排
/// 复用（`pub(crate)`），保证「同一条安装目录路径在多处被校验」时规则完全一致。
pub(crate) fn sanitize_path_component(name: &str, field: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest(format!("{field} cannot be empty")));
    }
    if name.len() > 63 {
        return Err(AppError::BadRequest(format!("{field} too long (max 63)")));
    }
    if name.contains("..") {
        return Err(AppError::BadRequest(format!("{field} cannot contain '..'")));
    }
    let first = name.chars().next().unwrap_or('0');
    if !first.is_ascii_alphanumeric() {
        return Err(AppError::BadRequest(format!(
            "{field} must start with an alphanumeric character"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(AppError::BadRequest(format!(
            "{field} contains invalid characters (allowed: alphanumerics, '_', '-', '.')"
        )));
    }
    Ok(())
}

/// 所有退出路径（含 TaskStore 失败和请求丢弃）都释放包安装互斥锁。
pub(super) struct InstallLockGuard<'a> {
    pub(super) service: &'a AppStoreService,
    pub(super) key: &'a str,
}

impl Drop for InstallLockGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut locks) = self.service.install_locks.lock() {
            locks.remove(self.key);
        }
    }
}

/// RAII：作用域结束自动释放源的同步互斥锁。
pub(super) struct SyncLockGuard<'a> {
    pub(super) service: &'a AppStoreService,
    pub(super) id: i64,
}

impl Drop for SyncLockGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut locks) = self.service.sync_locks.lock() {
            locks.remove(&self.id);
        }
    }
}

/// 一次远程源同步的结果（M-A Issue 1.2）。
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SyncReport {
    /// 本次新增包数
    pub added: usize,
    /// 已存在跳过数（幂等）
    pub skipped: usize,
    /// 本次索引到的包 key
    pub packages: Vec<String>,
}

/// 校验表单字段：required + pattern + 长度
pub fn validate_fields(
    fields: &[FormField],
    values: &HashMap<String, String>,
) -> Result<(), AppError> {
    // Doc/33 B-2：表单值会被原样代入 compose 模板（VariableMapper 不做 YAML 转义），
    // 含换行即可注入任意 compose 键（如追加 `privileged: true` / 宿主挂载）。
    // 对**所有**提交值（含表单未声明的键）统一拒绝控制字符；键名收敛为环境变量字符集。
    for (key, value) in values {
        if key.is_empty()
            || key.len() > 64
            || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(AppError::ValidationError(format!("非法表单字段名: {key}")));
        }
        if value.chars().any(|c| c.is_control()) {
            return Err(AppError::ValidationError(format!(
                "字段 [{key}] 不能包含换行或其他控制字符"
            )));
        }
    }
    for field in fields {
        let value = values.get(&field.env_key).map(|s| s.as_str()).unwrap_or("");
        if field.required && value.is_empty() {
            return Err(AppError::BadRequest(format!(
                "字段 [{}] 为必填项",
                field.label_zh
            )));
        }
        if value.is_empty() {
            continue;
        }
        if let Some(pattern) = &field.pattern {
            let re = regex::Regex::new(pattern)
                .map_err(|e| AppError::internal(format!("校验规则无效: {}", e)))?;
            if !re.is_match(value) {
                return Err(AppError::BadRequest(format!(
                    "字段 [{}] 格式不正确",
                    field.label_zh
                )));
            }
        }
        if let Some(min) = field.min {
            if let Ok(num) = value.parse::<i64>() {
                if num < min {
                    return Err(AppError::BadRequest(format!(
                        "字段 [{}] 不能小于 {}",
                        field.label_zh, min
                    )));
                }
            }
        }
        if let Some(max) = field.max {
            if let Ok(num) = value.parse::<i64>() {
                if num > max {
                    return Err(AppError::BadRequest(format!(
                        "字段 [{}] 不能大于 {}",
                        field.label_zh, max
                    )));
                }
            }
        }
        if let Some(min_len) = field.min_length {
            if value.len() < min_len {
                return Err(AppError::BadRequest(format!(
                    "字段 [{}] 长度不能少于 {}",
                    field.label_zh, min_len
                )));
            }
        }
        if let Some(max_len) = field.max_length {
            if value.len() > max_len {
                return Err(AppError::BadRequest(format!(
                    "字段 [{}] 长度不能超过 {}",
                    field.label_zh, max_len
                )));
            }
        }
    }
    Ok(())
}

pub fn short_uuid() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

pub(super) fn copy_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)
            .map_err(|e| AppError::internal(format!("创建目录失败: {}", e)))?;
        for entry in std::fs::read_dir(src)
            .map_err(|e| AppError::internal(format!("读取目录失败: {}", e)))?
        {
            let entry = entry.map_err(|e| AppError::internal(format!("读取目录失败: {}", e)))?;
            let file_type = entry
                .file_type()
                .map_err(|e| AppError::internal(format!("读取类型失败: {}", e)))?;
            if file_type.is_symlink() {
                // 不跟随符号链接：包内 `x -> /etc/shadow` 会把宿主文件内容复制进商店目录
                continue;
            }
            if file_type.is_dir() {
                copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                std::fs::copy(entry.path(), dst.join(entry.file_name()))
                    .map_err(|e| AppError::internal(format!("复制文件失败: {}", e)))?;
            }
        }
    }
    Ok(())
}

/// 当前系统资源（用于原生安装决策与预设）
#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemResources {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub is_ssd: bool,
}

impl SystemResources {
    pub fn detect() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        Self {
            cpu_cores: sys.cpus().len().max(1) as u32,
            memory_mb: sys.total_memory() / 1024,
            is_ssd: false,
        }
    }
}
