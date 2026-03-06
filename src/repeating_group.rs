//! FIX 5.0 Repeating Groups
//!
//! ネストされた tag-value リストのパースと構築。

use std::collections::HashMap;

/// Repeating Group エントリ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupEntry {
    /// タグ-値ペア。
    pub fields: HashMap<u32, String>,
}

impl GroupEntry {
    /// 新しいエントリを作成。
    #[must_use]
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// フィールドを設定。
    pub fn set(&mut self, tag: u32, value: &str) -> &mut Self {
        self.fields.insert(tag, value.to_string());
        self
    }

    /// フィールドを取得。
    #[must_use]
    pub fn get(&self, tag: u32) -> Option<&str> {
        self.fields.get(&tag).map(String::as_str)
    }
}

impl Default for GroupEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// Repeating Group。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatingGroup {
    /// カウントタグ (`NoXxx`)。
    pub count_tag: u32,
    /// 先頭タグ (各エントリの開始を示す)。
    pub delimiter_tag: u32,
    /// エントリリスト。
    pub entries: Vec<GroupEntry>,
}

impl RepeatingGroup {
    /// 新しい Repeating Group を作成。
    #[must_use]
    pub const fn new(count_tag: u32, delimiter_tag: u32) -> Self {
        Self {
            count_tag,
            delimiter_tag,
            entries: Vec::new(),
        }
    }

    /// エントリを追加。
    pub fn add_entry(&mut self, entry: GroupEntry) {
        self.entries.push(entry);
    }

    /// エントリ数。
    #[must_use]
    pub const fn count(&self) -> usize {
        self.entries.len()
    }

    /// インデックスでエントリを取得。
    #[must_use]
    pub fn get_entry(&self, index: usize) -> Option<&GroupEntry> {
        self.entries.get(index)
    }

    /// Repeating Group を FIX tag-value ペア列にシリアライズ。
    #[must_use]
    pub fn serialize(&self) -> Vec<(u32, String)> {
        let mut result = Vec::new();
        result.push((self.count_tag, self.entries.len().to_string()));

        for entry in &self.entries {
            // デリミタタグを最初に出力
            if let Some(val) = entry.get(self.delimiter_tag) {
                result.push((self.delimiter_tag, val.to_string()));
            }
            // 残りのフィールドを出力
            for (&tag, val) in &entry.fields {
                if tag != self.delimiter_tag {
                    result.push((tag, val.clone()));
                }
            }
        }

        result
    }
}

/// tag-value ペア列から Repeating Group をパース。
///
/// # Errors
///
/// カウントタグの値が不正な場合。
pub fn parse_group(
    tags: &[(u32, String)],
    count_tag: u32,
    delimiter_tag: u32,
) -> Result<RepeatingGroup, GroupParseError> {
    let mut group = RepeatingGroup::new(count_tag, delimiter_tag);

    // カウントタグを検索
    let count_str = tags
        .iter()
        .find(|(t, _)| *t == count_tag)
        .map(|(_, v)| v.as_str())
        .ok_or(GroupParseError::MissingCountTag)?;

    let expected_count: usize = count_str
        .parse()
        .map_err(|_| GroupParseError::InvalidCount)?;

    // デリミタタグでエントリを分割
    let mut current_entry: Option<GroupEntry> = None;
    let mut in_group = false;

    for &(tag, ref value) in tags {
        if tag == count_tag {
            in_group = true;
            continue;
        }

        if !in_group {
            continue;
        }

        if tag == delimiter_tag {
            // 前のエントリを保存
            if let Some(entry) = current_entry.take() {
                group.add_entry(entry);
            }
            let mut new_entry = GroupEntry::new();
            new_entry.set(tag, value);
            current_entry = Some(new_entry);
        } else if let Some(ref mut entry) = current_entry {
            entry.set(tag, value);
        }
    }

    // 最後のエントリを保存
    if let Some(entry) = current_entry {
        group.add_entry(entry);
    }

    if group.count() != expected_count {
        return Err(GroupParseError::CountMismatch {
            expected: expected_count,
            actual: group.count(),
        });
    }

    Ok(group)
}

/// グループパースエラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupParseError {
    /// カウントタグが見つからない。
    MissingCountTag,
    /// カウント値が不正。
    InvalidCount,
    /// エントリ数不一致。
    CountMismatch {
        /// 期待数。
        expected: usize,
        /// 実際数。
        actual: usize,
    },
}

impl core::fmt::Display for GroupParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingCountTag => write!(f, "Missing count tag"),
            Self::InvalidCount => write!(f, "Invalid count value"),
            Self::CountMismatch { expected, actual } => {
                write!(f, "Count mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

impl std::error::Error for GroupParseError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_entry_set_get() {
        let mut entry = GroupEntry::new();
        entry.set(11, "ORD1");
        assert_eq!(entry.get(11), Some("ORD1"));
    }

    #[test]
    fn group_entry_missing() {
        let entry = GroupEntry::new();
        assert!(entry.get(999).is_none());
    }

    #[test]
    fn repeating_group_add() {
        let mut group = RepeatingGroup::new(453, 448);
        let mut e = GroupEntry::new();
        e.set(448, "PARTY1");
        group.add_entry(e);
        assert_eq!(group.count(), 1);
    }

    #[test]
    fn repeating_group_get_entry() {
        let mut group = RepeatingGroup::new(453, 448);
        let mut e = GroupEntry::new();
        e.set(448, "P1");
        e.set(447, "D");
        group.add_entry(e);
        let entry = group.get_entry(0).unwrap();
        assert_eq!(entry.get(448), Some("P1"));
        assert_eq!(entry.get(447), Some("D"));
    }

    #[test]
    fn repeating_group_out_of_bounds() {
        let group = RepeatingGroup::new(453, 448);
        assert!(group.get_entry(0).is_none());
    }

    #[test]
    fn serialize_group() {
        let mut group = RepeatingGroup::new(453, 448);
        let mut e = GroupEntry::new();
        e.set(448, "PARTY1");
        group.add_entry(e);
        let pairs = group.serialize();
        assert_eq!(pairs[0], (453, "1".to_string()));
        assert!(pairs.iter().any(|(t, v)| *t == 448 && v == "PARTY1"));
    }

    #[test]
    fn parse_group_basic() {
        let tags = vec![
            (453, "2".to_string()),
            (448, "PARTY1".to_string()),
            (447, "D".to_string()),
            (448, "PARTY2".to_string()),
            (447, "C".to_string()),
        ];
        let group = parse_group(&tags, 453, 448).unwrap();
        assert_eq!(group.count(), 2);
        assert_eq!(group.get_entry(0).unwrap().get(448), Some("PARTY1"));
        assert_eq!(group.get_entry(1).unwrap().get(448), Some("PARTY2"));
    }

    #[test]
    fn parse_group_missing_count() {
        let tags = vec![(448, "P1".to_string())];
        assert!(parse_group(&tags, 453, 448).is_err());
    }

    #[test]
    fn parse_group_invalid_count() {
        let tags = vec![(453, "abc".to_string())];
        assert!(parse_group(&tags, 453, 448).is_err());
    }

    #[test]
    fn parse_group_count_mismatch() {
        let tags = vec![
            (453, "3".to_string()),
            (448, "P1".to_string()),
            (448, "P2".to_string()),
        ];
        assert!(parse_group(&tags, 453, 448).is_err());
    }

    #[test]
    fn group_parse_error_display() {
        assert_eq!(
            GroupParseError::MissingCountTag.to_string(),
            "Missing count tag"
        );
        assert_eq!(
            GroupParseError::InvalidCount.to_string(),
            "Invalid count value"
        );
    }

    #[test]
    fn group_entry_default() {
        let entry = GroupEntry::default();
        assert!(entry.fields.is_empty());
    }

    #[test]
    fn empty_group_serialize() {
        let group = RepeatingGroup::new(453, 448);
        let pairs = group.serialize();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], (453, "0".to_string()));
    }
}
