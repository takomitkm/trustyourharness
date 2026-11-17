//! 测试 stub：模拟 deja CLI（version / search 子命令），供降级与解析测试。

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(command) = args.get(1) else {
        println!("deja-stub: no command");
        return;
    };
    match command.as_str() {
        "version" => println!("deja-stub 0.1.0"),
        "search" => {
            let query = args.get(2).cloned().unwrap_or_default();
            // 罐头 JSON（结构对齐真实 deja 输出）：两条命中，query 回显。
            println!(
                "{{\"query\":\"{query}\",\"hits\":[{{\"source\":\"stub-agent\",\"title\":\"命中一 {query}\"}},{{\"source\":\"stub-agent\",\"title\":\"命中二\"}}]}}"
            );
        }
        _ => println!("deja-stub: unknown command {command}"),
    }
}
