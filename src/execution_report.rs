//! Execution Report パーシング
//!
//! FIX `ExecutionReport` (MsgType=8) の構造化パース。

use crate::message::FixMessage;
use crate::tag;

/// 約定種別 (`ExecType`, tag 150)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecType {
    /// 新規。
    New,
    /// 部分約定。
    PartialFill,
    /// 全量約定。
    Fill,
    /// キャンセル。
    Canceled,
    /// 修正。
    Replaced,
    /// 拒否。
    Rejected,
    /// その他。
    Other(u8),
}

impl ExecType {
    /// FIX 文字列から変換。
    #[must_use]
    pub fn from_fix(s: &str) -> Self {
        match s {
            "0" => Self::New,
            "1" => Self::PartialFill,
            "2" => Self::Fill,
            "4" => Self::Canceled,
            "5" => Self::Replaced,
            "8" => Self::Rejected,
            _ => Self::Other(s.as_bytes().first().copied().unwrap_or(0)),
        }
    }
}

/// 注文ステータス (`OrdStatus`, tag 39)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdStatus {
    /// 新規。
    New,
    /// 部分約定。
    PartiallyFilled,
    /// 全量約定。
    Filled,
    /// キャンセル済み。
    Canceled,
    /// 拒否。
    Rejected,
    /// その他。
    Other(u8),
}

impl OrdStatus {
    /// FIX 文字列から変換。
    #[must_use]
    pub fn from_fix(s: &str) -> Self {
        match s {
            "0" => Self::New,
            "1" => Self::PartiallyFilled,
            "2" => Self::Filled,
            "4" => Self::Canceled,
            "8" => Self::Rejected,
            _ => Self::Other(s.as_bytes().first().copied().unwrap_or(0)),
        }
    }
}

/// 構造化 Execution Report。
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    /// 注文 ID (tag 37)。
    pub order_id: String,
    /// クライアント注文 ID (tag 11)。
    pub cl_ord_id: String,
    /// 約定 ID (tag 17)。
    pub exec_id: String,
    /// 約定種別 (tag 150)。
    pub exec_type: ExecType,
    /// 注文ステータス (tag 39)。
    pub ord_status: OrdStatus,
    /// シンボル (tag 55)。
    pub symbol: String,
    /// サイド (tag 54)。
    pub side: String,
    /// 直近約定価格 (tag 31)。
    pub last_px: Option<f64>,
    /// 直近約定数量 (tag 32)。
    pub last_qty: Option<f64>,
    /// 残数量 (tag 151)。
    pub leaves_qty: Option<f64>,
    /// 累計約定数量 (tag 14)。
    pub cum_qty: Option<f64>,
    /// 平均約定価格 (tag 6)。
    pub avg_px: Option<f64>,
    /// テキスト (tag 58)。
    pub text: Option<String>,
}

impl ExecutionReport {
    /// `FixMessage` から `ExecutionReport` をパース。
    ///
    /// # Errors
    ///
    /// メッセージタイプが "8" でない場合、必須フィールドが欠落している場合。
    pub fn from_message(msg: &FixMessage) -> Result<Self, ExecReportError> {
        if msg.msg_type != "8" {
            return Err(ExecReportError::WrongMsgType(msg.msg_type.clone()));
        }

        let order_id = msg
            .get(tag::ORDER_ID)
            .ok_or(ExecReportError::MissingField(tag::ORDER_ID))?
            .to_string();
        let cl_ord_id = msg
            .get(tag::CL_ORD_ID)
            .ok_or(ExecReportError::MissingField(tag::CL_ORD_ID))?
            .to_string();
        let exec_id = msg
            .get(tag::EXEC_ID)
            .ok_or(ExecReportError::MissingField(tag::EXEC_ID))?
            .to_string();
        let exec_type_str = msg
            .get(tag::EXEC_TYPE)
            .ok_or(ExecReportError::MissingField(tag::EXEC_TYPE))?;
        let ord_status_str = msg
            .get(tag::ORD_STATUS)
            .ok_or(ExecReportError::MissingField(tag::ORD_STATUS))?;
        let symbol = msg
            .get(tag::SYMBOL)
            .ok_or(ExecReportError::MissingField(tag::SYMBOL))?
            .to_string();
        let side = msg
            .get(tag::SIDE)
            .ok_or(ExecReportError::MissingField(tag::SIDE))?
            .to_string();

        let parse_f64 = |t: u32| -> Option<f64> { msg.get(t).and_then(|v| v.parse().ok()) };

        Ok(Self {
            order_id,
            cl_ord_id,
            exec_id,
            exec_type: ExecType::from_fix(exec_type_str),
            ord_status: OrdStatus::from_fix(ord_status_str),
            symbol,
            side,
            last_px: parse_f64(tag::LAST_PX),
            last_qty: parse_f64(tag::LAST_QTY),
            leaves_qty: parse_f64(tag::LEAVES_QTY),
            cum_qty: parse_f64(tag::CUM_QTY),
            avg_px: parse_f64(tag::AVG_PX),
            text: msg.get(tag::TEXT).map(String::from),
        })
    }
}

/// Execution Report エラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecReportError {
    /// メッセージタイプが不正。
    WrongMsgType(String),
    /// 必須フィールドが欠落。
    MissingField(u32),
}

impl core::fmt::Display for ExecReportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::WrongMsgType(t) => write!(f, "Wrong MsgType: expected 8, got {t}"),
            Self::MissingField(tag) => write!(f, "Missing required field: tag {tag}"),
        }
    }
}

impl std::error::Error for ExecReportError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::FixMessage;
    use crate::tag;

    fn make_exec_report() -> FixMessage {
        let mut msg = FixMessage::new("FIX.4.4", "8");
        msg.set(tag::ORDER_ID, "ORD123");
        msg.set(tag::CL_ORD_ID, "CLI456");
        msg.set(tag::EXEC_ID, "EXEC789");
        msg.set(tag::EXEC_TYPE, "2");
        msg.set(tag::ORD_STATUS, "2");
        msg.set(tag::SYMBOL, "BTCUSD");
        msg.set(tag::SIDE, "1");
        msg.set(tag::LAST_PX, "50000.50");
        msg.set(tag::LAST_QTY, "1.5");
        msg.set(tag::LEAVES_QTY, "0");
        msg.set(tag::CUM_QTY, "1.5");
        msg.set(tag::AVG_PX, "50000.50");
        msg
    }

    #[test]
    fn parse_exec_report() {
        let msg = make_exec_report();
        let report = ExecutionReport::from_message(&msg).unwrap();
        assert_eq!(report.order_id, "ORD123");
        assert_eq!(report.exec_type, ExecType::Fill);
        assert_eq!(report.ord_status, OrdStatus::Filled);
        assert_eq!(report.symbol, "BTCUSD");
    }

    #[test]
    fn parse_fill_prices() {
        let msg = make_exec_report();
        let report = ExecutionReport::from_message(&msg).unwrap();
        assert!((report.last_px.unwrap() - 50000.50).abs() < 0.01);
        assert!((report.last_qty.unwrap() - 1.5).abs() < 0.01);
    }

    #[test]
    fn wrong_msg_type() {
        let msg = FixMessage::new("FIX.4.4", "D");
        assert!(ExecutionReport::from_message(&msg).is_err());
    }

    #[test]
    fn missing_order_id() {
        let mut msg = FixMessage::new("FIX.4.4", "8");
        msg.set(tag::CL_ORD_ID, "C1");
        msg.set(tag::EXEC_ID, "E1");
        msg.set(tag::EXEC_TYPE, "0");
        msg.set(tag::ORD_STATUS, "0");
        msg.set(tag::SYMBOL, "BTC");
        msg.set(tag::SIDE, "1");
        assert!(ExecutionReport::from_message(&msg).is_err());
    }

    #[test]
    fn exec_type_from_fix() {
        assert_eq!(ExecType::from_fix("0"), ExecType::New);
        assert_eq!(ExecType::from_fix("1"), ExecType::PartialFill);
        assert_eq!(ExecType::from_fix("2"), ExecType::Fill);
        assert_eq!(ExecType::from_fix("4"), ExecType::Canceled);
        assert_eq!(ExecType::from_fix("8"), ExecType::Rejected);
    }

    #[test]
    fn ord_status_from_fix() {
        assert_eq!(OrdStatus::from_fix("0"), OrdStatus::New);
        assert_eq!(OrdStatus::from_fix("1"), OrdStatus::PartiallyFilled);
        assert_eq!(OrdStatus::from_fix("2"), OrdStatus::Filled);
        assert_eq!(OrdStatus::from_fix("4"), OrdStatus::Canceled);
    }

    #[test]
    fn exec_type_other() {
        let et = ExecType::from_fix("Z");
        assert!(matches!(et, ExecType::Other(_)));
    }

    #[test]
    fn optional_text() {
        let mut msg = make_exec_report();
        msg.set(tag::TEXT, "Order filled");
        let report = ExecutionReport::from_message(&msg).unwrap();
        assert_eq!(report.text.as_deref(), Some("Order filled"));
    }

    #[test]
    fn no_optional_fields() {
        let mut msg = FixMessage::new("FIX.4.4", "8");
        msg.set(tag::ORDER_ID, "O1");
        msg.set(tag::CL_ORD_ID, "C1");
        msg.set(tag::EXEC_ID, "E1");
        msg.set(tag::EXEC_TYPE, "0");
        msg.set(tag::ORD_STATUS, "0");
        msg.set(tag::SYMBOL, "ETH");
        msg.set(tag::SIDE, "2");
        let report = ExecutionReport::from_message(&msg).unwrap();
        assert!(report.last_px.is_none());
        assert!(report.text.is_none());
    }

    #[test]
    fn error_display() {
        assert_eq!(
            ExecReportError::WrongMsgType("D".into()).to_string(),
            "Wrong MsgType: expected 8, got D"
        );
        assert_eq!(
            ExecReportError::MissingField(37).to_string(),
            "Missing required field: tag 37"
        );
    }

    #[test]
    fn exec_report_side() {
        let msg = make_exec_report();
        let report = ExecutionReport::from_message(&msg).unwrap();
        assert_eq!(report.side, "1");
    }

    #[test]
    fn partial_fill() {
        let mut msg = make_exec_report();
        msg.set(tag::EXEC_TYPE, "1");
        msg.set(tag::ORD_STATUS, "1");
        msg.set(tag::LEAVES_QTY, "0.5");
        let report = ExecutionReport::from_message(&msg).unwrap();
        assert_eq!(report.exec_type, ExecType::PartialFill);
        assert!((report.leaves_qty.unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn canceled_report() {
        let mut msg = make_exec_report();
        msg.set(tag::EXEC_TYPE, "4");
        msg.set(tag::ORD_STATUS, "4");
        let report = ExecutionReport::from_message(&msg).unwrap();
        assert_eq!(report.exec_type, ExecType::Canceled);
        assert_eq!(report.ord_status, OrdStatus::Canceled);
    }
}
