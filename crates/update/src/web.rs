
use anyhow_ext::{Ok, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use regex::Regex;

/// 升级基站
pub async fn upgrade_ap() -> Result<()> {
    // 创建一个新的 HTTP 客户端
    let client = Client::new();

    // 定义要发送的 JSON 数据
    let data = vec![json!({
        "apMac": "98:6D:35:79:C5:87",
        "back_url": "http://127.0.0.1:8080",
        "data": {
            "server_url": "http://127.0.0.1:8080/blob/apv5img/image.7.1.29_rc6.tar"
        },
        "type": 52
    })];

    // 定义目标 URL
    let url = "http://172.16.120.59:9264/api3/default/aps/management";

    // 发送 PUT 请求，携带 JSON 数据
    let res = client.put(url).json(&data).send().await?;

    // 检查响应状态码
    if res.status().is_success() {
        println!("请求成功，状态码：{}", res.status());
    } else {
        println!("请求失败，状态码：{}", res.status());
    }

    // 输出响应头
    println!("Headers:\n{:#?}", res.headers());

    // 获取响应体
    let body = res.text().await?;
    println!("Body:\n{}", body);

    Ok(())
}

fn check_essid(input: &str) -> bool {
    // 创建一个正则表达式来匹配 ESSID 的值
    let re = Regex::new(r#"ESSID:"(.*?)""#).unwrap();

    // 使用正则表达式进行匹配
    if let Some(captures) = re.captures(input) {
        // 获取 ESSID 的内容
        if let Some(essid) = captures.get(1) {
            // 判断 ESSID 是否为空
            return !essid.as_str().is_empty();
        }
    }

    false
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct CgiParm {
    pub ssl: String,
    #[serde_as(as = "DisplayFromStr")]
    pub wss: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub port: u16,
    #[serde_as(as = "DisplayFromStr")]
    pub net_dhcp: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub net_iptype: String,

    #[serde_as(as = "DisplayFromStr")]
    pub ew_auto: bool,
    pub ew_ipaddr: String,
    #[serde_as(as = "DisplayFromStr")]
    pub ew_port: u16,
    #[serde_as(as = "DisplayFromStr")]
    pub ew_ssl: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub ew_ssl_mutual_auth: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub ew_wss: bool,
}

fn into2data() -> Result<()> {
    let mut data: HashMap<String, String> = HashMap::new();
    data.insert("ssl".to_string(), "enabled".to_string());
    data.insert("wss".to_string(), "true".to_string());
    data.insert("port".to_string(), "8080".to_string());
    data.insert("net_dhcp".to_string(), "true".to_string());
    data.insert("net_iptype".to_string(), "static".to_string());
    data.insert("net_ipaddr".to_string(), "192.168.1.100".to_string());
    data.insert("net_ipaddr6".to_string(), "::1".to_string());
    data.insert("net_netmask".to_string(), "255.255.255.0".to_string());
    data.insert("net_router".to_string(), "192.168.1.1".to_string());
    data.insert("net_router6".to_string(), "::1".to_string());
    data.insert("net_dns1".to_string(), "8.8.8.8".to_string());
    data.insert("net_dns2".to_string(), "8.8.4.4".to_string());
    data.insert("ew_auto".to_string(), "false".to_string());
    data.insert("ew_ipaddr".to_string(), "example.com".to_string());
    data.insert("ew_port".to_string(), "443".to_string());
    data.insert("ew_ssl".to_string(), "true".to_string());
    data.insert("ew_ssl_mutual_auth".to_string(), "false".to_string());
    data.insert("ew_wss".to_string(), "true".to_string());

    // 将 HashMap<String, String> 转换为 serde_json::Value
    let json_value = serde_json::to_value(&data)?;

    // 反序列化为 CgiParm
    let cgi_parm: CgiParm = serde_json::from_value(json_value)?;

    println!("{:#?}", cgi_parm);

    Ok(())
}

fn check() {
    let itesm = vec!["a", "b", "c"];
    for (idx, item) in itesm.iter().enumerate() {
        println!("{}={}", idx + 1, item);
    }
}

