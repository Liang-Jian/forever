const EW: &str = "127.0.0.1:9000";
const UC: &str = "god.2";
const BAK_URL: &str = "http://172.16.120.59:8083/shopweb-webapp/ogi/ew/httpHandler?customerCode=god";
const ALLSTAR_LOGIN: &str = "http://172.16.120.59:8084/proxy/allstar/user/login";
const PS_BIND: &str = "http://172.16.120.59:8084/proxy/prismart/esl/god/2/binding";
const AURORA_BIND: &str = "http://172.16.120.59:8084/proxy/aurora/deviceGoods/bind";
const AURORA_CHECK: &str = "http://172.16.120.59:8084/proxy/aurora/deviceGoods/getList";
const AURORA_UN: &str = "superuser";
const AURORA_PW: &str = "ACD7600A8A2F0AA9AAB51C29F50C64A1";
const AURORA_DEVICE: [&str; 2] = ["218972322667593985", "223973162683266819"];

struct ASUpdate {
    update_price: i32,
    epd_list: Vec<String>,
    epd_status: bool,
    lcd_status: bool,
    lcd_update_icon: bool,
    s: Client,
    aurora_login_info: serde_json::Value,
    start_update_time: Option<u64>,
}


fn get_esl(fp: &str) -> Vec<String> {
    let file = File::open(fp).expect("Failed to open file");
    let reader = BufReader::new(file);
    let mut esl_list = Vec::new();
    
    for line in reader.lines() {
        if let Ok(l) = line {
            if let Some(id) = l.strip_prefix("eslid=") {
                esl_list.push(id.trim().to_owned());
            }
        }
    }
    
    esl_list
}

impl ASUpdate {
    async fn run(&mut self) -> Result<(), reqwest::Error> {
        let data: Vec<serde_json::Value> = self.epd_list.iter().map(|e| {
            json!({
                "eslId": e,
                "goodsSku": "123",
                "position": 0,
                "extra": {}
            })
        }).collect();
        
        let response = self.s.post(PS_BIND)
            .json(&data)
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", self.aurora_login_info["data"]["access_token"].as_str().unwrap()))
            .send()
            .await?;
        
        println!("{:?}", response.text().await?);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let mut as_update = ASUpdate {
        update_price: 0,
        epd_list: get_esl("esl.txt"),
        epd_status: true,
        lcd_status: false,
        lcd_update_icon: true,
        s: reqwest::Client::new(),
        aurora_login_info: reqwest::Client::new().post(ALLSTAR_LOGIN)
            .json(&json!({"username": AURORA_UN, "password": AURORA_PW}))
            .send()
            .await?
            .json()
            .await?,
        start_update_time: None,
    };
    
    as_update.run().await?;
    Ok(())
}