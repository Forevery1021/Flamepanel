use std::sync::{Mutex, MutexGuard, PoisonError};

/// 锁中毒时恢复内部数据继续使用（InMemory 仓储为纯数据结构，无跨字段不变量需要保护）。
pub fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_lock_or_recover_from_poisoned_mutex() {
        let data = Arc::new(Mutex::new(vec![1, 2, 3]));
        let data_clone = Arc::clone(&data);

        // 在子线程中获取锁并主动 panic，造成锁中毒
        let handle = std::thread::spawn(move || {
            let mut guard = data_clone.lock().unwrap();
            guard.push(4);
            panic!("intentionally poison the mutex");
        });

        assert!(handle.join().is_err());

        // 主线程使用 lock_or_recover 应能成功恢复并访问数据
        let guard = lock_or_recover(&data);
        assert_eq!(*guard, vec![1, 2, 3, 4]);
    }
}
