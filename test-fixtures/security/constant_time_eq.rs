// D02: shared synthetic UTF-8 equality vectors, included only by #[cfg(test)] modules.
// This is a functional contract, not a timing test or proof of side-channel resistance.
// Kernel-only tests additionally exercise arbitrary bytes that Agent &str cannot represent.

/// (case name, left input, right input, exact byte-equality result).
pub type EqualityCase = (&'static str, &'static str, &'static str, bool);

pub const UTF8_CASES: &[EqualityCase] = &[
    ("empty", "", "", true),
    ("empty_left", "", "a", false),
    ("empty_right", "a", "", false),
    ("one_equal", "a", "a", true),
    ("one_different", "a", "b", false),
    ("ascii_equal", "synthetic-key", "synthetic-key", true),
    ("first_byte", "abcde", "xbcde", false),
    ("middle_byte", "abcde", "abxde", false),
    ("last_byte", "abcde", "abcdx", false),
    ("prefix_left", "prefix", "prefix-tail", false),
    ("prefix_right", "prefix-tail", "prefix", false),
    ("case_sensitive", "Case", "case", false),
    ("xor_cancellation", "aaaa", "baab", false),
    ("all_bytes_different", "aaaa", "bbbb", false),
    ("nul_equal", "\0", "\0", true),
    ("nul_not_empty", "\0", "", false),
    ("embedded_nul_equal", "ab\0cd", "ab\0cd", true),
    ("embedded_nul_difference", "ab\0cd", "ab\0ce", false),
    ("trailing_nul", "key\0", "key", false),
    ("leading_space", " key", "key", false),
    ("trailing_space", "key ", "key", false),
    ("whitespace_equal", "\tkey\r\n", "\tkey\r\n", true),
    ("line_endings", "key\r\n", "key\n", false),
    ("unicode_equal", "密钥🔑", "密钥🔑", true),
    ("unicode_last_byte", "🔑", "🔒", false),
    ("unicode_shared_prefix", "é", "ê", false),
    ("unicode_same_byte_length", "é", "ab", false),
    ("unicode_same_scalar_count", "é", "a", false),
    ("unicode_not_normalized", "é", "e\u{0301}", false),
    ("unicode_lookalike", "A", "А", false),
    ("unicode_nul_suffix", "密\0甲", "密\0乙", false),
    (
        "long_equal",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        true,
    ),
    (
        "long_final_byte",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdee",
        false,
    ),
];

#[cfg(test)]
mod tests {
    use super::UTF8_CASES;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique_and_labels_mean_exact_byte_equality() {
        let mut names = HashSet::new();
        for &(name, left, right, equal) in UTF8_CASES {
            assert!(!name.is_empty());
            assert!(names.insert(name), "duplicate case name: {name}");
            assert_eq!(left.as_bytes() == right.as_bytes(), equal, "case: {name}");
        }
    }

    #[test]
    fn corpus_keeps_required_boundary_classes() {
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, b, equal)| equal && a.is_empty() && b.is_empty()));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, b, _)| a.is_empty() && !b.is_empty()));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, b, _)| !a.is_empty() && b.is_empty()));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, b, equal)| !equal && a.len() == b.len()));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, b, equal)| !equal && a.len() != b.len()));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, _, equal)| equal && !a.is_ascii()));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, b, equal)| !equal && !a.is_ascii() && !b.is_ascii()));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, _, equal)| equal && a.contains('\0')));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, b, equal)| !equal && a.contains('\0') && b.contains('\0')));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, b, equal)| !equal && a.trim() == b.trim()));
        assert!(UTF8_CASES
            .iter()
            .any(|&(_, a, _, equal)| equal && a.len() >= 64));
    }

    #[test]
    fn corpus_detects_common_functional_mutants() {
        type Predicate = fn(&str, &str) -> bool;
        let mutants: &[(&str, Predicate)] = &[
            ("prefix-only", |a, b| a.starts_with(b)),
            ("zip-without-length", |a, b| {
                a.bytes().zip(b.bytes()).all(|(x, y)| x == y)
            }),
            ("xor-cancellation", |a, b| {
                a.len() == b.len()
                    && a.bytes()
                        .zip(b.bytes())
                        .fold(0, |diff, (x, y)| diff ^ x ^ y)
                        == 0
            }),
            ("skip-final-byte", |a, b| {
                a.len() == b.len()
                    && a.bytes()
                        .zip(b.bytes())
                        .take(a.len().saturating_sub(1))
                        .all(|(x, y)| x == y)
            }),
            ("nul-terminated", |a, b| {
                a.split('\0').next() == b.split('\0').next()
            }),
            ("trim-before-comparison", |a, b| a.trim() == b.trim()),
            ("ascii-case-fold", |a, b| a.eq_ignore_ascii_case(b)),
        ];
        for &(name, compare) in mutants {
            assert!(
                UTF8_CASES
                    .iter()
                    .any(|&(_, left, right, equal)| compare(left, right) != equal),
                "corpus failed to detect functional mutant: {name}",
            );
        }
    }
}
