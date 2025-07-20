use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    net::Ipv4Addr,
    time::Duration,
};

use anyhow_ext::{bail, Result};

use product::uart::{UartServer, UartClient};
use product::logfile::log_init_console;

use log::{debug, info, warn};
use packed_struct::{derive::PackedStruct, PackedStruct, PackedStructSlice};
use structopt::StructOpt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    spawn,
    time::sleep,
};

#[derive(StructOpt, Clone)]
#[structopt(name = "product-server", about = "aoa product server and usage.")]
pub struct Opt {
    /// 监听IP
    #[structopt(long, default_value = "127.0.0.1")]
    pub ip: String,

    /// 网络端口
    #[structopt(long, default_value = "12345")]
    pub port: u16,

    /// 传感器工装串口
    #[structopt(long, default_value = "/dev/ttyLP3")]
    pub uart_sensor: String,

    /// 天线测试设备串口
    #[structopt(long, default_value = "/dev/ttyLP4")]
    pub uart_ant: String,

    /// 天线测试信道
    #[structopt(long, default_value = "38")] // 37 38 39
    pub ant_chn: u8,

    /// 八方位测试超时时间，单位为秒
    #[structopt(long, default_value = "60")]
    pub eight_timeout: u8,
}

#[derive(Debug, PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct LoginPkg {
    preamble: u8,
    sid: u8,
    length: u16,
    cmd: u16,
    mac: [u8; 6],
    sn: [u8; 32],
    crc: u32,
}

struct ProServer {
    port: u16,
    sensor_uc: UartClient,
    ant_uc: UartClient,
    ant_chn: u8,
    eight_timeout: u8,
    ip: [u8; 4],
}

impl ProServer {
    async fn listen_udp(ip: [u8; 4], port: u16) -> Result<()> {
        #[derive(Debug, PackedStruct)]
        #[packed_struct(endian = "lsb")]
        pub struct BcastPkg {
            pub head: u16,
            pub mac: [u8; 6],
            pub ver: [u8; 32],
            pub ip: [u8; 4],
        }

        #[derive(Debug, PackedStruct)]
        #[packed_struct(endian = "lsb")]
        pub struct BcastAckPkg {
            pub head: u16,
            pub ip: [u8; 4],
            pub port: u16,
            pub flag: u32,
        }

        let sock = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
        loop {
            let mut buf = Vec::with_capacity(128);
            let (_n, peer) = sock.recv_buf_from(&mut buf).await?;
            let pkg = BcastPkg::unpack_from_slice(&buf[0..44])?;
            let ack = BcastAckPkg { head: 0xfffe, ip, port, flag: 0x44332211 };
            let ack = ack.pack_to_vec()?;
            sock.send_to(&ack, peer).await?;

            info!("recv client: {pkg:?}");
        }
    }

    async fn wait_login(port: u16) -> Result<(TcpStream, LoginPkg)> {
        let sock = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        let (mut sock, _addr) = sock.accept().await?;

        let mut buf = [0u8; 48];
        sock.read_exact(&mut buf[..]).await?;
        let pkg = LoginPkg::unpack(&buf)?;

        Self::send_ack(&mut sock).await?;

        info!("recv client: {pkg:?}");

        Ok((sock, pkg))
    }

    async fn recv_ack(sock: &mut TcpStream) -> Result<()> {
        let mut buf = [0; 9];
        sock.read_exact(&mut buf).await?;
        Ok(())
    }

    async fn send_ack(sock: &mut TcpStream) -> Result<()> {
        let hello = [0xFE, 0x01, 0x05, 0x00, 0x06, 0x65, 0xE6, 0xCD, 0x99];
        sock.write_all(&hello).await?;
        Ok(())
    }

    async fn test_ant(sock: &mut TcpStream, uc: &mut UartClient, chn: u8, ant: u8) -> Result<[u8; 10]> {
        uc.cmd(&format!("4001,{ant},{chn},10000\n"), Duration::from_secs(3)).await?;

        #[derive(Debug, PackedStruct, Default)]
        #[packed_struct(endian = "lsb")]
        pub struct AoaAntPkg {
            preamble: u8,
            sid: u8,
            length: u16,
            cmd: u16,
            chn: u8,
            ant: u8,
            crc: u32,
        }
        let pkg = AoaAntPkg { length: 8, cmd: 0x2070, chn, ant, ..Default::default() };
        let pkg = pkg.pack_to_vec()?;
        sock.write_all(&pkg).await?;
        Self::recv_ack(sock).await?;

        #[derive(Debug, PackedStruct, Default)]
        #[packed_struct(endian = "lsb")]
        pub struct AoaAntAckPkg {
            preamble: u8,
            sid: u8,
            length: u16,
            cmd: u16,
            status: u16,
            rssi: [u8; 10],
            crc: u32,
        }
        let mut buf = [0u8; 22];
        sock.read_exact(&mut buf[..]).await?;
        let pkg = AoaAntAckPkg::unpack(&buf)?;
        Self::send_ack(sock).await?;

        if pkg.status != 0x0040 {
            bail!("recv ant rssi failed, {}", pkg.status);
        }

        Ok(pkg.rssi)
    }

    // 测试传感器
    async fn test_sensor(sock: &mut TcpStream, uc: &mut UartClient, pos: u8) -> Result<String> {
        let cmd = match pos {
            0 => "senx",
            1 => "seny",
            _ => "senz",
        };
        uc.cmd(&format!("3001,{cmd}\n"), Duration::from_secs(5)).await?; // 3001发给工装，需要5s,小于3s容易报错

        #[derive(Debug, PackedStruct, Default)]
        #[packed_struct(endian = "lsb")]
        pub struct AoaSnrPkg {
            preamble: u8,
            sid: u8,
            length: u16,
            cmd: u16,
            pos: u8,
            crc: u32,
        }
        let pkg = AoaSnrPkg { length: 7, cmd: 0x2072, pos, ..Default::default() };
        let pkg = pkg.pack_to_vec()?;
        sock.write_all(&pkg).await?;
        Self::recv_ack(sock).await?;

        #[derive(Debug, PackedStruct, Default)]
        #[packed_struct(endian = "lsb")]
        pub struct AoaSnrAckPkg {
            preamble: u8,
            sid: u8,
            length: u16,
            cmd: u16,
            status: u16,
            val: [i32; 3],
            crc: u32,
        }
        let mut buf = [0u8; 24];
        sock.read_exact(&mut buf[..]).await?;
        let pkg = AoaSnrAckPkg::unpack(&buf)?;
        Self::send_ack(sock).await?;

        if pkg.status != 0x0040 {
            bail!("recv ant rssi failed, {}", pkg.status);
        }

        Ok(format!("{}={:?}", cmd, pkg.val))
    }

    // 八方位测试
    async fn test_eight(sock: &mut TcpStream, chn: u8, time: u8) -> Result<String> {
        #[derive(Debug, PackedStruct)]
        #[packed_struct(endian = "lsb")]
        pub struct AoaEightPkg {
            preamble: u8,
            sn: u8,
            length: u16,
            cmd: u16,
            eslid: [u8; 36],
            time: u8,
            chn: u8,
            crc: u32,
        }
        let esl_id = Self::get_eslid_only("info.txt");
        warn!("esl_id= {:?}", esl_id);
        let mut esl_id_bytes = [0u8; 36]; // 36 是九个价签乘以4
        for (i, &id) in esl_id.iter().enumerate() {
            let bytes = id.to_be_bytes();
            let start = i * 4;
            esl_id_bytes[start..start + 4].copy_from_slice(&bytes);
        }

        let pkg =
            AoaEightPkg { length: 44, cmd: 0x2074, preamble: 0xFE, sn: 0, crc: 0, chn, time, eslid: esl_id_bytes };
        let pkg = pkg.pack_to_vec()?;
        sock.write_all(&pkg).await?;
        Self::recv_ack(sock).await?;

        #[derive(Debug, PackedStruct, Default)]
        #[packed_struct(endian = "lsb")]
        pub struct AoaEightAckPkg {
            preamble: u8,
            sid: u8,
            length: u16,
            cmd: u16,
            // para_crc: u8,
        }

        // 先收cmd（包含cmd）的数据
        let mut buf = [0u8; 6];
        sock.read_exact(&mut buf[..]).await?;
        let pkg = AoaEightAckPkg::unpack(&buf)?;
        // value length
        let value_len = pkg.length as usize - 2 - 4;
        // 得到length后减去一首cmd的长度就是 value+crc的长度
        let mut buf1 = vec![0u8; (pkg.length as usize) - 2];
        // 收齐数据
        sock.read_exact(&mut buf1[..]).await?;

        // 转为json, 计算方位角、俯仰角
        let recv_all_data = String::from_utf8_lossy(&buf1[std::mem::size_of_val(&pkg.cmd)..value_len]).to_string();
        let parsed_json: HashMap<String, Vec<(u32, u32)>> = serde_json::from_str(&recv_all_data)?;
        // 必须回ACK，先回ack,在处理数据
        Self::send_ack(sock).await?;
        let result_data = Self::eight_get_data_rst(&parsed_json);

        // let file_json: HashMap<String, [f32; 2]> =
        //     serde_json::from_str(&read_to_string(r"D:\loopupgrade\src\esl.txt")?)?;

        Ok(result_data)
    }

    /// 从文件中取去ESLID下发给产测程序
    pub fn get_eslid_only(fp: &str) -> Vec<u32> {
        let mut esl_list = Vec::new();
        let file_contents = read_to_string(fp).expect("txt file read error, please check file");
        let json_value: HashMap<String, Vec<(u32, u32)>> =
            serde_json::from_str(&file_contents).expect("txt to json fail, please check file");
        for key in json_value.keys() {
            let cleaned_key = key.to_uppercase().trim().replace("-", "");
            if let Ok(parsed) = u32::from_str_radix(&cleaned_key, 16) {
                esl_list.push(parsed);
            }
        }
        esl_list
    }

    // 转为16进制MAC
    fn mac2hex(bytes: [u8; 6]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(":")
    }

    /// 输出9个ESL的结果信息 {FF-FF-FF-FF:[aze,ble]}，
    pub fn eight_get_data_rst(s: &HashMap<String, Vec<(u32, u32)>>) -> String {
        let mut result_map: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
        for (key, value) in s {
            // 去掉方括号并拆分数字
            let clean_key = key.trim_matches(|c| c == '[' || c == ']' || c == ' ');
            let bytes: Vec<u8> = clean_key.split(',').map(|s| s.trim().parse().unwrap()).collect();

            // 转为16进制字符串
            let hex_key = bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join("-");
            result_map.insert(hex_key, value.to_owned());
        }

        // 9个ESL的原始数据
        let nine_src_data: HashMap<String, Vec<(u32, u32)>> = Self::eight_get_txt_src_data(result_map);
        let nine_exp_data: HashMap<String, [f32; 2]> =
            serde_json::from_str(&read_to_string("esl.txt").unwrap_or_default()).unwrap();

        let mut all_esl = Vec::new();

        for (idx, (esl, val)) in nine_src_data.clone().iter().enumerate() {
            let s = format!(
                "esl={},exp_azi={:?},data={:?},packnum={}",
                &esl,
                nine_exp_data[esl][0],
                Self::eight_calc_maxi(idx, &val),
                Self::eight_calc_pack(&val)
            );
            warn!("{}\n", s);
            all_esl.push(s);
        }
        let eight_result = all_esl.join(";");
        eight_result
        // nine_src_data
    }

    /// 计算总包数
    fn eight_calc_pack(src: &Vec<(u32, u32)>) -> usize {
        src.len()
    }

    /// 只取9个指定ESL实际受到的数据
    fn eight_get_txt_src_data(src: HashMap<String, Vec<(u32, u32)>>) -> HashMap<String, Vec<(u32, u32)>> {
        let mut src_data: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
        let file_contents = read_to_string("esl.txt").expect("txt file read error, please check file");
        let data: HashMap<String, Vec<(u32, u32)>> =
            serde_json::from_str(&file_contents).expect("failed to parse json");
        let all_key: HashSet<&String> = data.keys().collect();
        for (key, val) in src {
            if all_key.contains(&key) {
                src_data.insert(key.to_owned(), val.to_owned());
            }
        }
        src_data
    }

    /// 计算方差
    fn eight_calculate_variance(data: &[f64]) -> Option<f64> {
        let n = data.len() as f64;
        if n <= 1.0 {
            return None; // 要求数据点大等于1
        }
        // Step 1: 计算均值
        let mean = data.iter().sum::<f64>() / n;
        // Step 2: 计算每个数据点与均值的平方差
        let sum_of_squares = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>();
        // Step 3: 计算方差
        let variance = sum_of_squares / (n - 1.0); // 使用无偏估计，分母为(n - 1)
        Some(variance)
    }

    /// 返回ESL最大、小、平均、方差 [aze_max,aze_min,aze_avg,aze_var]
    fn eight_calc_maxi(idx: usize, src: &Vec<(u32, u32)>) -> String {
        let mut x_vec: Vec<f64> = Vec::new();
        let mut y_vec: Vec<f64> = Vec::new();

        for i in src {
            x_vec.push(i.0 as f64 / 1000 as f64);
            y_vec.push(i.1 as f64 / 1000 as f64);
        }
        debug!("azi_vec={:?}", x_vec);
        debug!("ele_vec={:?}", y_vec);

        let azi_max: f64 = x_vec.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let azi_min: f64 = x_vec.iter().cloned().fold(f64::INFINITY, f64::min);
        // 0度单独求平均
        let azi_avg = if idx == 0 { Self::calc_avg_index0(x_vec.clone()) } else { Self::calculate_mean(x_vec.clone()) };
        let azi_variance = Self::eight_calculate_variance(&x_vec).unwrap_or_default();
        let ele_max = y_vec.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let ele_min = y_vec.iter().cloned().fold(f64::INFINITY, f64::min);

        let ele_avg = if idx == 0 { Self::calc_avg_index0(y_vec.clone()) } else { Self::calculate_mean(y_vec.clone()) };
        let ele_variance = Self::eight_calculate_variance(&y_vec).unwrap_or_default();
        let s = format!(
            "azi_max={},aze_min={},aze_avg={},aze_variance={},ele_max={},ele_min={},ele_avg={},ele_variance={}",
            azi_max, azi_min, azi_avg, azi_variance, ele_max, ele_min, ele_avg, ele_variance
        );
        // [
        //     azi_max,
        //     azi_min,
        //     azi_avg,
        //     azi_variance,
        //     ele_max,
        //     ele_min,
        //     ele_avg,
        //     ele_variance

        // ]
        s
    }

    /// 求平均值
    fn calculate_mean(data: Vec<f64>) -> f64 {
        let sum: f64 = data.iter().sum();
        let average = sum / (data.len() as f64);
        (average * 100.0).round() / 100.0
    }

    /// 求index 为0的ESL的平均值，大于180的减去360,其他的不变
    fn calc_avg_index0(src_vec: Vec<f64>) -> f64 {
        let limit_val = 180.00;
        let new_vec_list: Vec<f64> =
            src_vec.iter().map(|&d| if d > limit_val { d - (limit_val * 2.00) } else { d }).collect();
        let sum: f64 = new_vec_list.iter().sum();
        let average = sum / (new_vec_list.len() as f64);
        (average * 100.0).round() / 100.0
    }

    /// 单个测试
    async fn handle_test(&mut self, sock: &mut TcpStream, mac: [u8; 6]) -> Result<()> {
        // 测试天线, 4和12 跳过
        let ap = Self::mac2hex(mac);

        debug!("test ant");
        let mut rssi_result = Vec::new();
        for i in 1..19 {
            if i != 4 && i != 12 {
                let v = Self::test_ant(sock, &mut self.ant_uc, self.ant_chn, i).await?;
                // info!("ap: {ap}, ant: {i}, rssi:{v:?}");
                rssi_result.push(format!("{}={:?}", i, v));
            }
        }
        info!("ap: {ap}, ant: {}", rssi_result.join(";"));

        // 测试Sensor
        debug!("test sensor");
        let mut sensor_result = Vec::new();
        self.sensor_uc.cmd(&format!("3001,grab\n"), Duration::from_secs(3)).await?; // 固定基站，夹住
        sleep(Duration::from_secs(3)).await;

        for i in 0..3 {
            if let Ok(v) = Self::test_sensor(sock, &mut self.sensor_uc, i).await {
                // info!("ap: {ap:?}, sensor: {i}, value:{v:?}");
                sensor_result.push(v);
            }
        }
        info!("ap: {ap}, sensor: {}", sensor_result.join(";"));
        sleep(Duration::from_secs(5)).await;
        self.sensor_uc.cmd(&format!("3001,release\n"), Duration::from_secs(3)).await?; // 松开基站

        // 测试八方位
        debug!("test eight");
        let v = Self::test_eight(sock, self.ant_chn, self.eight_timeout).await?;
        info!("ap: {ap}, eight: {v}");

        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        let port = self.port;
        let ip = self.ip;
        spawn(async move {
            loop {
                if let Err(e) = Self::listen_udp(ip, port).await {
                    warn!("listen udp failed, {e}");
                    sleep(Duration::from_secs(5)).await;
                }
            }
        });

        loop {
            match Self::wait_login(self.port).await {
                Ok((mut sock, pkg)) => {
                    if let Err(e) = self.handle_test(&mut sock, pkg.mac).await {
                        warn!("handle test failed, {e}");
                    }
                }
                Err(e) => warn!("wait login failed, {e}"),
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    log_init_console();
    info!("product test start");

    let opt = Opt::from_args();
    let ip = opt.ip.parse::<Ipv4Addr>()?.octets();

    let mut sensor_srv = UartServer::new(&opt.uart_sensor, 115200, 1000, vec!["log,".into()]);
    let mut ant_srv = UartServer::new(&opt.uart_ant, 115200, 1000, vec!["log,".into()]);
    let sensor_uc = sensor_srv.client();
    let ant_uc = ant_srv.client();

    spawn(async move {
        sensor_srv.run().await?;
        anyhow_ext::Ok(())
    });

    spawn(async move {
        ant_srv.run().await?;
        anyhow_ext::Ok(())
    });

    let mut srv =
        ProServer { sensor_uc, ant_uc, ant_chn: opt.ant_chn, eight_timeout: opt.eight_timeout, ip, port: opt.port };
    srv.run().await?;

    Ok(())
}
