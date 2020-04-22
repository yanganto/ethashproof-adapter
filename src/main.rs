extern crate libc;
extern crate reqwest;
extern crate serde_json;

use actix_web::{post, App, HttpResponse, HttpServer, Responder};
use libc::c_char;
use regex::Regex;
use reqwest::header;
use std::ffi::CStr;
use std::{env, str, u64};

#[link(name = "ethash_proof")]
extern "C" {
    fn EthashProof(input: libc::c_ulonglong) -> *const c_char;
}

#[post("/")]
async fn adaptor(body: String) -> impl Responder {
    println!("=====================================",);
    println!("req: {}", body);
    println!("is json: {}", body.contains("\"json\""));
    if !body.contains("\"json\"") {
        println!("return 404 if not json");
        return HttpResponse::NotFound().body("Not Found".to_string());
    }
    // params: {"block_num": 1000, "transcation": false, "options": {"format": "json"}}
    let para = Regex::new(r#""block_num":\s?(\d+?),"#).unwrap();
    let block_number_str = para
        .captures_iter(&body)
        .nth(0)
        .map(|c| c[1].to_string())
        .unwrap();
    println!("block number: {:#?}", block_number_str);
    let block_number = u64::from_str_radix(&block_number_str, 10).unwrap();

    let req = if let Ok(api_key) = env::var("ETHSCANAPIKEY") {
        println!("api key: {}", api_key);
        let client = reqwest::Client::new();
        client
            .get(&format!(
			"https://api.etherscan.io/api?module=proxy&action=eth_getBlockByNumber&tag=0x{:x}&boolean=false&apikey={}", block_number, api_key)).send()
    } else {
        println!("block number: {}", block_number);
        let client = reqwest::Client::new();
        client
            .post("https://cloudflare-eth.com/")
            .header(header::CONTENT_TYPE, "application/json")
			.body(format!(r#"{{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x{:x}",false],"id":1}}"#, block_number))
            .send()
    };
    let output: *const c_char = unsafe { EthashProof(block_number) };
    let c_str: &CStr = unsafe { CStr::from_ptr(output) };
    let str_slice: &str = c_str.to_str().unwrap();
    let str_buf: String = str_slice.to_owned();

    let resp = req.await.unwrap();
    let resp_json: serde_json::Value = resp.json().await.unwrap();
    // println!("origin resp: {}", resp_json);

    let mut output_json: serde_json::Value = serde_json::from_str(
        r#"
		{
		  "id":1,
		  "jsonrpc":"2.0",
		  "result": {
			"eth_header":{} ,
			"proof": [
				 ]
		  }
		}
	"#,
    )
    .unwrap();
    // resp_json["proof"] = serde_json::Value::String(str_buf.into());
    output_json["result"]["eth_header"] = resp_json["result"].clone();
    let proof_str: String = str_buf.into();
    output_json["result"]["proof"] = serde_json::from_str(&proof_str).unwrap();
    println!("output resp: {}", output_json);
    HttpResponse::Ok().body(format!("{}", output_json))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(adaptor))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
