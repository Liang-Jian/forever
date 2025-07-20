use chrono::Local;
use env_logger::Target;
use is_terminal::IsTerminal;
use log::LevelFilter;
use std::io::Write;

struct SimTarget {}

impl Write for SimTarget {
    // 重定向日志到标准输出和错误输出
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        if s.contains(" WARN ") || s.contains(" ERROR ") {
            std::io::stderr().write(buf)
        } else {
            std::io::stdout().write(buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()?;
        std::io::stderr().flush()?;
        Ok(())
    }
}

pub fn log_init_console() {
    // 设置模块日志级别
    let default_filer =
        "debug,paho_mqtt_c=warn,paho_mqtt=warn,hyper=warn,mio-serial=warn,reqwest=warn,ssh=warn,ntp=warn";

    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, default_filer);
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}:{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                //record.module_path().unwrap_or("<unnamed>"),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or_default(),
                &record.args()
            )
        })
        .target(Target::Pipe(Box::new(SimTarget {})))
        .init();
}

cfg_if::cfg_if! {
    if #[cfg(feature = "using-journal")] {
        use log::Log;
        use std::collections::HashMap;
        use systemd_journal_logger::JournalLog;
        use std::env;
    }
}

#[cfg(feature = "using-journal")]
struct EnvJournal<K: AsRef<str>, V: AsRef<str>> {
    log: JournalLog<K, V>,
    filter: HashMap<&'static str, log::Level>,
    default_level: log::Level,
}

#[cfg(feature = "using-journal")]
impl<K: AsRef<str>, V: AsRef<str>> Log for EnvJournal<K, V>
where
    K: AsRef<str> + Send + Sync + 'static,
    V: AsRef<str> + Send + Sync + 'static,
{
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if let Some(m) = record.module_path() {
            let m: Vec<&str> = m.split("::").collect();
            if let Some(m) = m.first() {
                let r_l = record.level();
                if r_l > self.default_level {
                    return;
                }

                if let Some(l) = self.filter.get(m) {
                    if &r_l > l {
                        return;
                    }
                }
            }
        }
        self.log.log(record)
    }

    fn flush(&self) {
        self.log.flush();
    }
}

#[cfg(feature = "using-journal")]
fn log_init_journal() {
    // If the output streams of this process are directly connected to the
    // systemd journal log directly to the journal to preserve structured
    // log entries (e.g. proper multiline messages, metadata fields, etc.)
    let args: Vec<String> = env::args().collect();
    let prog = args.first().unwrap_or(&"unknown".to_string()).to_string();
    let mut prog = std::path::Path::new(&prog).file_name().unwrap_or_default().to_str().unwrap_or_default().to_string();

    // 产测时重命令syslog程序名字
    let second = args.get(1).unwrap_or(&"unknown".to_string()).to_string();
    if second == "product-test" {
        prog = second.to_string();
    }

    let mut filter = HashMap::new();

    // err:1, warn:2, info:3, debug:4
    filter.insert("paho_mqtt_c", log::Level::Warn);
    filter.insert("paho_mqtt", log::Level::Warn);
    filter.insert("hyper", log::Level::Warn);
    filter.insert("mio-serial", log::Level::Warn);
    filter.insert("reqwest", log::Level::Warn);
    filter.insert("ssh", log::Level::Warn);
    filter.insert("ntp", log::Level::Warn);

    let target = EnvJournal {
        log: JournalLog::default().with_syslog_identifier(prog),
        filter,
        default_level: log::Level::Debug,
    };
    log::set_boxed_logger(Box::new(target)).unwrap();
    log::set_max_level(LevelFilter::Debug)
}

use log4rs::{
    self,
    append::rolling_file::policy::compound::{
        roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
    },
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};
const TRIGGER_FILE_SIZE: u64 = 2 * 1024 * 1024;
const ARCHIVE_PATTERN: &str = "logs/slinker.{}.gz";
const FILE_PATH: &str = "logs/slinker.log";
const LOG_FILE_COUNT: u32 = 3;

pub fn log_init_log4rs(level: LevelFilter) {
    let trigger = SizeTrigger::new(TRIGGER_FILE_SIZE);
    let roller = FixedWindowRoller::builder()
        .base(0) // Default Value (line not needed unless you want to change from 0 (only here for demo purposes)
        .build(ARCHIVE_PATTERN, LOG_FILE_COUNT) // Roll based on pattern and max 3 archive files
        .unwrap();
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let logfile = log4rs::append::rolling_file::RollingFileAppender::builder()
        // Pattern: https://docs.rs/log4rs/1.3.0/log4rs/encode/pattern/index.html
        .encoder(Box::new(PatternEncoder::new("{d} {l} {f}:{L} - {m}{n}")))
        .build(FILE_PATH, Box::new(policy))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(level))
        .unwrap();

    let _handle = log4rs::init_config(config).unwrap();
}

pub fn log_init() {
    if std::io::stdout().is_terminal() {
        log_init_console();
    } else {
        #[cfg(feature = "using-journal")]
        log_init_journal();
        #[cfg(not(feature = "using-journal"))]
        log_init_console();
    }
}
