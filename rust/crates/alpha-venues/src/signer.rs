use crate::error::VenueError;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn sign_okx(
    secret: &str,
    timestamp: &str,
    method: &str,
    request_path: &str,
    body: &str,
) -> Result<String, VenueError> {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| VenueError::SignError {
            reason: e.to_string(),
        })?;

    mac.update(timestamp.as_bytes());
    mac.update(method.as_bytes());
    mac.update(request_path.as_bytes());
    if !body.is_empty() {
        mac.update(body.as_bytes());
    }

    let result = mac.finalize().into_bytes();
    Ok(BASE64.encode(result))
}

pub fn sign_binance(secret: &str, query: &str) -> Result<String, VenueError> {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| VenueError::SignError {
            reason: e.to_string(),
        })?;

    mac.update(query.as_bytes());
    let result = mac.finalize().into_bytes();
    Ok(hex::encode(result))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_okx_signature_format() {
        let secret = "test-secret-key-123";
        let timestamp = "2026-11-08T18:00:00.000Z";
        let method = "POST";
        let path = "/api/v5/trade/order";
        let body = r#"{"instId":"BTC-USDT"}"#;

        let sig = sign_okx(secret, timestamp, method, path, body);
        assert!(sig.is_ok());
        let sig_str = sig.unwrap();
        assert!(!sig_str.is_empty());
        assert!(BASE64.decode(sig_str).is_ok());
    }

    #[test]
    fn test_binance_signature_format() {
        let secret = "NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
        let query = "symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000&timestamp=1499827319559";

        let sig = sign_binance(secret, query).unwrap();
        assert_eq!(
            sig,
            "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71"
        );
    }
}
