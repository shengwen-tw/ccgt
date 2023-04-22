extern crate yaml_rust;

use base64::encode as b64_encode;
use hmac::{Hmac, Mac, NewMac};
use reqwest::header;
use serde::Serialize;
use sha2::Sha256;
use std::env;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};
use yaml_rust::YamlLoader;

#[derive(Serialize)]
struct Payload {
    nonce: String,
    path: String,
}

fn load_yaml(yaml_str: &str) {
    let docs = YamlLoader::load_from_str(yaml_str).unwrap();
    let doc = &docs[0];

    assert_eq!(doc["symbol"].as_str().unwrap(), "DOGETWD");
}

fn get_timestamp(time: SystemTime) -> u128 {
    let since_epoch = time.duration_since(UNIX_EPOCH).unwrap();
    since_epoch.as_millis()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let s = "
    symbol:        DOGETWD
    quantity:      365
    grid_number:   50
    profit_spread: 0.03
    upper_price:   3.0
    lower_price:   2.1
    long:          true
    ";

    /* load keys to interact with the cryptocurrency exchange */
    dotenv::dotenv().ok();
    let access_key = env::var("MAX_API_KEY").unwrap();
    let secret_key = env::var("MAX_API_SECRET").unwrap();

    /* get milliseconds time of UNIX epoch time since 1970 */
    let timestamp = get_timestamp(SystemTime::now());

    /* construct raw payload structure */
    let payload_raw = Payload {
        nonce: timestamp.to_string(),
        path: "/api/v2/members/me".to_string(),
    };
    let params = format!("nonce={}", timestamp.to_string());
    println!("nonce:{}, path:{}", payload_raw.nonce, payload_raw.path);

    /* pack the payload with Base64 format */
    let payload = b64_encode(serde_json::to_string(&payload_raw).unwrap().as_bytes());
    println!("payload: {}", payload);

    /* generate the signature */
    let mut signed_key = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()).unwrap();
    signed_key.update(payload.as_bytes());
    let signature = hex::encode(signed_key.finalize().into_bytes());
    println!("signature: {}", signature);

    /* setup request header */
    let mut header = header::HeaderMap::new();
    header.insert(
        "X-MAX-ACCESSKEY",
        header::HeaderValue::from_str(&access_key).unwrap(),
    );
    header.insert(
        "X-MAX-PAYLOAD",
        header::HeaderValue::from_str(&payload).unwrap(),
    );
    header.insert(
        "X-MAX-SIGNATURE",
        header::HeaderValue::from_str(&signature).unwrap(),
    );
    header.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );

    /* setup request content */
    let request = format!("https://max-api.maicoin.com/api/v2/members/me?{}", params);
    println!("{}", request);

    /* create request sender with the pre-defined header */
    let client = reqwest::Client::builder().default_headers(header).build()?;

    /* send the request and wait for the respond */
    let respond = client.get(request).send().await?;
    println!("{}", respond.text().await?);

    let respond = reqwest::get("https://max-api.maicoin.com/api/v2/timestamp")
        .await?
        .json::<i32>()
        .await?;
    println!("{:#?}", respond);

    load_yaml(s);

    Ok(())
}
