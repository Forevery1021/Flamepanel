//! 最小 zip 读写（零新增依赖：deflate 与 CRC-32 均来自 flate2）。
//!
//! 支持范围——覆盖面板场景的常见 zip，不追求完整规范：
//! - 读：store / deflate 条目，UTF-8 或本地编码文件名（非 UTF-8 按有损转换），Unix 权限与符号链接；
//!   拒绝加密条目与 ZIP64（单文件 ≥ 4 GiB 或条目数 ≥ 65535）；
//! - 写：deflate（目录为 store），UTF-8 文件名标志，Unix 权限写入外部属性；超出 ZIP64 界限时报错，
//!   提示改用 tar.gz。
//!
//! 条目内容的写出（路径校验、体积上限、落盘）由 [`super::archive`] 负责，本模块只做格式解析。
use crate::core::error::AppError;
use std::io::{Read, Seek, SeekFrom, Write};

const SIG_LOCAL: u32 = 0x0403_4b50;
const SIG_CENTRAL: u32 = 0x0201_4b50;
const SIG_EOCD: u32 = 0x0605_4b50;
const SIG_DESCRIPTOR: u32 = 0x0807_4b50;
const FLAG_ENCRYPTED: u16 = 0x0001;
const FLAG_DESCRIPTOR: u16 = 0x0008;
const FLAG_UTF8: u16 = 0x0800;
const METHOD_STORE: u16 = 0;
const METHOD_DEFLATE: u16 = 8;
const S_IFMT: u32 = 0o170000;
const S_IFLNK: u32 = 0o120000;
const S_IFDIR: u32 = 0o040000;

fn bad(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(format!("zip 文件无效: {}", msg.into()))
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// 中央目录中的一个条目。
#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub method: u16,
    pub crc32: u32,
    pub compressed_size: u64,
    pub size: u64,
    /// Unix 模式（含类型位；非 Unix 创建的包为 0）
    pub unix_mode: u32,
    local_offset: u64,
}

impl ZipEntry {
    pub fn is_dir(&self) -> bool {
        self.name.ends_with('/') || self.unix_mode & S_IFMT == S_IFDIR
    }

    pub fn is_symlink(&self) -> bool {
        self.unix_mode & S_IFMT == S_IFLNK
    }

    /// 权限位（去掉 setuid / setgid / sticky，缺省 644 / 755）。
    pub fn permissions(&self) -> u32 {
        match self.unix_mode & 0o777 {
            0 if self.is_dir() => 0o755,
            0 => 0o644,
            m => m,
        }
    }
}

/// 读取中央目录（不解压任何内容）。
pub fn read_entries<R: Read + Seek>(r: &mut R) -> Result<Vec<ZipEntry>, AppError> {
    let len = r
        .seek(SeekFrom::End(0))
        .map_err(|e| bad(format!("读取失败: {e}")))?;
    if len < 22 {
        return Err(bad("文件过小"));
    }
    // EOCD 位于末尾 22 字节 + 最多 65535 字节注释之内
    let tail_len = len.min(22 + 65_535);
    r.seek(SeekFrom::Start(len - tail_len))
        .map_err(|e| bad(format!("读取失败: {e}")))?;
    let mut tail = vec![0u8; tail_len as usize];
    r.read_exact(&mut tail)
        .map_err(|e| bad(format!("读取失败: {e}")))?;
    let pos = (0..=tail.len() - 22)
        .rev()
        .find(|&i| u32_at(&tail, i) == SIG_EOCD)
        .ok_or_else(|| bad("找不到中央目录结尾（不是 zip 或文件已损坏）"))?;
    let eocd = &tail[pos..];
    let total = u16_at(eocd, 10);
    let cd_size = u32_at(eocd, 12);
    let cd_offset = u32_at(eocd, 16);
    if total == 0xFFFF || cd_size == 0xFFFF_FFFF || cd_offset == 0xFFFF_FFFF {
        return Err(AppError::BadRequest(
            "暂不支持 ZIP64 格式（超大 zip），请改用 tar.gz".into(),
        ));
    }
    if u64::from(cd_offset) + u64::from(cd_size) > len {
        return Err(bad("中央目录越界"));
    }
    r.seek(SeekFrom::Start(u64::from(cd_offset)))
        .map_err(|e| bad(format!("读取失败: {e}")))?;
    let mut cd = vec![0u8; cd_size as usize];
    r.read_exact(&mut cd)
        .map_err(|e| bad(format!("读取中央目录失败: {e}")))?;

    let mut entries = Vec::with_capacity(total as usize);
    let mut i = 0usize;
    for _ in 0..total {
        if i + 46 > cd.len() || u32_at(&cd, i) != SIG_CENTRAL {
            return Err(bad("中央目录条目损坏"));
        }
        let made_by_os = cd[i + 5];
        let flags = u16_at(&cd, i + 8);
        let method = u16_at(&cd, i + 10);
        let crc32 = u32_at(&cd, i + 16);
        let csize = u32_at(&cd, i + 20);
        let usize_ = u32_at(&cd, i + 24);
        let name_len = u16_at(&cd, i + 28) as usize;
        let extra_len = u16_at(&cd, i + 30) as usize;
        let comment_len = u16_at(&cd, i + 32) as usize;
        let external = u32_at(&cd, i + 38);
        let offset = u32_at(&cd, i + 42);
        let end = i + 46 + name_len + extra_len + comment_len;
        if end > cd.len() {
            return Err(bad("中央目录条目越界"));
        }
        let raw_name = &cd[i + 46..i + 46 + name_len];
        let name = if flags & FLAG_UTF8 != 0 {
            String::from_utf8(raw_name.to_vec()).map_err(|_| bad("文件名不是合法 UTF-8"))?
        } else {
            String::from_utf8_lossy(raw_name).into_owned()
        };
        if flags & FLAG_ENCRYPTED != 0 {
            return Err(AppError::BadRequest(format!(
                "不支持加密的 zip 条目: {name}"
            )));
        }
        if csize == 0xFFFF_FFFF || usize_ == 0xFFFF_FFFF || offset == 0xFFFF_FFFF {
            return Err(AppError::BadRequest(
                "暂不支持 ZIP64 格式（超大 zip），请改用 tar.gz".into(),
            ));
        }
        if method != METHOD_STORE && method != METHOD_DEFLATE {
            return Err(AppError::BadRequest(format!(
                "不支持的 zip 压缩方式 {method}（仅支持 store / deflate）: {name}"
            )));
        }
        entries.push(ZipEntry {
            // Windows 打包工具可能使用反斜杠分隔
            name: name.replace('\\', "/"),
            method,
            crc32,
            compressed_size: u64::from(csize),
            size: u64::from(usize_),
            // 3 = Unix；其他系统创建的包外部属性不是 Unix 模式
            unix_mode: if made_by_os == 3 { external >> 16 } else { 0 },
            local_offset: u64::from(offset),
        });
        i = end;
    }
    Ok(entries)
}

/// 把条目内容解压写入 `out`，校验 CRC-32 与声明大小；`limit` 为允许写出的最大字节数。
pub fn extract_to<R: Read + Seek, W: Write>(
    r: &mut R,
    entry: &ZipEntry,
    out: &mut W,
    limit: u64,
) -> Result<u64, AppError> {
    if entry.size > limit {
        return Err(AppError::BadRequest(format!(
            "解压体积超出上限: {}",
            entry.name
        )));
    }
    r.seek(SeekFrom::Start(entry.local_offset))
        .map_err(|e| bad(format!("读取失败: {e}")))?;
    let mut lh = [0u8; 30];
    r.read_exact(&mut lh)
        .map_err(|e| bad(format!("读取本地头失败: {e}")))?;
    if u32_at(&lh, 0) != SIG_LOCAL {
        return Err(bad(format!("本地头损坏: {}", entry.name)));
    }
    let skip = u64::from(u16_at(&lh, 26)) + u64::from(u16_at(&lh, 28));
    r.seek(SeekFrom::Current(skip as i64))
        .map_err(|e| bad(format!("读取失败: {e}")))?;
    let raw = r.take(entry.compressed_size);
    let mut reader: Box<dyn Read + '_> = match entry.method {
        METHOD_DEFLATE => Box::new(flate2::read::DeflateDecoder::new(raw)),
        _ => Box::new(raw),
    };
    // 多读 1 字节用于发现「实际内容比声明的大」（压缩炸弹 / 伪造大小）
    let mut limited = (&mut reader).take(entry.size + 1);
    let mut crc = flate2::Crc::new();
    let mut buf = [0u8; 64 * 1024];
    let mut written = 0u64;
    loop {
        let n = limited
            .read(&mut buf)
            .map_err(|e| bad(format!("解压 {} 失败: {e}", entry.name)))?;
        if n == 0 {
            break;
        }
        written += n as u64;
        if written > entry.size {
            return Err(bad(format!("条目实际大小与声明不符: {}", entry.name)));
        }
        crc.update(&buf[..n]);
        out.write_all(&buf[..n])
            .map_err(|e| AppError::internal(format!("写出 {} 失败: {e}", entry.name)))?;
    }
    if written != entry.size || crc.sum() != entry.crc32 {
        return Err(bad(format!("校验失败（CRC / 大小不符）: {}", entry.name)));
    }
    Ok(written)
}

/// 计数写入器：记录已写出的字节偏移（zip 头需要各条目的绝对偏移）。
struct Counting<W: Write> {
    inner: W,
    pos: u64,
}

impl<W: Write> Write for Counting<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

struct CentralRecord {
    name: Vec<u8>,
    flags: u16,
    method: u16,
    crc: u32,
    csize: u32,
    size: u32,
    mode: u32,
    offset: u32,
    dos_time: u16,
    dos_date: u16,
}

/// 流式 zip 写入器。
pub struct ZipWriter<W: Write> {
    out: Counting<W>,
    central: Vec<CentralRecord>,
    dos_time: u16,
    dos_date: u16,
}

fn too_large() -> AppError {
    AppError::BadRequest(
        "内容超出 zip 格式限制（单文件或总计 ≥ 4 GiB / 条目 ≥ 65535），请改用 tar.gz".into(),
    )
}

impl<W: Write> ZipWriter<W> {
    pub fn new(out: W) -> Self {
        use chrono::{Datelike, Timelike};
        let now = chrono::Local::now();
        let dos_time =
            ((now.hour() as u16) << 11) | ((now.minute() as u16) << 5) | (now.second() as u16 / 2);
        let dos_date = (((now.year().max(1980) - 1980) as u16) << 9)
            | ((now.month() as u16) << 5)
            | now.day() as u16;
        Self {
            out: Counting { inner: out, pos: 0 },
            central: Vec::new(),
            dos_time,
            dos_date,
        }
    }

    fn io(e: std::io::Error) -> AppError {
        AppError::internal(format!("写入 zip 失败: {e}"))
    }

    fn offset(&self) -> Result<u32, AppError> {
        u32::try_from(self.out.pos).map_err(|_| too_large())
    }

    fn local_header(&mut self, name: &[u8], flags: u16, method: u16) -> Result<(), AppError> {
        let mut h = Vec::with_capacity(30 + name.len());
        h.extend_from_slice(&SIG_LOCAL.to_le_bytes());
        h.extend_from_slice(&20u16.to_le_bytes());
        h.extend_from_slice(&flags.to_le_bytes());
        h.extend_from_slice(&method.to_le_bytes());
        h.extend_from_slice(&self.dos_time.to_le_bytes());
        h.extend_from_slice(&self.dos_date.to_le_bytes());
        h.extend_from_slice(&[0u8; 12]); // crc / 压缩后 / 原始大小：由数据描述符给出
        h.extend_from_slice(&(name.len() as u16).to_le_bytes());
        h.extend_from_slice(&0u16.to_le_bytes());
        h.extend_from_slice(name);
        self.out.write_all(&h).map_err(Self::io)
    }

    fn check_name(name: &str) -> Result<Vec<u8>, AppError> {
        if name.len() > u16::MAX as usize {
            return Err(AppError::BadRequest(format!("文件名过长: {name}")));
        }
        Ok(name.as_bytes().to_vec())
    }

    /// 目录条目（`name` 以 `/` 结尾）。
    pub fn add_dir(&mut self, name: &str, mode: u32) -> Result<(), AppError> {
        let name = if name.ends_with('/') {
            name.to_string()
        } else {
            format!("{name}/")
        };
        let bytes = Self::check_name(&name)?;
        if self.central.len() >= 0xFFFF {
            return Err(too_large());
        }
        let offset = self.offset()?;
        self.local_header(&bytes, FLAG_UTF8, METHOD_STORE)?;
        // 目录没有内容：不带数据描述符，大小与 CRC 均为 0
        self.central.push(CentralRecord {
            name: bytes,
            flags: FLAG_UTF8,
            method: METHOD_STORE,
            crc: 0,
            csize: 0,
            size: 0,
            mode: S_IFDIR | (mode & 0o777),
            offset,
            dos_time: self.dos_time,
            dos_date: self.dos_date,
        });
        Ok(())
    }

    /// 文件条目：从 `src` 流式读取并 deflate 压缩。
    pub fn add_file<R: Read>(
        &mut self,
        name: &str,
        mode: u32,
        src: &mut R,
    ) -> Result<(), AppError> {
        let bytes = Self::check_name(name)?;
        if self.central.len() >= 0xFFFF {
            return Err(too_large());
        }
        let offset = self.offset()?;
        let flags = FLAG_UTF8 | FLAG_DESCRIPTOR;
        self.local_header(&bytes, flags, METHOD_DEFLATE)?;
        let start = self.out.pos;
        let mut crc = flate2::Crc::new();
        let mut size = 0u64;
        {
            let mut enc =
                flate2::write::DeflateEncoder::new(&mut self.out, flate2::Compression::default());
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = src
                    .read(&mut buf)
                    .map_err(|e| AppError::internal(format!("读取 {name} 失败: {e}")))?;
                if n == 0 {
                    break;
                }
                crc.update(&buf[..n]);
                size += n as u64;
                enc.write_all(&buf[..n]).map_err(Self::io)?;
            }
            enc.finish().map_err(Self::io)?;
        }
        let csize = u32::try_from(self.out.pos - start).map_err(|_| too_large())?;
        let size = u32::try_from(size).map_err(|_| too_large())?;
        let mut d = Vec::with_capacity(16);
        d.extend_from_slice(&SIG_DESCRIPTOR.to_le_bytes());
        d.extend_from_slice(&crc.sum().to_le_bytes());
        d.extend_from_slice(&csize.to_le_bytes());
        d.extend_from_slice(&size.to_le_bytes());
        self.out.write_all(&d).map_err(Self::io)?;
        self.central.push(CentralRecord {
            name: bytes,
            flags,
            method: METHOD_DEFLATE,
            crc: crc.sum(),
            csize,
            size,
            mode: 0o100000 | (mode & 0o777),
            offset,
            dos_time: self.dos_time,
            dos_date: self.dos_date,
        });
        Ok(())
    }

    /// 写中央目录与结尾记录，返回内部写入器。
    pub fn finish(mut self) -> Result<W, AppError> {
        let cd_start = self.offset()?;
        let records = std::mem::take(&mut self.central);
        for c in &records {
            let mut h = Vec::with_capacity(46 + c.name.len());
            h.extend_from_slice(&SIG_CENTRAL.to_le_bytes());
            h.extend_from_slice(&((3u16 << 8) | 20).to_le_bytes()); // made by: Unix, 2.0
            h.extend_from_slice(&20u16.to_le_bytes());
            h.extend_from_slice(&c.flags.to_le_bytes());
            h.extend_from_slice(&c.method.to_le_bytes());
            h.extend_from_slice(&c.dos_time.to_le_bytes());
            h.extend_from_slice(&c.dos_date.to_le_bytes());
            h.extend_from_slice(&c.crc.to_le_bytes());
            h.extend_from_slice(&c.csize.to_le_bytes());
            h.extend_from_slice(&c.size.to_le_bytes());
            h.extend_from_slice(&(c.name.len() as u16).to_le_bytes());
            h.extend_from_slice(&[0u8; 8]); // extra / comment / disk / internal attr
            h.extend_from_slice(&(c.mode << 16).to_le_bytes());
            h.extend_from_slice(&c.offset.to_le_bytes());
            h.extend_from_slice(&c.name);
            self.out.write_all(&h).map_err(Self::io)?;
        }
        let cd_size = self.offset()? - cd_start;
        let n = records.len() as u16;
        let mut e = Vec::with_capacity(22);
        e.extend_from_slice(&SIG_EOCD.to_le_bytes());
        e.extend_from_slice(&[0u8; 4]);
        e.extend_from_slice(&n.to_le_bytes());
        e.extend_from_slice(&n.to_le_bytes());
        e.extend_from_slice(&cd_size.to_le_bytes());
        e.extend_from_slice(&cd_start.to_le_bytes());
        e.extend_from_slice(&0u16.to_le_bytes());
        self.out.write_all(&e).map_err(Self::io)?;
        self.out.flush().map_err(Self::io)?;
        Ok(self.out.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn build(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        w.add_dir("dir", 0o755).unwrap();
        for (name, data) in files {
            w.add_file(name, 0o640, &mut Cursor::new(data.to_vec()))
                .unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn roundtrip_store_dirs_files_and_modes() {
        let big = vec![b'a'; 200_000];
        let zip = build(&[("dir/a.txt", b"hello"), ("中文.md", &big), ("empty", b"")]);
        let mut r = Cursor::new(zip);
        let entries = read_entries(&mut r).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir/", "dir/a.txt", "中文.md", "empty"]);
        assert!(entries[0].is_dir());
        assert_eq!(entries[1].permissions(), 0o640);
        assert!(entries[2].compressed_size < 10_000, "deflate 生效");

        let mut out = Vec::new();
        extract_to(&mut r, &entries[2], &mut out, u64::MAX).unwrap();
        assert_eq!(out, big);
        let mut out = Vec::new();
        extract_to(&mut r, &entries[1], &mut out, u64::MAX).unwrap();
        assert_eq!(out, b"hello");
        let mut out = Vec::new();
        extract_to(&mut r, &entries[3], &mut out, u64::MAX).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn corrupted_data_fails_crc_check() {
        let mut zip = build(&[("a.txt", b"hello world hello world")]);
        let entries = read_entries(&mut Cursor::new(zip.clone())).unwrap();
        // 篡改 a.txt 的压缩数据中间一个字节
        let data_start = entries[1].local_offset as usize + 30 + "a.txt".len();
        zip[data_start + 3] ^= 0x55;
        let mut out = Vec::new();
        let err = extract_to(&mut Cursor::new(zip), &entries[1], &mut out, u64::MAX);
        assert!(err.is_err());
    }

    #[test]
    fn declared_size_limit_is_enforced() {
        let zip = build(&[("a.bin", &[7u8; 4096])]);
        let mut r = Cursor::new(zip);
        let entries = read_entries(&mut r).unwrap();
        let mut out = Vec::new();
        assert!(extract_to(&mut r, &entries[1], &mut out, 1000).is_err());
    }

    #[test]
    fn rejects_non_zip_and_truncated_input() {
        assert!(read_entries(&mut Cursor::new(
            b"not a zip file at all, definitely".to_vec()
        ))
        .is_err());
        let zip = build(&[("a", b"x")]);
        assert!(read_entries(&mut Cursor::new(zip[..zip.len() - 10].to_vec())).is_err());
    }

    #[test]
    fn reads_archives_from_system_zip_when_available() {
        // 与系统 zip 工具互通（CI 容器不一定有 zip，缺失时跳过）
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f.txt"), "from system zip\n").unwrap();
        let ok = std::process::Command::new("zip")
            .current_dir(dir.path())
            .args(["-q", "-X", "out.zip", "f.txt"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return;
        }
        let mut f = std::fs::File::open(dir.path().join("out.zip")).unwrap();
        let entries = read_entries(&mut f).unwrap();
        let mut out = Vec::new();
        extract_to(&mut f, &entries[0], &mut out, u64::MAX).unwrap();
        assert_eq!(out, b"from system zip\n");
    }
}
