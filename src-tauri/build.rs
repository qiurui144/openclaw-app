fn main() {
    // 确保环境变量变化时重新编译（CI 中 Lite/Full 和多客户构建依赖这些变量）
    println!("cargo:rerun-if-env-changed=OC_BUILD_BUNDLED");
    println!("cargo:rerun-if-env-changed=OC_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=OC_LICENSE_API");
    println!("cargo:rerun-if-env-changed=OC_UPDATE_URL");
    tauri_build::build()
}
