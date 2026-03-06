//! Sequence Number Gap Detection & Retransmission
//!
//! シーケンス番号のギャップ検出と再送要求管理。

/// ギャップ情報。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceGap {
    /// ギャップ開始シーケンス番号。
    pub begin: u64,
    /// ギャップ終了シーケンス番号。
    pub end: u64,
}

impl SequenceGap {
    /// ギャップ内のメッセージ数。
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.end - self.begin + 1
    }
}

/// シーケンストラッカー。
#[derive(Debug)]
pub struct SequenceTracker {
    /// 期待される次のシーケンス番号。
    expected_seq: u64,
    /// 検出されたギャップ。
    gaps: Vec<SequenceGap>,
    /// 再送要求済みギャップ。
    pending_resends: Vec<SequenceGap>,
    /// 処理済みメッセージ数。
    processed_count: u64,
}

impl SequenceTracker {
    /// 新しいトラッカーを作成。
    #[must_use]
    pub const fn new(initial_seq: u64) -> Self {
        Self {
            expected_seq: initial_seq,
            gaps: Vec::new(),
            pending_resends: Vec::new(),
            processed_count: 0,
        }
    }

    /// メッセージのシーケンス番号を処理。
    ///
    /// ギャップが検出された場合 `Some(SequenceGap)` を返す。
    pub fn process(&mut self, seq_num: u64) -> Option<SequenceGap> {
        self.processed_count += 1;

        match seq_num.cmp(&self.expected_seq) {
            core::cmp::Ordering::Equal => {
                self.expected_seq += 1;
                self.resolve_gap(seq_num);
                None
            }
            core::cmp::Ordering::Greater => {
                let gap = SequenceGap {
                    begin: self.expected_seq,
                    end: seq_num - 1,
                };
                self.gaps.push(gap.clone());
                self.expected_seq = seq_num + 1;
                Some(gap)
            }
            core::cmp::Ordering::Less => {
                self.resolve_gap(seq_num);
                None
            }
        }
    }

    /// ギャップを解消。
    fn resolve_gap(&mut self, seq_num: u64) {
        self.gaps
            .retain(|g| !(seq_num >= g.begin && seq_num <= g.end && g.count() == 1));
        self.pending_resends
            .retain(|g| !(seq_num >= g.begin && seq_num <= g.end && g.count() == 1));
    }

    /// 再送要求を登録。
    pub fn request_resend(&mut self, gap: SequenceGap) {
        self.pending_resends.push(gap);
    }

    /// 未解決ギャップ。
    #[must_use]
    pub fn gaps(&self) -> &[SequenceGap] {
        &self.gaps
    }

    /// 再送待ちギャップ。
    #[must_use]
    pub fn pending_resends(&self) -> &[SequenceGap] {
        &self.pending_resends
    }

    /// 期待される次のシーケンス番号。
    #[must_use]
    pub const fn expected_seq(&self) -> u64 {
        self.expected_seq
    }

    /// 処理済みメッセージ数。
    #[must_use]
    pub const fn processed_count(&self) -> u64 {
        self.processed_count
    }

    /// ギャップがあるか。
    #[must_use]
    pub const fn has_gaps(&self) -> bool {
        !self.gaps.is_empty()
    }

    /// シーケンス番号をリセット。
    pub fn reset(&mut self, new_seq: u64) {
        self.expected_seq = new_seq;
        self.gaps.clear();
        self.pending_resends.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_no_gap() {
        let mut tracker = SequenceTracker::new(1);
        assert!(tracker.process(1).is_none());
        assert!(tracker.process(2).is_none());
        assert!(tracker.process(3).is_none());
        assert_eq!(tracker.expected_seq(), 4);
    }

    #[test]
    fn detect_gap() {
        let mut tracker = SequenceTracker::new(1);
        tracker.process(1);
        let gap = tracker.process(5);
        assert!(gap.is_some());
        let gap = gap.unwrap();
        assert_eq!(gap.begin, 2);
        assert_eq!(gap.end, 4);
        assert_eq!(gap.count(), 3);
    }

    #[test]
    fn has_gaps() {
        let mut tracker = SequenceTracker::new(1);
        tracker.process(1);
        tracker.process(5);
        assert!(tracker.has_gaps());
    }

    #[test]
    fn no_gaps_initially() {
        let tracker = SequenceTracker::new(1);
        assert!(!tracker.has_gaps());
    }

    #[test]
    fn duplicate_message() {
        let mut tracker = SequenceTracker::new(1);
        tracker.process(1);
        tracker.process(2);
        let gap = tracker.process(1); // 重複
        assert!(gap.is_none());
    }

    #[test]
    fn request_resend() {
        let mut tracker = SequenceTracker::new(1);
        tracker.process(1);
        let gap = tracker.process(5).unwrap();
        tracker.request_resend(gap);
        assert_eq!(tracker.pending_resends().len(), 1);
    }

    #[test]
    fn processed_count() {
        let mut tracker = SequenceTracker::new(1);
        tracker.process(1);
        tracker.process(2);
        tracker.process(3);
        assert_eq!(tracker.processed_count(), 3);
    }

    #[test]
    fn reset() {
        let mut tracker = SequenceTracker::new(1);
        tracker.process(1);
        tracker.process(5);
        tracker.reset(1);
        assert_eq!(tracker.expected_seq(), 1);
        assert!(!tracker.has_gaps());
    }

    #[test]
    fn multiple_gaps() {
        let mut tracker = SequenceTracker::new(1);
        tracker.process(1);
        tracker.process(5); // gap 2-4
        tracker.process(10); // gap 6-9
        assert_eq!(tracker.gaps().len(), 2);
    }

    #[test]
    fn gap_count() {
        let gap = SequenceGap { begin: 3, end: 7 };
        assert_eq!(gap.count(), 5);
    }

    #[test]
    fn single_message_gap() {
        let mut tracker = SequenceTracker::new(1);
        tracker.process(1);
        let gap = tracker.process(3);
        assert!(gap.is_some());
        let gap = gap.unwrap();
        assert_eq!(gap.begin, 2);
        assert_eq!(gap.end, 2);
        assert_eq!(gap.count(), 1);
    }

    #[test]
    fn gap_eq() {
        let a = SequenceGap { begin: 1, end: 5 };
        let b = SequenceGap { begin: 1, end: 5 };
        assert_eq!(a, b);
    }

    #[test]
    fn start_from_non_one() {
        let mut tracker = SequenceTracker::new(100);
        assert!(tracker.process(100).is_none());
        assert_eq!(tracker.expected_seq(), 101);
    }
}
