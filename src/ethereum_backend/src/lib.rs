use candid::candid_method;
use ic_cdk_macros::{self, update, query};
use std::str::FromStr;
use serde_json::{Value};

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, TransformContext, HttpResponse, TransformArgs
};

use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, KeyInfo};
use ic_web3::{
    ethabi::ethereum_types::U256,
    types::{Address, TransactionParameters},
};

const URL: &str = "https://goerli.infura.io/v3/d009354476b140008dd04c741c00341b";
const CHAIN_ID: u64 = 5;
const KEY_NAME: &str = "dfx_test_key";

const COINMARKETCAP_URL: &str = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol=ETH";
const COINMARKETCAP_API_KEY: &str = "f3c6adf6-c252-4869-a762-0dde0dee4779";

type Result<T, E> = std::result::Result<T, E>;

#[query(name = "transform")]
#[candid_method(query, rename = "transform")]
fn transform(response: TransformArgs) -> HttpResponse {
    response.response
}

#[update(name = "get_canister_addr")]
#[candid_method(update, rename = "get_canister_addr")]
async fn get_canister_addr() -> Result<String, String> {
    match get_eth_addr(None, None, KEY_NAME.to_string()).await {
        Ok(addr) => { Ok(hex::encode(addr)) },
        Err(e) => { Err(e) },
    }
}

#[update(name = "send_eth")]
#[candid_method(update, rename = "send_eth")]
async fn send_eth(to: String, value: u64) -> Result<String, String> {
    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{
        derivation_path: derivation_path, key_name: KEY_NAME.to_string(), ecdsa_sign_cycles: None
    };

    let from_addr = get_eth_addr(None, None, KEY_NAME.to_string())
        .await
        .map_err(|e| format!("get canister eth addr failed: {}", e))?;
    
    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let tx_count = w3.eth()
        .transaction_count(from_addr, None)
        .await
        .map_err(|e| format!("get tx count error: {}", e))?;

    ic_cdk::println!("canister eth address {} tx count: {}", hex::encode(from_addr), tx_count);
    
    let to = Address::from_str(&to).unwrap();
    let tx = TransactionParameters {
        to: Some(to),
        nonce: Some(tx_count),
        value: U256::from(value),
        gas_price: Some(U256::exp10(10)),
        gas: U256::from(21000),
        ..Default::default()
    };

    let signed_tx = w3.accounts()
        .sign_transaction(tx, hex::encode(from_addr), key_info, CHAIN_ID)
        .await
        .map_err(|e| format!("sign tx error: {}", e))?;
    match w3.eth().send_raw_transaction(signed_tx.raw_transaction).await {
        Ok(txhash) => { 
            ic_cdk::println!("txhash: {}", hex::encode(txhash.0));
            Ok(format!("{}", hex::encode(txhash.0)))
        },
        Err(e) => { Err(e.to_string()) },
    }
}

#[update(name = "send_eth_with_payload")]
#[candid_method(update, rename = "send_eth_with_payload")]
async fn send_eth_with_payload(to: String, value: u64) -> Result<String, String> {
    let price = get_eth_price().await
        .expect("failed to get price");

    let encoded_price = hex::encode(price);

    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{
        derivation_path, key_name: KEY_NAME.to_string(), ecdsa_sign_cycles: None
    };

    let from_addr = get_eth_addr(None, None, KEY_NAME.to_string())
        .await
        .map_err(|e| format!("get canister eth addr failed: {}", e))?;
    
    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let tx_count = w3.eth()
        .transaction_count(from_addr, None)
        .await
        .map_err(|e| format!("get tx count error: {}", e))?;

    ic_cdk::println!("canister eth address {} tx count: {}", hex::encode(from_addr), tx_count);

    let gas_price = w3.eth()
        .gas_price()
        .await
        .map_err(|e| format!("get gas price error: {}", e))?;

    let to = Address::from_str(&to).unwrap();
    let tx = TransactionParameters {
        to: Some(to),
        nonce: Some(tx_count),
        value: U256::from(value),
        data: ic_web3::types::Bytes(encoded_price.into_bytes()),
        gas_price: Some(gas_price),
        gas: U256::from(50_000),
        ..Default::default()
    };

    let signed_tx = w3.accounts()
        .sign_transaction(tx, hex::encode(from_addr), key_info, CHAIN_ID)
        .await
        .map_err(|e| format!("sign tx error: {}", e))?;

    match w3.eth().send_raw_transaction(signed_tx.raw_transaction).await {
        Ok(txhash) => { 
            ic_cdk::println!("txhash: {}", hex::encode(txhash.0));
            Ok(format!("{}", hex::encode(txhash.0)))
        },
        Err(e) => { Err(e.to_string()) },
    }
}

#[update(name = "get_eth_price")]
#[candid_method(update, rename = "get_eth_price")]
async fn get_eth_price() -> Result<String, String> {
    let request_headers = vec![
        HttpHeader {
            name: "X-CMC_PRO_API_KEY".to_string(),
            value: COINMARKETCAP_API_KEY.to_string(),
        }
    ];

    let request = CanisterHttpRequestArgument {
        url: COINMARKETCAP_URL.to_string(),
        method: HttpMethod::GET,
        body: None,
        max_response_bytes: None,
        transform: Some(TransformContext::new(transform, vec![])),
        headers: request_headers,
    };

    match http_request(request).await {
        Ok((response,)) => {
            let str_body = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");

            let response_json: Value = serde_json::from_str(&str_body)
                .expect("Failed to parse json");
            
            let eth_price = response_json["data"]["ETH"]["quote"]["USD"]["price"].to_string();

            Ok(eth_price.to_owned())
        }
        Err((r, m)) => {
            return Err(format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}"));
        }
    }
}