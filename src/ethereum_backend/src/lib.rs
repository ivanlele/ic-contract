use candid::candid_method;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cdk_macros::{self, update, query};
use std::str::FromStr;

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