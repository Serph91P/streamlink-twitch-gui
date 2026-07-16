use std::{env, process, thread, time::Duration};

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.iter().any(|argument| argument == "--json") {
        println!(
            r#"{{"plugin":"twitch","streams":{{"1440p60 (av1)":{{"type":"hls","url":"https://fixture.invalid/av1.m3u8"}},"best":{{"type":"hls","url":"https://fixture.invalid/av1.m3u8"}}}}}}"#
        );
        return;
    }
    for argument in &arguments {
        println!("argument={argument}");
    }
    eprintln!("fake streamlink diagnostic");

    if arguments.iter().any(|argument| argument.contains("/wait")) {
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
    if arguments.iter().any(|argument| argument.contains("/fail")) {
        eprintln!("Authorization: Bearer raw-output-secret");
        eprintln!(
            "resolved=https://cdn.example.test/playlist.m3u8?token=raw-query-secret&sig=signed"
        );
        eprintln!("standalone-token=standalone-secret");
        process::exit(7);
    }
}
