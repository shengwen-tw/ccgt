mod ccgt {
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

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Account {
        currency: String,
        balance: String,
        locked: String,
        stacked: String,
        r#type: String,
        fiat_currency: String,
        fiat_balance: String,
    }

    pub struct GridTradeBot {
        access_key: String,
        secret_key: String,
        accounts: Vec<Account>,
    }

    fn get_timestamp(time: SystemTime) -> u128 {
        let since_epoch = time.duration_since(UNIX_EPOCH).unwrap();
        since_epoch.as_millis()
    }

    impl GridTradeBot {
        pub fn new() -> GridTradeBot {
            dotenv::dotenv().ok();

            GridTradeBot {
                access_key: env::var("MAX_API_KEY").unwrap(),
                secret_key: env::var("MAX_API_SECRET").unwrap(),
                accounts: Vec::new(),
            }
        }

        pub fn load_yaml(&self, yaml_str: &str) {
            let docs = YamlLoader::load_from_str(yaml_str).unwrap();
            let doc = &docs[0];

            assert_eq!(doc["symbol"].as_str().unwrap(), "DOGETWD");
        }

        pub fn read_server_time(&self) -> i32 {
            let respond = reqwest::blocking::get("https://max-api.maicoin.com/api/v2/timestamp")
                .unwrap()
                .json::<i32>()
                .unwrap();
            //println!("server time: {:#?}", respond);

            respond
        }

        pub fn build_auth_client(
            &mut self,
            api_path: &str,
            params: &String,
            payload: &String,
        ) -> (reqwest::blocking::Client, String) {
            /* generate the signature */
            let mut signed_key =
                Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
            signed_key.update(payload.as_bytes());
            let signature = hex::encode(signed_key.finalize().into_bytes());

            /* setup request header */
            let mut header = header::HeaderMap::new();
            header.insert(
                "X-MAX-ACCESSKEY",
                header::HeaderValue::from_str(&self.access_key).unwrap(),
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
            let request = format!("https://max-api.maicoin.com/{}?{}", api_path, params);
            println!("{}", request);

            /* create request sender with the pre-defined header */
            let client = reqwest::blocking::Client::builder()
                .default_headers(header)
                .build()
                .unwrap();

            (client, request)
        }

        pub fn get_orders(&mut self) {
            let api_path = "/api/v2/orders";

            /* get milliseconds time of UNIX epoch time since 1970 */
            let timestamp = get_timestamp(SystemTime::now());

            #[derive(Serialize)]
            struct Payload {
                nonce: String,
                market: String,
                state: String,
                order_by: String,
                group_id: i32,
                pagination: bool,
                page: i32,
                limit: i32,
                offset: i32,
                path: String,
            }

            /* prepare payload data */
            let payload_raw = Payload {
                nonce: timestamp.to_string(),
                market: "dogetwd".to_string(),
                state: "%5Bwait%5D".to_string(),
                order_by: "desc".to_string(),
                group_id: 0,
                pagination: false,
                page: 1,
                limit: 100,
                offset: 0,
                path: api_path.to_string(),
            };

            let params = format!(
                "nonce={},market=\"{}\",state=%5B{}%5D,\
                 order_by=\"{}\",group_id={},pagination={},\
                 page={},limit={},offset={}",
                payload_raw.nonce,
                payload_raw.market,
                payload_raw.state,
                payload_raw.order_by,
                payload_raw.group_id,
                payload_raw.pagination,
                payload_raw.page,
                payload_raw.limit,
                payload_raw.offset
            );
            println!("params: {}", params);

            /* pack the payload with Base64 format */
            let payload_json_b64 =
                b64_encode(serde_json::to_string(&payload_raw).unwrap().as_bytes());
            println!("json: {}", serde_json::to_string(&payload_raw).unwrap());

            /* build client embedded with authorization info */
            let (client, request) = self.build_auth_client(&api_path, &params, &payload_json_b64);

            /* send the request and wait for the respond */
            let vec = client
                .get(request)
                .send()
                .unwrap()
                .json::<serde_json::Value>()
                .unwrap();
            println!("result: {:?}", vec);
        }

        pub fn sync_accounts_info(&mut self) {
            let api_path = "/api/v2/members/accounts";

            /* get milliseconds time of UNIX epoch time since 1970 */
            let timestamp = get_timestamp(SystemTime::now());

            #[derive(Serialize)]
            struct Payload {
                nonce: String,
                path: String,
            }

            /* prepare payload data */
            let payload_raw = Payload {
                nonce: timestamp.to_string(),
                path: api_path.to_string(),
            };

            let params = format!("nonce={}", timestamp.to_string());

            /* pack the payload with Base64 format */
            let payload_json_b64 =
                b64_encode(serde_json::to_string(&payload_raw).unwrap().as_bytes());

            /* build client embedded with authorization info */
            let (client, request) = self.build_auth_client(&api_path, &params, &payload_json_b64);

            /* send the request and wait for the respond */
            let vec = client
                .get(request)
                .send()
                .unwrap()
                .json::<Vec<serde_json::Value>>()
                .unwrap();
            //println!("result: {:?}", vec);

            /* read accounts */
            self.accounts.clear();
            for i in 0..vec.len() {
                let account = Account {
                    currency: vec[i]["currency"].to_string(),
                    balance: vec[i]["balance"].to_string(),
                    locked: vec[i]["locked"].to_string(),
                    stacked: vec[i]["stacked"].to_string(),
                    r#type: vec[i]["type"].to_string(),
                    fiat_currency: vec[i]["fiat_currency"].to_string(),
                    fiat_balance: vec[i]["balance"].to_string(),
                };
                self.accounts.push(account);

                println!("{:?}", &self.accounts[i]);
            }
        }
    }
}

fn main() {
    let mut trade_bot = ccgt::GridTradeBot::new();

    let s = "
    symbol:        DOGETWD
    quantity:      365
    grid_number:   50
    profit_spread: 0.03
    upper_price:   3.0
    lower_price:   2.1
    long:          true
    ";
    trade_bot.load_yaml(s);

    trade_bot.sync_accounts_info();
    trade_bot.get_orders();
    trade_bot.read_server_time();
}
