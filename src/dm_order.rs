use serde::Deserialize;
use thiserror::Error;

const SPOT_HOST: &str = "https://spot.photoprintit.com";

#[derive(Debug, Error)]
pub enum OrderError {
    #[error("order number must look like 544850-103554 (6 digits, dash, 6 digits)")]
    InvalidFormat,
    #[error("network error talking to dm order API: {0}")]
    Network(#[from] reqwest::Error),
    #[error("dm order API returned HTTP {status}")]
    BadStatus { status: u16 },
}

/// States at which the order is "ready" for pickup (UI badge), i.e. in the
/// Filiale / shipped — not the same as ticket completion.
pub const READY_STATES: [&str; 3] = ["SHIPPED", "DELIVERED", "PICKED_UP"];

/// States at which a tracking ticket is considered complete.
/// Pickup orders finish at DELIVERED ("zur Abholung bereit"); PICKED_UP still counts.
pub const DONE_STATES: [&str; 2] = ["DELIVERED", "PICKED_UP"];

/// Subset of the spot.photoprintit.com `orderInfo` response that we care about.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInfo {
    pub summary_state_code: String,
    #[serde(default)]
    pub summary_state_text: Option<String>,
    #[serde(default)]
    pub summary_date: Option<String>,
    #[serde(default)]
    pub customer_no: Option<String>,
    #[serde(default)]
    pub shop_no: Option<String>,
    #[serde(default)]
    pub order_no: Option<String>,
    #[serde(default)]
    pub order_date: Option<String>,
    /// 0 = pickup in a dm-Markt (Filiale), 1 = home delivery, -1 = unknown.
    #[serde(default)]
    pub delivery_type: i64,
}

impl OrderInfo {
    /// `true` when dm could not resolve the order. This is what the public site
    /// renders as "Da hat etwas nicht geklappt. Bitte prüfe Deine Eingaben...",
    /// i.e. the order is not initialized yet.
    pub fn is_error(&self) -> bool {
        self.summary_state_code.eq_ignore_ascii_case("ERROR")
    }

    /// `true` once the order reached the ready stage ("in die Filiale
    /// geliefert" / SHIPPED) or beyond — for display only.
    pub fn is_ready(&self) -> bool {
        is_ready_code(&self.summary_state_code)
    }

    /// `true` once the order is ready for pickup (DELIVERED) or collected
    /// (PICKED_UP). Tickets are marked completed when this is true.
    pub fn is_done(&self) -> bool {
        is_done_code(&self.summary_state_code)
    }

    /// The ordered progress steps for this order, mirroring what the dm site
    /// shows. Empty when the current state is not part of the normal flow
    /// (e.g. ERROR or RETURN).
    pub fn timeline(&self) -> Vec<TimelineStep> {
        build_timeline(&self.summary_state_code, self.delivery_type)
    }
}

/// `true` when `code` is a ready-or-later state (for UI).
pub fn is_ready_code(code: &str) -> bool {
    READY_STATES.iter().any(|s| code.eq_ignore_ascii_case(s))
}

/// `true` when the order is ready for pickup or already collected.
pub fn is_done_code(code: &str) -> bool {
    DONE_STATES.iter().any(|s| code.eq_ignore_ascii_case(s))
}

/// Where a step sits relative to the current order state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Past,
    Current,
    Future,
}

impl StepStatus {
    pub fn css(&self) -> &'static str {
        match self {
            StepStatus::Past => "past",
            StepStatus::Current => "current",
            StepStatus::Future => "future",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimelineStep {
    pub code: &'static str,
    pub label: &'static str,
    pub status: StepStatus,
}

fn step_sequence(delivery_type: i64) -> &'static [&'static str] {
    if delivery_type == 1 {
        // Home delivery includes a transport leg.
        &[
            "SUBMITTED",
            "PROCESSING",
            "SHIPPED",
            "TRANSPORT",
            "DELIVERED",
            "PICKED_UP",
        ]
    } else {
        // Pickup in a dm-Markt (Filiale).
        &[
            "SUBMITTED",
            "PROCESSING",
            "SHIPPED",
            "DELIVERED",
            "PICKED_UP",
        ]
    }
}

/// German label for a step, matching dm's text resources. `current` selects the
/// "active" wording where it differs from the "inactive" wording.
fn step_label(code: &str, home: bool, current: bool) -> &'static str {
    match code {
        "SUBMITTED" => "Deinen Auftrag haben wir erhalten.",
        "PROCESSING" => "Dein Auftrag wird gefertigt.",
        "SHIPPED" => {
            if home {
                "Deinen Auftrag haben wir an Deine Lieferadresse versendet."
            } else if current {
                "Dein Auftrag wird in den dm-Markt geliefert."
            } else {
                "Dein Auftrag wird in die Filiale geliefert."
            }
        }
        "TRANSPORT" => "Dein Auftrag ist unterwegs",
        "DELIVERED" => {
            if home {
                "Dein Auftrag wurde zugestellt."
            } else {
                "Dein Auftrag liegt zur Abholung bereit."
            }
        }
        "PICKED_UP" => "Dein Auftrag wurde abgeholt.",
        _ => "Auftragsstatus",
    }
}

/// Build the ordered step list for a given current state and delivery type.
pub fn build_timeline(summary_state_code: &str, delivery_type: i64) -> Vec<TimelineStep> {
    let seq = step_sequence(delivery_type);
    let home = delivery_type == 1;
    let current_idx = seq
        .iter()
        .position(|c| c.eq_ignore_ascii_case(summary_state_code));
    let Some(current_idx) = current_idx else {
        return Vec::new();
    };

    seq.iter()
        .enumerate()
        .map(|(i, &code)| {
            let status = match i.cmp(&current_idx) {
                std::cmp::Ordering::Less => StepStatus::Past,
                std::cmp::Ordering::Equal => StepStatus::Current,
                std::cmp::Ordering::Greater => StepStatus::Future,
            };
            TimelineStep {
                code,
                label: step_label(code, home, status == StepStatus::Current),
                status,
            }
        })
        .collect()
}

/// Validate that `order_number` matches the 12-digit `NNNNNN-NNNNNN` format.
pub fn is_valid_order_number(order_number: &str) -> bool {
    let bytes = order_number.as_bytes();
    if bytes.len() != 13 {
        return false;
    }
    bytes.iter().enumerate().all(|(i, b)| {
        if i == 6 {
            *b == b'-'
        } else {
            b.is_ascii_digit()
        }
    })
}

/// Query the dm Foto order-status API for the given 12-digit order number.
pub async fn query_order(
    http: &reqwest::Client,
    key_account_id: &str,
    order_number: &str,
) -> Result<OrderInfo, OrderError> {
    if !is_valid_order_number(order_number) {
        return Err(OrderError::InvalidFormat);
    }

    let url = format!("{SPOT_HOST}/spotapi/orderInfo/order");
    let resp = http
        .get(url)
        .query(&[("config", key_account_id), ("fullOrderId", order_number)])
        .header("Accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(OrderError::BadStatus {
            status: resp.status().as_u16(),
        });
    }

    let info = resp.json::<OrderInfo>().await?;
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_number() {
        assert!(is_valid_order_number("544850-103554"));
    }

    #[test]
    fn rejects_bad_numbers() {
        assert!(!is_valid_order_number("544850103554")); // no dash
        assert!(!is_valid_order_number("54485-103554")); // 5 digits left
        assert!(!is_valid_order_number("544850-10355")); // 5 digits right
        assert!(!is_valid_order_number("abc850-103554")); // letters
        assert!(!is_valid_order_number("544850_103554")); // wrong sep
    }

    #[test]
    fn parses_real_error_payload() {
        // Captured live from spot.photoprintit.com for an unknown order.
        let json = r#"{"resultDateTime":"2026-06-24T14:00:01+0200","summaryStateCode":"ERROR","summaryDate":"2026-06-24","summaryStateText":"Auftragsnummer nicht gefunden.  [DON]","summaryPrice":0,"summaryPriceText":"","currency":null,"language":"de_DE","customerNo":"544850","shopNo":null,"orderNo":"103554","orderDate":null,"deliveryType":-1,"deliveryText":"","infoText":null,"subOrders":[]}"#;
        let info: OrderInfo = serde_json::from_str(json).expect("deserialize");
        assert!(info.is_error());
        assert!(!info.is_ready());
        assert_eq!(info.customer_no.as_deref(), Some("544850"));
        assert_eq!(info.order_no.as_deref(), Some("103554"));
        assert_eq!(info.shop_no, None);
        assert!(info.timeline().is_empty());
    }

    #[test]
    fn readiness_matches_shipped_and_later() {
        assert!(!is_ready_code("SUBMITTED"));
        assert!(!is_ready_code("PROCESSING"));
        assert!(is_ready_code("SHIPPED"));
        assert!(is_ready_code("DELIVERED"));
        assert!(is_ready_code("PICKED_UP"));
        assert!(!is_ready_code("ERROR"));
    }

    #[test]
    fn done_at_delivered_or_picked_up() {
        assert!(!is_done_code("SUBMITTED"));
        assert!(!is_done_code("PROCESSING"));
        assert!(!is_done_code("SHIPPED"));
        assert!(is_done_code("DELIVERED"));
        assert!(is_done_code("PICKED_UP"));
    }

    #[test]
    fn pickup_timeline_marks_current_and_ready_label() {
        // Pickup order currently SHIPPED -> "in die Filiale geliefert".
        let steps = build_timeline("SHIPPED", 0);
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[0].status, StepStatus::Past); // SUBMITTED
        assert_eq!(steps[1].status, StepStatus::Past); // PROCESSING
        assert_eq!(steps[2].code, "SHIPPED");
        assert_eq!(steps[2].status, StepStatus::Current);
        assert_eq!(
            steps[2].label,
            "Dein Auftrag wird in den dm-Markt geliefert."
        );
        assert_eq!(steps[3].status, StepStatus::Future); // DELIVERED
    }

    #[test]
    fn home_timeline_has_transport_step() {
        let steps = build_timeline("TRANSPORT", 1);
        assert!(steps.iter().any(|s| s.code == "TRANSPORT"));
        assert_eq!(steps.len(), 6);
    }
}
