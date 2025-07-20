use anyhow_ext::Error;
// use crate::web::WebError;
use log::{debug, info, warn};
use reqwest::{header, Client};
use serde::Serialize;
use serde_json::json;
use std::result::Result;
use std::time::Duration;

/***********************************************************************************************
 * 更新OFF函数 rust重写.                                                                         *
 *                                                                                             *
 * INPUT:   none                                                                               *
 *                                                                                             *
 * OUTPUT:  none                                                                               *
 *                                                                                             *
 * WARNINGS:   You must call updata() before using a button constructed with this function,    *
 *                                                                                             *
 *                                                                                             *
 * HISTORY:                                                                                    *
 *   06/12/2024 SCT : Created.                                                                 *
 *=============================================================================================*/

/**
 * 更新的数据格式
 */
#[derive(Serialize, Debug)]
struct UpdateData {
    sid: String,
    priority: i32,
    esl_id: String,
    bak_url: String,
    template: String,
}

/**
 ** 获取制定的文件内容，并去标eslid=开头
 */
fn get_esl(fp: &str) -> Vec<String> {
    let mut esl_list = Vec::new();
    if let Ok(contents) = std::fs::read_to_string(fp) {
        for line in contents.lines() {
            if line.starts_with("eslid=") {
                if let Some(id) = line.strip_prefix("eslid=") {
                    esl_list.push(id.trim().to_owned());
                }
            } else {
                warn!("{:#?}", &line);
                esl_list.push(line.to_string());
            }
        }
    }

    esl_list
}

/** 更新函数，刷OFF
 **
 */
pub async fn up_off() -> Result<(), Box<dyn std::error::Error>> {
    let bak_url_ = "http://172.16.120.59:8083";
    let req_url_ = "http://172.16.120.59:9100/api3/god.2/esls";
    let client = Client::new();

    let esl_list = get_esl("/Users/kali/loopupgrade/src/esl.txt");
    let mut params = Vec::new();
    for e in esl_list {
        let _data = UpdateData {
            sid: "39847999881".to_string(),
            priority: 10,
            esl_id: e,
            bak_url: bak_url_.to_string(),
            template: "UNBIND".to_string(),
        };
        params.push(_data);
    }
    // print!("{:#?}", &params);
    let data = serde_json::to_string(&params)?;
    let response = client
        .put(req_url_)
        .timeout(Duration::from_secs(10))
        .header("Content-Type", "application/json")
        .body(data)
        .send()
        .await?;

    info!("Status: {}", response.status());
    Ok(())
}

const EW: &str = "127.0.0.1:9000";
const UC: &str = "god.2";
const BAK_URL: &str =
    "http://172.16.120.59:8083/shopweb-webapp/ogi/ew/httpHandler?customerCode=god";
const ALLSTAR_LOGIN: &str = "http://172.16.120.59:8084/proxy/allstar/user/login";
const PS_BIND: &str = "http://172.16.120.59:8084/proxy/prismart/esl/god/2/binding";
const AURORA_BIND: &str = "http://172.16.120.59:8084/proxy/aurora/deviceGoods/bind";
const AURORA_CHECK: &str = "http://172.16.120.59:8084/proxy/aurora/deviceGoods/getList";
const AURORA_UN: &str = "superuser";
const AURORA_PW: &str = "ACD7600A8A2F0AA9AAB51C29F50C64A1";
const AURORA_DEVICE: [&str; 2] = ["218972322667593985", "223973162683266819"];

pub fn as_bind() -> Result<(), Error> {
    let upate_esl = get_esl("/Users/kali/loopupgrade/src/esl.txt");
    let mut headers = header::HeaderMap::new();
    headers.insert("User-Agent", header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"));
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json;charset=utf-8"),
    );

    let client = reqwest::blocking::Client::new();
    let aurora_login_info = client
        .post(ALLSTAR_LOGIN)
        .headers(headers.clone())
        .json(&json!({"username": AURORA_UN, "password": AURORA_PW}))
        .send()?
        .json::<serde_json::Value>()?; // Assuming the response is JSON

    let mut data = Vec::new();
    for e in &upate_esl {
        let d = json!({
            "eslId": e,
            "goodsSku": "123",
            "position": 0,
            "extra": {}
        });
        data.push(d);
    }

    let access_token = &aurora_login_info["data"]["access_token"];

    let resp = client
        .post(PS_BIND)
        .headers(headers)
        .bearer_auth(access_token)
        .json(&data)
        .send()?
        .json::<serde_json::Value>()?; // Assuming the response is JSON

    println!("{:?}", resp);

    Ok(())
}

