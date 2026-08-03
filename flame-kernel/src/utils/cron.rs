use chrono::{DateTime, Datelike, Duration, Months, Timelike, Utc};

/// 标准 5 字段 cron 解析器（分钟 小时 日 月 周几）
///
/// 支持 `*`、`*/n`、`a-b`、`a,b,c` 组合；周几 0/7 均为周日。
/// 日与周几同时受限时按标准 cron 语义取「或」。
#[derive(Debug, Clone)]
pub struct CronSchedule {
    minutes: Vec<u32>,
    hours: Vec<u32>,
    days: Vec<u32>,
    months: Vec<u32>,
    weekdays: Vec<u32>,
}

impl CronSchedule {
    pub fn parse(expr: &str) -> Result<Self, String> {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(format!("expected 5 fields, got {}", fields.len()));
        }
        let mut weekdays = parse_field(fields[4], 0, 7)?;
        if weekdays.contains(&7) {
            if !weekdays.contains(&0) {
                weekdays.push(0);
            }
            weekdays.retain(|w| *w != 7);
        }
        Ok(Self {
            minutes: parse_field(fields[0], 0, 59)?,
            hours: parse_field(fields[1], 0, 23)?,
            days: parse_field(fields[2], 1, 31)?,
            months: parse_field(fields[3], 1, 12)?,
            weekdays,
        })
    }

    pub fn is_valid(expr: &str) -> bool {
        Self::parse(expr).is_ok()
    }

    /// 计算严格晚于 `from` 的下一次执行时间（最多向后搜索 5 年）
    pub fn next_run(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let deadline = from + Duration::days(365 * 5);
        let mut cand = floor_minute(from) + Duration::minutes(1);
        while cand <= deadline {
            let month = cand.month();
            if !self.months.contains(&month) {
                cand = next_month_start(cand);
                continue;
            }
            let day = cand.day();
            let dom_match = self.days.contains(&day);
            let dow_match = self
                .weekdays
                .contains(&cand.weekday().num_days_from_sunday());
            let day_ok = if self.days.len() == 31 && self.weekdays.len() != 7 {
                dow_match
            } else if self.weekdays.len() == 7 && self.days.len() != 31 {
                dom_match
            } else {
                dom_match || dow_match
            };
            if !day_ok {
                cand = next_day_start(cand);
                continue;
            }
            let hour = cand.hour();
            if !self.hours.contains(&hour) {
                cand = hour_start(cand) + Duration::hours(1);
                continue;
            }
            let minute = cand.minute();
            if !self.minutes.contains(&minute) {
                cand += Duration::minutes(1);
                continue;
            }
            return Some(cand);
        }
        None
    }

    /// 判断给定时刻是否命中（用于测试）
    pub fn matches(&self, dt: DateTime<Utc>) -> bool {
        self.months.contains(&dt.month())
            && self.days.contains(&dt.day())
            && self.weekdays.contains(&dt.weekday().num_days_from_sunday())
            && self.hours.contains(&dt.hour())
            && self.minutes.contains(&dt.minute())
    }
}

fn parse_field(spec: &str, min: u32, max: u32) -> Result<Vec<u32>, String> {
    let mut values = std::collections::BTreeSet::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err("empty field part".into());
        }
        let (range_part, step) = match part.split_once('/') {
            Some((r, s)) => (r, s.parse::<u32>().map_err(|_| "invalid step".to_string())?),
            None => (part, 1),
        };
        if step == 0 {
            return Err("step cannot be zero".into());
        }
        let (start, end) = if range_part == "*" {
            (min, max)
        } else if let Some((s, e)) = range_part.split_once('-') {
            let s = s
                .parse::<u32>()
                .map_err(|_| format!("invalid value: {s}"))?;
            let e = e
                .parse::<u32>()
                .map_err(|_| format!("invalid value: {e}"))?;
            (s, e)
        } else {
            let v = range_part
                .parse::<u32>()
                .map_err(|_| format!("invalid value: {range_part}"))?;
            (v, v)
        };
        if start < min || end > max || start > end {
            return Err(format!("range {start}-{end} out of bounds {min}-{max}"));
        }
        let mut v = start;
        while v <= end {
            values.insert(v);
            v += step;
        }
    }
    Ok(values.into_iter().collect())
}

fn floor_minute(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_second(0).unwrap().with_nanosecond(0).unwrap()
}

fn hour_start(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_minute(0).unwrap().with_second(0).unwrap()
}

fn next_month_start(dt: DateTime<Utc>) -> DateTime<Utc> {
    let first = dt.date_naive().with_day(1).unwrap();
    let next = first.checked_add_months(Months::new(1)).unwrap_or(first);
    next.and_hms_opt(0, 0, 0).unwrap().and_utc()
}

fn next_day_start(dt: DateTime<Utc>) -> DateTime<Utc> {
    let next = dt.date_naive() + Duration::days(1);
    next.and_hms_opt(0, 0, 0).unwrap().and_utc()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn test_parse_validation() {
        assert!(CronSchedule::parse("* * * * *").is_ok());
        assert!(CronSchedule::parse("*/5 */2 1-15 * 0,6").is_ok());
        assert!(CronSchedule::parse("1-10/2 * * * *").is_ok());
        assert!(CronSchedule::parse("60 * * * *").is_err());
        assert!(CronSchedule::parse("*/0 * * * *").is_err());
        assert!(CronSchedule::parse("* * * *").is_err());
        assert!(CronSchedule::parse("").is_err());
        assert!(!CronSchedule::is_valid("* * * *"));
        assert!(CronSchedule::is_valid("0 3 * * *"));
    }

    #[test]
    fn test_next_run_every_minute() {
        let s = CronSchedule::parse("* * * * *").unwrap();
        let now = utc(2026, 8, 3, 10, 30);
        assert_eq!(s.next_run(now).unwrap(), now + Duration::minutes(1));
    }

    #[test]
    fn test_next_run_daily_at_hour() {
        let s = CronSchedule::parse("0 3 * * *").unwrap();
        let now = utc(2026, 8, 3, 10, 30);
        assert_eq!(s.next_run(now).unwrap(), utc(2026, 8, 4, 3, 0));
        let before = utc(2026, 8, 3, 2, 59);
        assert_eq!(s.next_run(before).unwrap(), utc(2026, 8, 3, 3, 0));
    }

    #[test]
    fn test_next_run_step_and_list() {
        let s = CronSchedule::parse("*/15 9,18 * * *").unwrap();
        let now = utc(2026, 8, 3, 10, 30);
        assert_eq!(s.next_run(now).unwrap(), utc(2026, 8, 3, 18, 0));
        let before = utc(2026, 8, 3, 8, 59);
        assert_eq!(s.next_run(before).unwrap(), utc(2026, 8, 3, 9, 0));
    }

    #[test]
    fn test_next_run_weekday() {
        // 周一 08:00（2026-08-03 是周一）
        let s = CronSchedule::parse("0 8 * * 1").unwrap();
        let now = utc(2026, 8, 3, 9, 0);
        assert_eq!(s.next_run(now).unwrap(), utc(2026, 8, 10, 8, 0));
        let before = utc(2026, 8, 3, 7, 0);
        assert_eq!(s.next_run(before).unwrap(), utc(2026, 8, 3, 8, 0));
    }

    #[test]
    fn test_next_run_month_boundary() {
        let s = CronSchedule::parse("0 0 1 * *").unwrap();
        let now = utc(2026, 8, 15, 12, 0);
        assert_eq!(s.next_run(now).unwrap(), utc(2026, 9, 1, 0, 0));
    }

    #[test]
    fn test_matches() {
        let s = CronSchedule::parse("30 2 * * *").unwrap();
        assert!(s.matches(utc(2026, 8, 3, 2, 30)));
        assert!(!s.matches(utc(2026, 8, 3, 2, 31)));
        assert!(!s.matches(utc(2026, 8, 3, 3, 30)));
    }
}
