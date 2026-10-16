//! `fpctl` —— Flamepanel 命令行。设计见 Doc/32 §3，实现见 `flame_kernel::cli`。

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(flame_kernel::cli::run(argv));
}
