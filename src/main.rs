extern crate yaml_rust;

use std::str;
use yaml_rust::{YamlLoader};

fn load_yaml(yaml_str: &str)
{
    let docs = YamlLoader::load_from_str(yaml_str).unwrap();
    let doc = &docs[0];

    assert_eq!(doc["symbol"].as_str().unwrap(), "DOGETWD");
}

fn main() {
    let s = "
    symbol:        DOGETWD
    quantity:      365
    grid_number:   50
    profit_spread: 0.03
    upper_price:   3.0
    lower_price:   2.1
    long:          true
    ";

    load_yaml(s);
}
