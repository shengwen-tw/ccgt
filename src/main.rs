mod ccgt {
    extern crate yaml_rust;

    use base64::encode as b64_encode;
    use hmac::{Hmac, Mac, NewMac};
    use log::{error, info, warn, LevelFilter};
    use reqwest::header;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use serde::Serialize;
    use sha2::Sha256;
    use std::env;
    use std::fmt::Display;
    use std::io::Write;
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
            std::env::set_var("RUST_LOG", "info");
            env_logger::Builder::new()
                .format(|buf, record| {
                    writeln!(
                        buf,
                        "[{} {}] {}",
                        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                        record.level(),
                        record.args()
                    )
                })
                .filter(None, LevelFilter::Info)
                .target(env_logger::Target::Stdout)
                .init();

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

            assert_eq!(doc["symbol"].as_str().unwrap(), "dogetwd");
        }

        pub fn get_server_time(&self) -> i32 {
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
            //println!("{}", request);

            /* create request sender with the pre-defined header */
            let client = reqwest::blocking::Client::builder()
                .default_headers(header)
                .build()
                .unwrap();

            (client, request)
        }

        fn option_to_string<T: Display>(&self, option: Option<T>) -> String {
            match option {
                None => "".to_string(),
                Some(v) => v.to_string(),
            }
        }

        pub fn submit_order(&mut self) {
            let api_path = "/api/v2/orders";

            /* get milliseconds time of UNIX epoch time since 1970 */
            let timestamp = get_timestamp(SystemTime::now());

            #[derive(Serialize)]
            struct Payload {
                nonce: String,
                market: String,
                side: String,
                volume: Option<Decimal>,
                price: Option<Decimal>,
                client_oid: String,
                stop_price: Option<Decimal>,
                ord_type: String,
                group_id: Option<u64>,
                path: String,
            }

            /* prepare payload data */
            let payload_raw = Payload {
                nonce: timestamp.to_string(),
                market: "dogetwd".into(),
                side: "buy".into(),
                volume: Some(dec!(100000.0)),
                price: Some(dec!(1)),
                client_oid: "max_rs_api_case_create_order".into(),
                stop_price: None,
                ord_type: "limit".into(),
                group_id: None,
                path: api_path.into(),
            };

            let params = format!(
                "nonce={}&market={}&side={}&volume={}&\
                 price={}&client_oid={}&stop_price={}&\
                 ord_type={}&group_id={}",
                payload_raw.nonce,
                payload_raw.market,
                payload_raw.side,
                self.option_to_string(payload_raw.volume),
                self.option_to_string(payload_raw.price),
                payload_raw.client_oid,
                self.option_to_string(payload_raw.stop_price),
                payload_raw.ord_type,
                self.option_to_string(payload_raw.group_id),
            );
            //println!("params: {}", params);

            /* pack the payload with Base64 format */
            let payload_json_b64 =
                b64_encode(serde_json::to_string(&payload_raw).unwrap().as_bytes());
            //println!("json: {}", serde_json::to_string(&payload_raw).unwrap());

            /* build client embedded with authorization info */
            let (client, request) = self.build_auth_client(&api_path, &params, &payload_json_b64);

            /* send the request and wait for the respond */
            let respond = client
                .post(request)
                .send()
                .unwrap()
                .json::<serde_json::Value>()
                .unwrap();
            //println!("result: {:?}", respond);

            if respond["error"] != serde_json::Value::Null {
                error!(
                    "Failed to submit the order: {}",
                    respond["error"]["message"]
                );
            }
        }

        pub fn delete_order(&mut self) {
            let api_path = "/api/v2/order/delete";

            /* get milliseconds time of UNIX epoch time since 1970 */
            let timestamp = get_timestamp(SystemTime::now());

            #[derive(Serialize)]
            struct Payload {
                nonce: String,
                id: Option<u64>,
                client_oid: String,
                path: String,
            }

            /* prepare payload data */
            let payload_raw = Payload {
                nonce: timestamp.to_string(),
                id: Some(543210),
                client_oid: "max_rs_api_case_create_order".into(),
                path: api_path.into(),
            };

            let params = format!(
                "nonce={}&id={}&client_oid={}",
                payload_raw.nonce,
                self.option_to_string(payload_raw.id),
                payload_raw.client_oid,
            );
            //println!("params: {}", params);

            /* pack the payload with Base64 format */
            let payload_json_b64 =
                b64_encode(serde_json::to_string(&payload_raw).unwrap().as_bytes());
            //println!("json: {}", serde_json::to_string(&payload_raw).unwrap());

            /* build client embedded with authorization info */
            let (client, request) = self.build_auth_client(&api_path, &params, &payload_json_b64);

            /* send the request and wait for the respond */
            let respond = client
                .post(request)
                .send()
                .unwrap()
                .json::<serde_json::Value>()
                .unwrap();
            //println!("result: {:?}", respond);

            if respond["error"] != serde_json::Value::Null {
                error!(
                    "Failed to delete the order: {}",
                    respond["error"]["message"]
                );
            }
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
                group_id: Option<u64>,
                pagination: Option<bool>,
                page: Option<u64>,
                limit: Option<u64>,
                offset: Option<u64>,
                path: String,
            }

            /* prepare payload data */
            let payload_raw = Payload {
                nonce: timestamp.to_string(),
                market: "dogetwd".into(),
                state: "wait".into(),
                order_by: "asc".into(),
                group_id: None,
                pagination: Some(true),
                page: Some(1),
                limit: Some(100),
                offset: Some(0),
                path: api_path.into(),
            };

            let params = format!(
                "nonce={}&market={}&state={}&order_by={}&\
                 group_id={}&pagination={}&page={}&\
                 limit={}&offset={}",
                payload_raw.nonce,
                payload_raw.market,
                payload_raw.state,
                payload_raw.order_by,
                self.option_to_string(payload_raw.group_id),
                self.option_to_string(payload_raw.pagination),
                self.option_to_string(payload_raw.page),
                self.option_to_string(payload_raw.limit),
                self.option_to_string(payload_raw.offset)
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
                .json::<Vec<serde_json::Value>>()
                .unwrap();
            //println!("result: {:?}", vec);

            /* read orders */
            for i in 0..vec.len() {
                println!("{:?}", &vec[i]);
            }
        }

        pub fn sync_accounts(&mut self) {
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

        pub fn get_vip_level(&mut self) {
            let api_path = "/api/v2/members/vip_level";

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
            let response = client
                .get(request)
                .send()
                .unwrap()
                .json::<serde_json::Value>()
                .unwrap();
            println!("{:?}", response);
        }
    }
}

fn main() {
    let mut trade_bot = ccgt::GridTradeBot::new();

    let s = "
    symbol:        dogetwd
    quantity:      365
    grid_number:   50
    profit_spread: 0.03
    upper_price:   3.0
    lower_price:   2.1
    long:          true
    ";
    trade_bot.load_yaml(s);

    trade_bot.sync_accounts();
    trade_bot.get_orders();
    trade_bot.get_vip_level();
    trade_bot.get_server_time();
    trade_bot.submit_order();
    trade_bot.delete_order();
}
