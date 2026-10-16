//! 导出 OpenAPI JSON 快照（frontend/openapi.json 离线生成源）。
//!
//! 用法：`cargo run --bin export-openapi > ../frontend/openapi.json`

fn main() {
    let json = flame_kernel::openapi::openapi_json();
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
