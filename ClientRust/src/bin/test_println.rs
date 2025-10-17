// 极简测试 - 只测试 println! 是否工作
fn main() {
    println!("=== 测试开始 ===");
    println!("如果你能看到这条消息,说明 println 工作正常");
    
    // 暂停等待用户输入
    println!("\n按 Enter 键退出...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
}
