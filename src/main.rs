use anyhow_ext::{anyhow, Context, Result};
// use base64::encode;
use base64::{engine::general_purpose::STANDARD, Engine};
// use base64::Engine::encode;
use chrono::{Local, NaiveTime, Timelike};
use image::{GenericImageView, ImageOutputFormat, Rgba, RgbaImage};
use log::info;
use rand::distributions::Alphanumeric;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use regex::Regex;
use reqwest::Client;
use rusttype::{point, Font, Scale};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt::{self};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom};
use std::time::Duration;
use tokio::time::sleep;

#[allow(dead_code)]
#[derive(Serialize)]
struct FlashLight {
    colors: Vec<String>,
    on_time: String,
    led_rule: String, // 0是预置 1是接口
    off_time: String,
    flash_count: String,
    sleep_time: String,
    loop_count: String,
    task_id: String,
}

#[allow(dead_code)]
#[derive(Serialize)]
struct FlashControlData {
    sid: String,
    priority: u32,
    back_url: String,
    operation_type: String,
    flash_light: FlashLight,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Page {
    id: u32,
    name: String,
    image: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Screen {
    name: String,
    default_page: String,
    default_page_id: String,
    pages: Vec<Page>,
}
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ESLupdate {
    sid: String,
    priority: u32,
    esl_id: String,
    back_url: String,
    screen: Screen,
}

impl fmt::Display for ESLupdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ESLupdate {{ sid: {}, priority: {}, esl_id: {}, back_url: {} }}",
            self.sid, self.priority, self.esl_id, self.back_url
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EwConf {
    pub api: String,            // ewapi
    uc: String,                 // usercode
    pub back_url: String,       // back url
    pub epd_wl: String,         //
    pub ewlog: String,          // ew 日志
    pub startprice: i32,        // 开始的价格
    pub limittime: [String; 2], // 休眠时间
    #[serde(skip_serializing, skip_deserializing)]
    pub esl_id_list: Vec<String>, // 要更新的epd
    #[serde(skip_serializing, skip_deserializing)]
    pub starttime: Option<NaiveTime>, // 开始时间
    #[serde(skip_serializing, skip_deserializing)]
    fileseek: u64, // 文件指针位置
    pub template: Option<String>, // 自定义更细模版文件夹
    pub auto: Option<bool>,     // 是否不查询日志
    pub autotime: Option<u64>,  // 定时更新
}

struct RunTime {
    st: NaiveTime,
    et: NaiveTime,
}

impl RunTime {
    fn timediff(&self) -> Duration {
        // 计算时间差，得到的是一个 `chrono::Duration`
        let duration = self.et.signed_duration_since(self.st);
        // 将 `chrono::Duration` 转换为 `std::time::Duration`
        Duration::from_secs(duration.num_seconds() as u64)
    }
}
// 计算休眠时间
pub fn need_sleep_time(args: &[String; 2]) -> u64 {
    let time1_str = &args[0];
    let time2_str = &args[1];
    let time1 = NaiveTime::parse_from_str(time1_str, "%H:%M").expect("cant't");
    let time2 = NaiveTime::parse_from_str(time2_str, "%H:%M").expect("ÎÞ·¨½âÎöÊ±¼ä");
    let seconds1 = time1.num_seconds_from_midnight();
    let seconds2 = time2.num_seconds_from_midnight();
    let difference = (seconds2 - seconds1) as u64;
    difference
}

// 生成制定长度的随机字符串 ，用来sid
fn generate_random_string(length: usize) -> String {
    // 创建一个线程安全的随机数生成器
    let mut rng = thread_rng();
    // 生成指定长度的随机字符串
    (0..length)
        .map(|_| rng.sample(Alphanumeric))
        .map(char::from)
        .collect()
}

// 获取初始文件seek
pub fn get_eslwlog_seek(fp: &str) -> Result<u64> {
    let mut file = File::open(fp)?;
    let position = file.seek(SeekFrom::End(0))?;
    info!("start file seek ={}", position);
    Ok(position)
}

/// 获取id
pub fn get_esl_id_out(fp: &String, uc: &String) -> Result<Vec<String>> {
    let file = File::open(fp).expect("epd_wl is not found,pls check");
    let reader = io::BufReader::new(file);

    let uc_suffix = format!("={}", uc);
    let esl_list: Vec<String> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.replace("\n", "").replace(&uc_suffix, ""))
        .collect();
    info!("esl list len = {}", esl_list.len());
    Ok(esl_list)
}

// 读取png转为图片
pub fn make_self_pic(fpdir: String) -> Result<String> {
    // 打开文件
    let mut file = fs::File::open(fpdir)?;
    // 读取文件内容到缓冲区
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    // 将缓冲区编码为 Base64 字符串
    let encoded = STANDARD.encode(&buffer);
    // 打印或使用 encoded 字符串
    println!("Base64 encoded PNG: {}", encoded);
    Ok(encoded)
}

/// 生成的模版为15m 大模版
pub fn make_auto_pic(random_number: i32) -> String {
    // 读取 PNG 图片
    let img = image::open("src/test.png").expect("test.png need in you src path");
    // 获取图片的宽度和高度
    let (width, height) = img.dimensions();
    // 获取图片的 RGBA 像素数据
    let binding = img.to_rgba8();
    let mut pixels: Vec<_> = binding.pixels().collect();
    // 打乱像素数据
    let mut rng = thread_rng();
    pixels.shuffle(&mut rng);
    // 创建一个新的 RGBA 图像
    let mut output_image = RgbaImage::new(width, height);
    // 将打乱的像素重新填充到图像中
    for (i, pixel) in pixels.into_iter().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        output_image.put_pixel(x, y, *pixel);
    }

    // 加载字体文件
    let font_data = include_bytes!("SourceCodePro-Black.ttf") as &[u8];
    let font = Font::try_from_bytes(font_data).expect("Error loading font file");

    // 随机生成一个数字
    // let random_number = rng.gen_range(10..100);

    // 设置字体大小和位置
    let scale = Scale {
        x: 1200.0,
        y: 1200.0,
    };
    let v_metrics = font.v_metrics(scale);
    let text = random_number.to_string();
    let glyphs: Vec<_> = font
        .layout(&text, scale, point(0.0, v_metrics.ascent))
        .collect();

    // 计算文本绘制的起始位置
    let glyph_width = glyphs.iter().rev().next().unwrap().position().x as u32;
    let glyph_x_offset = (width - glyph_width) / 2;
    let glyph_y_offset = (height - v_metrics.ascent as u32) / 2;

    // 绘制随机数字到图像中间
    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                let x = x + glyph_x_offset + bounding_box.min.x as u32;
                let y = y + glyph_y_offset as u32;
                let value = (v * 255.0) as u8;
                output_image.put_pixel(x, y, Rgba([255, 255, 255, value])); // 白色字体
            });
        }
    }

    // 将图像保存到内存中
    let mut buffer = Cursor::new(Vec::new());
    output_image
        .write_to(&mut buffer, ImageOutputFormat::Png)
        .unwrap();

    // 获取图像数据的字节数组
    let image_data = buffer.into_inner();
    // 将图像字节数组编码为 Base64 字符串
    let base64_string = STANDARD.encode(&image_data);
    // 打印出 Base64 编码的字符串
    // println!("Base64 Encoded Image: {}", base64_string);
    // 保存打乱后的图片并包含随机数字
    // output_image.save("output_image_with_number.png").unwrap();
    // println!("图片处理完成并保存为 output_image_with_number.png");
    info!("make pic finish");
    base64_string
}

impl EwConf {
    fn new() -> Self {
        let _f = File::open("src/conf.txt").expect("conf.txt is not found"); // 打开文件
        let reader = BufReader::new(_f); // 创建一个带缓冲区的读取器
                                         // 将JSON数据解析为 EwConf 结构体
        let conf_info: EwConf = serde_json::from_reader(reader).unwrap();
        let esl_id_list_ = get_esl_id_out(&conf_info.epd_wl, &conf_info.uc).unwrap();
        let start_fileseek = get_eslwlog_seek(&conf_info.ewlog).expect("ew log path not found");
        let tpt = conf_info.template;

        if conf_info.auto.unwrap_or(false) && conf_info.autotime.is_none() {
            anyhow!("配置错误：当 `auto` 为 true 时，`autotime` 不能为空");
        }
        let autotime = conf_info.autotime;
        Self {
            api: conf_info.api,
            uc: conf_info.uc,
            back_url: conf_info.back_url,
            epd_wl: conf_info.epd_wl,
            ewlog: conf_info.ewlog,
            startprice: conf_info.startprice,
            limittime: conf_info.limittime,
            esl_id_list: esl_id_list_,
            starttime: None,
            fileseek: start_fileseek,
            template: tpt,        // 自定义更新模版
            auto: conf_info.auto, // 间隔日志
            autotime: autotime,   // 间隔时间 s
        }
    }

    fn is_during(start_time_: &str, end_time_: &str) -> bool {
        let mut status = false;
        let start_time = NaiveTime::parse_from_str(start_time_, "%H:%M").unwrap();
        let end_time = NaiveTime::parse_from_str(end_time_, "%H:%M").unwrap();
        let now = Local::now().time();
        if now >= start_time && now <= end_time {
            info!("check time pass ");
            status = true;
        }
        status
    }
    // 如果recv和 eslid不一致，需要看下是哪个价签有问题
    pub fn check_is_in(&mut self, all: &Vec<String>, recv: &Vec<String>) {
        if all.len() == recv.len() {
            return;
        }
        let diff1: Vec<_> = all.iter().filter(|x| !recv.contains(x)).collect();
        if !diff1.is_empty() {
            info!("{:?} not in recv list, please check", diff1);
        }
    }

    //  获取eslid的尺寸用来的自定义模版，返回{"eslid":"templatename"}
    pub async fn get_esl_id_size(&mut self, esl: &Vec<String>) -> Result<HashMap<String, String>> {
        //127.0.0.1:9000/api3/esls/36-F0-BF-8B
        let cli = Client::new();
        let mut tmpresult: HashMap<String, String> = HashMap::new();
        for ev in esl {
            let data: HashMap<String, Value> = cli
                .get(format!("http://{a}/api3/esls/{e}", a = self.api, e = ev))
                .send()
                .await?
                .json()
                .await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            let picname = data
                .get("data")
                .ok_or(anyhow!("can't find key data"))?
                .get("description")
                .ok_or(anyhow!("no exist description"))?;
            tmpresult.insert(String::from(ev), picname.to_string());
        }
        info!("final hashmap ={}", serde_json::to_string(&tmpresult)?);
        Ok(tmpresult)
    }

    /// 读取eslid问题
    pub fn get_esl_id(&mut self) -> Result<Vec<String>> {
        let file = File::open(&self.epd_wl).unwrap();
        let reader = io::BufReader::new(file);

        let uc_suffix = format!("={}", self.uc);
        let esl_list: Vec<String> = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter(|line| !line.starts_with('#'))
            .map(|line| line.replace("\n", "").replace(&uc_suffix, ""))
            .collect();

        info!("esl list len = {}", esl_list.len());
        Ok(esl_list)
    }

    // async fn send_flash_control(&mut self, client: &Client, url: String, headers: reqwest::header::HeaderMap, data: FlashControlData) -> Result<()> {
    //     let response = client
    //         .put(&url)
    //         .headers(headers.clone())
    //         .json(&data)
    //         .send()
    //         .await
    //         .with_context(|| format!("发送 PUT 请求到 URL: {}", url))?;

    //     if response.status().is_success() {
    //         let response_text = response.text().await.with_context(|| "读取响应文本失败")?;
    //         println!("请求成功，响应内容:\n{}", response_text);
    //     } else {
    //         let status = response.status();
    //         let response_text = response.text().await.unwrap_or_default();
    //         println!(
    //             "请求失败，状态码: {}, 响应内容: {}",
    //             status, response_text
    //         );
    //     }

    //     Ok(())
    // }

    // 循环更新用
    pub async fn singlerun(mut self) {
        info!("start loop only update");
        loop {
            let _ = self.update().await;
            sleep(Duration::from_secs(300)).await;
        }
    }

    // 获取电池电量信息。并且美观输出
    // fn get_battery_info(&mut self, file_max_seek: u64, api_log_fp: &str) -> Result<()> {
    //     // 获取 ESL ID 列表
    //     let esl_ids = self.esl_id_list;

    //     // 打开 battery_fp 文件，以追加模式
    //     let mut battery_file: File = OpenOptions::new()
    //         .create(true)
    //         .append(true)
    //         .open(self.template.unwrap())
    //         .with_context(|| format!("can't open or create file : {}", self.template.unwrap()))?;

    //     info!("write battery start");

    //     // 预编译正则表达式
    //     let re = Regex::new(r",query_type=53,battery=(.*?),sid=").expect("正则表达式编译失败");

    //     // 打开 api_log_fp 文件，以只读模式
    //     let api_log_file = OpenOptions::new()
    //         .read(true)
    //         .open(api_log_fp)
    //         .with_context(|| format!("无法打开文件: {}", api_log_fp))?;

    //     let mut reader = BufReader::new(api_log_file);
    //     reader.seek(SeekFrom::Start(file_max_seek)).context("文件定位失败")?;

    //     // 逐行读取日志文件
    //     let mut lines = reader.lines();

    //     while let Some(line) = lines.next() {
    //         let line = line.context("读取日志文件失败")?;
    //         // 检查是否包含特定关键词
    //         if line.contains("category=api,action=prepare_ack,cmd=ESL_STATISTICS_QUERY_ACK") {
    //             for esl in &esl_ids {
    //                 if line.contains(&format!("esl_id={}", esl)) {
    //                     if let Some(caps) = re.captures(&line) {
    //                         let battery_power = caps.get(1)
    //                             .map_or("0".to_string(), |m| m.as_str().to_string());

    //                         // 假设日期时间信息位于行首 23 个字符
    //                         let dt = if line.len() >= 23 {
    //                             &line[..23]
    //                         } else {
    //                             "none"
    //                         };

    //                         // 记录电池信息到文件
    //                         writeln!(battery_file, "{} - esl={};battery={}", dt, esl, battery_power).context("写入电池信息失败")?;
    //                         // 日志记录
    //                         info!("{} - esl={};battery={}", dt, esl, battery_power);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    //     info!("write battery finish");
    //     Ok(())
    // }

    pub async fn update(&mut self) -> Result<()> {
        if self.template.is_some() {
            self.update_tpl().await?;
        } else {
            self.update_pic().await?;
        }
        Ok(())
    }

    // 下发更新
    async fn update_tpl(&mut self) -> Result<()> {
        // 将 esl_id_list 按每 200 个一组分块处理
        for esl_chunk in self.esl_id_list.chunks(200) {
            let mut batch = Vec::new();
            // 为当前块的每个 esl_id 创建 ESLupdate 并添加到 batch
            for e in esl_chunk {
                let d = json!({
                    "sid": generate_random_string(12),
                    "esl_id": e,
                    "priority": 1,
                    "back_url": self.back_url,
                    "store_name": self.uc,
                    "price": self.startprice,
                    "template": self.template,
                });
                batch.push(d);
            }

            // 构建请求数据
            let data = json!({ "data": batch });
            let client = Client::new();
            let response: reqwest::Response = client
                .put(&format!("http://{}/api3/{}/esls", self.api, self.uc))
                .json(&data)
                .send()
                .await
                .map_err(|e| anyhow_ext::Error::from(e))?;

            // 检查请求是否成功
            if response.status().is_success() {
                info!("Request was successful for a batch of 200!");
            } else {
                info!("Request failed with status: {}", response.status());
            }

            // 每批发送完成后休眠一段时间，避免请求过快
            sleep(Duration::from_millis(200)).await;
        }

        // 记录开始时间和价格更新
        self.starttime = Some(Local::now().time());
        info!(
            "Update  send over and price is {}; update start time = {:?} ",
            self.startprice, &self.starttime
        );
        self.startprice += 1; // 价格增加
        Ok(())
    }

    // 下发更新
    async fn update_pic(&mut self) -> Result<()> {
        // 将按每 30 个一组分块处理，要不太快
        let sid_info = generate_random_string(12);
        let img_data = make_auto_pic(self.startprice);
        for esl_chunk in self.esl_id_list.chunks(30) {
            let mut batch = Vec::new();
            for e in esl_chunk {
                let d = json!({
                    "sid": sid_info,
                    "esl_id": e,
                    "priority": 10,
                    "back_url": self.back_url,
                    "screen": {
                        "name": e,
                        "default_page": "normal",
                        "default_page_id": "0",
                        "pages": [
                            {
                                "id": 0,
                                "name": "normal",
                                "image": img_data
                            },
                        ]
                    },
                });
                batch.push(d);
            }
            // 构建请求数据
            let data = json!({ "data": batch });
            let client = Client::new();
            let response: reqwest::Response = client
                .put(&format!("http://{}/api3/{}/esls", self.api, self.uc))
                .json(&data)
                .send()
                .await
                .map_err(|e| anyhow_ext::Error::from(e))?;

            // 检查请求是否成功
            if response.status().is_success() {
                info!("Request was successful for a batch of 200!");
            } else {
                info!("Request failed with status: {}", response.status());
            }
            // 每批发送完成后休眠一段时间，避免请求过快
            sleep(Duration::from_millis(400)).await;
        }

        // 记录开始时间和价格更新
        self.starttime = Some(Local::now().time());
        info!(
            "Update pic send over and price is {}; update start time = {:?} ",
            self.startprice, &self.starttime
        );
        self.startprice += 1; // 价格增加
        Ok(())
    }

    pub async fn run(mut self) {
        let esl_id = self.get_esl_id().unwrap();
        let esl_log_clone = self.ewlog.clone();

        let receive_re = Regex::new(r"category=esl,action=receive,user_code=(.*),eslid=(.*),payload_type=UPDATE,payload_retry_time=").unwrap();
        let release_re = Regex::new(
            r"category=esl,action=esl_update_finished,user_code=(.*),eslid=(.*),status=",
        )
        .unwrap();

        // 循环读取日志，每次记录file seek 位置
        loop {
            let file = File::open(&esl_log_clone).expect("Unable to open file");
            let mut reader = BufReader::new(file);
            reader
                .seek(SeekFrom::Start(self.fileseek))
                .expect("move to seek in file");
            let mut receive_esl = Vec::new();
            let mut release_esl = Vec::new();
            loop {
                let mut line = String::new();
                let bytes_read = reader.read_line(&mut line).unwrap();

                if bytes_read == 0 {
                    break;
                }

                if let Some(captures) = &receive_re.captures(&line) {
                    let esl = captures.get(2).unwrap().as_str().to_string();
                    if esl_id.contains(&esl) && !esl.is_empty() {
                        receive_esl.push(esl);
                    }
                }

                if let Some(captures) = &release_re.captures(&line) {
                    let esl = captures.get(2).unwrap().as_str().to_string();
                    if esl_id.contains(&esl) && !esl.is_empty() {
                        release_esl.push(esl);
                    }

                    if &release_esl.len() == &receive_esl.len() {
                        self.check_is_in(&esl_id, &receive_esl);
                        if Self::is_during(&self.limittime[0], &self.limittime[1])
                            && self.fileseek > 1048576000
                        {
                            let finishtime = Local::now().time();
                            let td = RunTime {
                                st: self.starttime.unwrap(),
                                et: finishtime,
                            }
                            .timediff();
                            let sleeptime = need_sleep_time(&self.limittime);
                            info!(
                                "loop update finish; use second={:?}; sleep {}s pause",
                                td,
                                sleeptime + 30
                            );
                            // spawn(async move {
                            // 输出报告文件
                            //     anyhow_ext::Ok(())
                            // });

                            sleep(Duration::from_secs(sleeptime + 30)).await;
                            self.fileseek = 0; // waiting log change
                            let _ = self.update().await;
                            break;
                        } else {
                            let finishtime = Local::now().time();
                            let td = RunTime {
                                st: self.starttime.unwrap(),
                                et: finishtime,
                            }
                            .timediff();
                            self.fileseek = reader
                                .seek(SeekFrom::End(0))
                                .expect("Update unable to get seek position");
                            info!(
                                "loop update finish; use second={:?}; file seek={}",
                                td, self.fileseek
                            );
                            let _ = self.update().await;
                            break;
                        }
                    }
                }
            }
            if release_esl.len() > 0 {
                info!("recv:={}; finish={};", receive_esl.len(), release_esl.len());
            }
            sleep(Duration::from_secs(5)).await;
        }
    }
}

#[tokio::main]
async fn main() {
    log4rs::init_file("src/log4rs.yaml", Default::default()).unwrap();
    let mut contron = EwConf::new();
    if contron.auto.unwrap() {
        contron.clone().singlerun().await;
    }
    let _ = contron.update().await;
    sleep(Duration::from_secs(70)).await;
    contron.run().await;
}
