use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::process::exit;
use reqwest::blocking::Client;
use serde_json::Value;

const TIMESTAMP_PATTERN: &str = " --> ";
const SEND_BODY_PREFIX: &str = r#"{
    "model": "qwen2.5",
    "stream": false,
    "system": "请你帮助我将英文电影台词翻译成中文。你必须直接输出翻译后的结果，不要输出除了翻译结果之外的其它任何内容。你只需要输出一种可能的翻译即可，不要输出多种可能的翻译结果。以下是需要你翻译的台词：",
    "prompt": ""#;
const SEND_BODY_SUFFIX: &str = r#""}"#;

fn get_translation(client: &Client, text: &str) -> String {
    let body = format!("{SEND_BODY_PREFIX}{}{SEND_BODY_SUFFIX}", text.replace("\\", "\\\\"));
    let resp = client.post("http://localhost:11434/api/generate")
        .body(body)
        .send()
        .expect("That request should be sent");
    let deserialized: Value = serde_json::from_str(&resp.text().unwrap()).unwrap();
    let Value::String(translated) = &deserialized["response"] else {
        eprintln!("Resp: {deserialized:#?}");
        panic!("no response");
    };
    translated.lines().filter(|line| !line.is_empty()).collect::<String>()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];

    let mut file = File::open(filename).expect(&format!("Failed to open file: {filename}"));
    println!("File opened: {filename}");

    let mut translated = File::create(&format!("{filename}.out")).unwrap();

    let mut original_subtitles = String::new();
    file.read_to_string(&mut original_subtitles).expect("Failed to read subtitles to string");
    let line_count = original_subtitles.lines().filter(|line| { line.contains(TIMESTAMP_PATTERN) }).count();
    println!("{line_count} line(s) of subtitles to be processed");

    let mut lines = original_subtitles.lines();
    let client = Client::new();
    for i in 1..=line_count {
        // line number
        let Some(line) = lines.next() else { panic!("Line {i} is not found"); };
        writeln!(translated, "{line}").unwrap();

        // timestamp
        let Some(line) = lines.next() else { panic!("Timestamp of line {i} is not found"); };
        if !line.contains(TIMESTAMP_PATTERN) { panic!("Timestamp of line {i} has wrong format"); };
        writeln!(translated, "{line}").unwrap();

        // subtitle 0, 1, 2, ... or empty
        let mut subtitle = String::new();
        loop {
            let Some(line) = lines.next() else { println!("Finished early at line {i}"); exit(0); };
            if line.is_empty() { break; }
            subtitle.push(' ');
            subtitle.push_str(line);
        }
        println!("{i}\t{subtitle}");
        let translation = get_translation(&client, &subtitle);
        println!("{i}\t{}", translation);
        println!("");
        writeln!(translated, "{translation}\n").unwrap();
    }
}
