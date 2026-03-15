fn main() {
    // 确保环境变量变化时重新编译（CI 中 Lite/Full 构建依赖此变量）
    println!("cargo:rerun-if-env-changed=OC_BUILD_BUNDLED");
    println!("cargo:rerun-if-env-changed=OC_UPDATE_URL");
    tauri_build::build()
}
