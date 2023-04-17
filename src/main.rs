use clap::{App, Arg, ArgMatches};
use reqwest::header;
use serde_json;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::process;
use std::time::Duration;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::sync::Semaphore;

fn banner() {
    let logo = r###" _                 _        _               _    
| |__   ___   ___ | | _____| |__   ___  ___| | __
| '_ \ / _ \ / _ \| |/ / __| '_ \ / _ \/ __| |/ /
| |_) | (_) | (_) |   < (__| | | |  __/ (__|   < 
|_.__/ \___/ \___/|_|\_\___|_| |_|\___|\___|_|\_\"###;
    println!("{logo}");
}

fn cmdline() -> ArgMatches {
    App::new("Bookcheck")
        .version("0.1.0")
        .author("by 0xchang")
        .about("A book source verification tool written in Rust language")
        .arg(
            Arg::with_name("file")
                .short('f')
                .long("file")
                .value_name("filename")
                .help("Set Book Source File(default shareBookSource.json)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("thread")
                .short('t')
                .long("thread")
                .value_name("thread")
                .help("Set the number of threads(default 15)")
                .takes_value(true),
        )
        .get_matches()
}

async fn req_head(semaphore: &Semaphore, url: String) -> u16 {
    let _permit = semaphore.acquire().await.unwrap();
    println!("checking in {url}");
    let timeout_duration = Duration::from_secs(3);
    let client = reqwest::Client::new();
    let response= client
        .head(&url).timeout(timeout_duration)
        .header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36 Edg/112.0.1722.39")
        .send()
        .await;
    let code = match response {
        Ok(resp) => resp.status().as_u16(),
        Err(_) => 0,
    };
    println!("{url} is checked");
    return code;
}

#[tokio::main]
async fn main() {
    let mut colstdout = StandardStream::stdout(ColorChoice::Always);
    colstdout
        .set_color(ColorSpec::new().set_fg(Some(Color::Blue)))
        .unwrap();
    banner();
    colstdout.reset().unwrap();
    // 定义命令行参数和选项
    let matches = cmdline();

    // 解析命令行参数和选项
    let filename = matches.value_of("file").unwrap_or("shareBookSource.json");
    let threadres = matches.value_of("thread").unwrap_or("15").parse::<usize>();
    let threadnum = match threadres {
        Ok(num) => num,
        Err(_) => {
            colstdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Red)))
                .unwrap();
            println!("Thread parameters must be greater than 0");
            colstdout.reset().unwrap();
            process::exit(1)
        }
    };
    // 打印问候语
    let semaphore = Semaphore::new(threadnum);
    let file = File::open(filename).expect("Fialed Open File");
    let reader = BufReader::new(file);
    let booksources: Vec<Value> = serde_json::from_reader(reader).expect("Failed to Parse JSON");
    let mut tasks = Vec::new();
    //println!("{:#}", booksource);
    for booksource in &booksources {
        if let Value::Object(map) = booksource {
            let bookurl = map.get("bookSourceUrl").unwrap();
            let bookurl: String = bookurl.to_string().replace("\"", "").replace(" ", "");
            if bookurl.starts_with("http") {
                tasks.push(req_head(&semaphore, bookurl));
            }
        }
    }
    let codes = futures::future::join_all(tasks).await;
    print!("{:#?}", codes);
    let mut index = 0;
    let mut out = vec![];
    for code in codes {
        if code >= 200 && code < 300 {
            out.push(booksources.get(index));
            index += 1;
        }
    }
    //println!("{:#?}", out);
    let json_str = serde_json::to_string(&out).unwrap();
    let mut file = File::create("new".to_owned() + filename).unwrap();
    file.write_all(json_str.as_bytes()).unwrap();
}
