use crate::error::VenueError;
use alpha_types::{OrderIntent, OrderType, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OkxAttachAlgo {
    #[serde(rename = "attachAlgoClOrdId")]
    pub attach_algo_cl_ord_id: String,
    #[serde(rename = "slTriggerPx", skip_serializing_if = "Option::is_none")]
    pub sl_trigger_px: Option<String>,
    #[serde(rename = "slOrdPx", skip_serializing_if = "Option::is_none")]
    pub sl_ord_px: Option<String>,
    #[serde(rename = "tpTriggerPx", skip_serializing_if = "Option::is_none")]
    pub tp_trigger_px: Option<String>,
    #[serde(rename = "tpOrdPx", skip_serializing_if = "Option::is_none")]
    pub tp_ord_px: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OkxOrderPayload {
    #[serde(rename = "instId")]
    pub inst_id: String,
    #[serde(rename = "tdMode")]
    pub td_mode: String,
    pub side: String,
    #[serde(rename = "ordType")]
    pub ord_type: String,
    pub sz: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub px: Option<String>,
    #[serde(rename = "clOrdId")]
    pub cl_ord_id: String,
    #[serde(rename = "attachAlgoOrds", skip_serializing_if = "Vec::is_empty")]
    pub attach_algo_ords: Vec<OkxAttachAlgo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OkxDeadManPayload {
    #[serde(rename = "timeOut")]
    pub time_out: String,
}

impl OkxDeadManPayload {
    pub fn new(timeout_secs: u32) -> Result<Self, VenueError> {
        if !(10..=120).contains(&timeout_secs) {
            return Err(VenueError::InvalidOrder {
                reason: "OKX cancel-all-after timeout must be between 10 and 120 seconds".into(),
            });
        }
        Ok(Self {
            time_out: timeout_secs.to_string(),
        })
    }
}

pub fn build_okx_atomic_order(intent: &OrderIntent) -> Result<OkxOrderPayload, VenueError> {
    let side_str = match intent.side {
        Side::Buy => "buy",
        Side::Sell => "sell",
    };

    let ord_type_str = match intent.order_type {
        OrderType::LimitMaker => "post_only",
        OrderType::Limit => "limit",
        OrderType::Market => "market",
        OrderType::StopLossLimit => "conditional",
    };

    let px_str = intent.price.map(|p| p.to_string());
    if intent.order_type != OrderType::Market && px_str.is_none() {
        return Err(VenueError::InvalidOrder {
            reason: "Non-market OKX order requires price".into(),
        });
    }

    let mut attach_algo_ords = Vec::new();
    if let Some(sl) = intent.stop_loss_price {
        let algo_cl_ord_id = format!("sl-{}", intent.client_order_id);
        let tp_trigger_px = intent.take_profit_price.map(|tp| tp.to_string());
        let tp_ord_px = if tp_trigger_px.is_some() {
            Some("-1".to_string())
        } else {
            None
        };

        attach_algo_ords.push(OkxAttachAlgo {
            attach_algo_cl_ord_id: algo_cl_ord_id,
            sl_trigger_px: Some(sl.to_string()),
            sl_ord_px: Some("-1".to_string()), // "-1" represents market execution on trigger
            tp_trigger_px,
            tp_ord_px,
        });
    }

    Ok(OkxOrderPayload {
        inst_id: intent.symbol.clone(),
        td_mode: "cash".into(),
        side: side_str.into(),
        ord_type: ord_type_str.into(),
        sz: intent.qty.to_string(),
        px: px_str,
        cl_ord_id: intent.client_order_id.clone(),
        attach_algo_ords,
    })
}

#[derive(Debug, Default, Clone)]
pub struct OkxBookSequencer {
    last_seq_id: Option<i64>,
}

impl OkxBookSequencer {
    pub fn new() -> Self {
        Self { last_seq_id: None }
    }

    pub fn process_update(&mut self, prev_seq_id: i64, seq_id: i64) -> Result<(), VenueError> {
        // -1 indicates a fresh snapshot or post-maintenance reset
        if prev_seq_id == -1 {
            self.last_seq_id = Some(seq_id);
            return Ok(());
        }

        match self.last_seq_id {
            Some(last) if prev_seq_id == last => {
                self.last_seq_id = Some(seq_id);
                Ok(())
            }
            Some(last) => Err(VenueError::SequenceGap {
                expected: last,
                got: prev_seq_id,
            }),
            None => Err(VenueError::SequenceGap {
                expected: -1,
                got: prev_seq_id,
            }),
        }
    }

    pub fn reset(&mut self) {
        self.last_seq_id = None;
    }

    pub fn last_seq_id(&self) -> Option<i64> {
        self.last_seq_id
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use alpha_types::Venue;
    use rust_decimal_macros::dec;

    #[test]
    fn test_okx_atomic_order_construction() {
        let intent = OrderIntent::new(
            Venue::Okx,
            "BTC-USDT",
            Side::Buy,
            OrderType::LimitMaker,
            Some(dec!(65000.0)),
            dec!(0.05),
            Some(dec!(63500.0)),
            Some(dec!(70000.0)),
            "cl-ord-001",
        )
        .unwrap();

        let payload = build_okx_atomic_order(&intent).unwrap();
        assert_eq!(payload.inst_id, "BTC-USDT");
        assert_eq!(payload.td_mode, "cash");
        assert_eq!(payload.ord_type, "post_only");
        assert_eq!(payload.px.as_deref(), Some("65000.0"));
        assert_eq!(payload.sz, "0.05");
        assert_eq!(payload.attach_algo_ords.len(), 1);

        let algo = &payload.attach_algo_ords[0];
        assert_eq!(algo.sl_trigger_px.as_deref(), Some("63500.0"));
        assert_eq!(algo.sl_ord_px.as_deref(), Some("-1"));
        assert_eq!(algo.tp_trigger_px.as_deref(), Some("70000.0"));

        let serialized = serde_json::to_string(&payload).unwrap();
        assert!(serialized.contains("attachAlgoOrds"));
        assert!(serialized.contains("slTriggerPx"));
    }

    #[test]
    fn test_okx_dead_man_payload() {
        assert!(OkxDeadManPayload::new(9).is_err());
        assert!(OkxDeadManPayload::new(121).is_err());
        let valid = OkxDeadManPayload::new(60).unwrap();
        let json = serde_json::to_string(&valid).unwrap();
        assert_eq!(json, r#"{"timeOut":"60"}"#);
    }

    #[test]
    fn test_okx_book_sequencing() {
        let mut seq = OkxBookSequencer::new();
        // First message must be snapshot
        assert!(seq.process_update(100, 101).is_err());

        // Snapshot arrives
        assert!(seq.process_update(-1, 1000).is_ok());
        assert_eq!(seq.last_seq_id(), Some(1000));

        // Incremental update arrives in order
        assert!(seq.process_update(1000, 1001).is_ok());
        assert_eq!(seq.last_seq_id(), Some(1001));

        // Sequence gap triggers error
        let err = seq.process_update(1005, 1006);
        assert!(matches!(
            err,
            Err(VenueError::SequenceGap {
                expected: 1001,
                got: 1005
            })
        ));
    }
}
