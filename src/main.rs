extern crate libc;
extern crate reqwest;
extern crate serde_json;

use actix_web::{post, App, HttpServer, Responder};
use libc::c_char;
use regex::Regex;
use reqwest::header;
use std::ffi::CStr;
use std::str;
use std::u64;

#[link(name = "ethash_proof")]
extern "C" {
    fn EthashProof(input: libc::c_ulonglong) -> *const c_char;
}

#[post("/")]
async fn adaptor(body: String) -> impl Responder {
    println!("req: {}", body);
    let para = Regex::new(r#"params":\s?\["0x([^"]+?)",\s?\w+\]"#).unwrap();
    let block_number_hex_str = para.captures_iter(&body).nth(0).map(|c| c[1].to_string());
    println!("block number: {:#?}", block_number_hex_str);
    let block_number = u64::from_str_radix(&block_number_hex_str.unwrap(), 16).unwrap();
    println!("block number: {}", block_number);
    let client = reqwest::Client::new();

    let req = client
        .post("https://cloudflare-eth.com/")
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .send();
    let output: *const c_char = unsafe { EthashProof(block_number) };
    let c_str: &CStr = unsafe { CStr::from_ptr(output) };
    let str_slice: &str = c_str.to_str().unwrap();
    let str_buf: String = str_slice.to_owned();

    let resp = req.await.unwrap();
    let mut resp_json: serde_json::Value = resp.json().await.unwrap();
    resp_json["proof"] = serde_json::Value::String(str_buf.into());
    format!("{}", resp_json)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(adaptor))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
