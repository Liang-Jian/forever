use anyhow_ext::{anyhow, bail, Result};
use log::{debug, warn};
use std::time::Duration;
use std::{fmt::Debug, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    select,
    sync::{mpsc::error::TryRecvError, oneshot, Mutex},
    time::{self, sleep},
};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    time::timeout,
};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

pub const UART_INVALID_CHAR: &str = ">";

enum UartCmd {
    NoWait(String, oneshot::Sender<String>),
    Wait(String, String, oneshot::Sender<String>),
}

pub struct UartServer {
    device: String,
    speed: u32,
    timeout_ms: u64,
    filters: Vec<String>,
    cmd_snd: Sender<UartCmd>,
    cmd_rcv: Receiver<UartCmd>,

    //wait cmd
    wait_mode: bool,
    wait: String,
    write_data: String,
    tmp_chn: Option<oneshot::Sender<String>>,
    wait_chn: Option<oneshot::Sender<String>>,

    // 如果是tty，则要忽略回显的数据
    is_tty: bool,
    line_buf: String,
    echo_data: String,

    lock: Arc<Mutex<u8>>,
}

#[derive(Clone)]
pub struct UartClient {
    cmd_snd: Sender<UartCmd>,
    lock: Arc<Mutex<u8>>,
}

impl Debug for UartClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UartClient").finish()
    }
}

impl UartClient {
    pub async fn new_test() -> Self {
        let (ctx, _crx) = channel(1000);
        UartClient { lock: Arc::new(Mutex::new(1)), cmd_snd: ctx }
    }

    pub async fn cmd(&mut self, cmd: &str, tim: Duration) -> Result<String> {
        // 获取锁
        let _lock = self.lock.lock().await;
        let c1 = cmd;

        for i in 0..3 {
            let (ret_snd, ret_rcv) = oneshot::channel();
            let cmd = UartCmd::NoWait(cmd.to_string(), ret_snd);
            self.cmd_snd.send(cmd).await?;
            match timeout(tim, ret_rcv).await {
                Ok(Ok(v)) => {
                    return Ok(v);
                }
                Ok(Err(e)) => {
                    bail!("run cmd {c1} faild, {e}");
                }
                _ => {
                    warn!("run cmd timeout, sleep {} sec", i + 1);
                    sleep(Duration::from_secs(i + 1)).await
                }
            }
        }

        Err(anyhow!("cmd timeout"))
    }

    pub async fn wait_cmd(&mut self, wait: &str, cmd: &str, tim: Duration) -> Result<String> {
        // 获取锁
        let _lock = self.lock.lock().await;

        for i in 0..3 {
            let (ret_snd, ret_rcv) = oneshot::channel();
            let cmd = UartCmd::Wait(wait.to_string(), cmd.to_string(), ret_snd);
            self.cmd_snd.send(cmd).await?;
            match timeout(tim, ret_rcv).await {
                Ok(Ok(v)) => return Ok(v),
                Ok(Err(e)) => {
                    bail!("run cmd faild, {e}");
                }
                _ => {
                    warn!("run cmd timeout, sleep {} sec", i + 1);
                    sleep(Duration::from_secs(i + 1)).await
                }
            }
        }

        Err(anyhow!("cmd timeout"))
    }
}

impl UartServer {
    pub fn new(device: &str, speed: u32, timeout_ms: u64, filters: Vec<String>) -> Self {
        let (cmd_snd, cmd_rcv) = channel(100);
        Self {
            device: device.to_string(),
            speed,
            timeout_ms,
            cmd_snd,
            cmd_rcv,
            filters,
            wait_mode: false,
            wait: "".to_string(),
            write_data: "".to_string(),
            tmp_chn: None,
            wait_chn: None,
            is_tty: false,
            line_buf: "".to_string(),
            echo_data: "".to_string(),
            lock: Arc::new(Mutex::new(1)),
        }
    }

    pub fn set_tty(mut self, value: bool) -> Self {
        self.is_tty = value;
        self
    }

    pub fn client(&self) -> UartClient {
        UartClient { cmd_snd: self.cmd_snd.clone(), lock: self.lock.clone() }
    }

    fn filter_lines(filters: &Vec<&str>, line_buf: &str) -> Result<Option<String>> {
        // 如果filter里有">"
        if filters.contains(&UART_INVALID_CHAR) {
            for s in line_buf.lines() {
                if s.starts_with(UART_INVALID_CHAR) {
                    bail!("detect invalid char {UART_INVALID_CHAR}, send ctrl+d");
                }
            }
        }

        let s: Vec<&str> = line_buf
            .lines()
            .filter(|x| {
                if x.is_empty() {
                    return false;
                }
                for f1 in filters {
                    if x.starts_with(f1) {
                        return false;
                    }
                }
                true
            })
            .map(|x| x.trim())
            .collect();
        if !s.is_empty() {
            Ok(Some(s.join("\n")))
        } else {
            Ok(None)
        }
    }

    async fn flush_buf(&mut self, port: &mut SerialStream) -> Result<()> {
        let mut v: Vec<&str> = Vec::new();
        for k in &self.filters {
            v.push(k);
        }
        // 如果串口是tty，则过滤掉回显的数据
        if self.is_tty && !self.write_data.is_empty() {
            self.echo_data.clone_from(&self.write_data);
            self.echo_data = self.echo_data.trim().to_string();
            if !self.echo_data.is_empty() {
                v.push(&self.echo_data);
            }
        }

        match Self::filter_lines(&v, &self.line_buf) {
            Ok(Some(s)) => {
                if s.is_ascii() {
                    print!("{s}");
                }

                // 只有等到了wait字符串，才能写入data，并返回数据
                if self.wait_mode && s.contains(&self.wait) {
                    debug!("write uart: {}", self.write_data);
                    port.write_all(self.write_data.as_bytes()).await?;
                    self.tmp_chn = self.wait_chn.take();
                    self.wait_mode = false;
                } else if let Some(q) = self.tmp_chn.take() {
                    if let Err(e) = q.send(s) {
                        warn!("send uart reply error, {e}");
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                warn!("{e}");
                port.write_all(&[0x04]).await.unwrap_or_default();
                port.flush().await.unwrap_or_default();
                self.line_buf = String::new();
            }
        }

        self.line_buf = String::new();

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut port = tokio_serial::new(&self.device, self.speed).open_native_async()?;

        loop {
            // 收到命令后，更新返回结果的chn地址
            match self.cmd_rcv.try_recv() {
                Ok(UartCmd::NoWait(data, chn)) => {
                    self.write_data.clone_from(&data);
                    //debug!("write to uart: {}", self.write_data);
                    port.write_all(data.as_bytes()).await?;
                    self.tmp_chn = Some(chn);
                    self.wait_mode = false;
                }
                Ok(UartCmd::Wait(wait, data, chn)) => {
                    self.wait_mode = true;
                    self.wait = wait;
                    self.write_data = data;
                    self.wait_chn = Some(chn);
                }
                Err(TryRecvError::Empty) => {}
                Err(x) => return Err(anyhow!(x)),
            }

            let mut b1 = [0; 1024];
            let timeout = time::sleep(Duration::from_millis(self.timeout_ms));

            select! {
                v = port.read(&mut b1[..]) => match v {
                    Ok(0) => {
                        warn!("read to eof of uart");
                        break;
                    }
                    Ok(n) => {
                        let s: String = String::from_utf8_lossy(&b1[..n]).to_string();
                        if s.is_ascii() {
                            print!("{s}");
                        }

                        self.line_buf.push_str(s.as_str());
                        if self.line_buf.len() > 1024*1024 { // 如果内容超过1M，强制刷出
                            self.flush_buf(&mut port).await?;
                        }
                    }
                    Err(x) => {
                        warn!("read error {x} of uart");
                        break;
                    }
                },
                _ = timeout => {
                    if !self.line_buf.is_empty() {
                        self.flush_buf(&mut port).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::UartServer;

    #[test]
    fn test_filter_lines() {
        let ret = UartServer::filter_lines(&vec![], &"".to_string()).unwrap();
        assert_eq!(ret, None);
        let ret = UartServer::filter_lines(&vec![], &"\n".to_string()).unwrap();
        assert_eq!(ret, None);
        let ret = UartServer::filter_lines(&vec![], &"\r\nOpenWrt login:".to_string()).unwrap();
        assert_eq!(ret, Some("OpenWrt login:".to_string()));
    }
}
