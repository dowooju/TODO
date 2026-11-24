//! 빌드 스크립트
//! - 프로젝트 루트의 .env 에서 GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET 을 읽어 컴파일 시점 상수로 넣는다.
//!   (.env 는 git 에 올라가지 않는다. 같은 이름의 환경 변수가 있으면 그것이 우선한다.)

fn main() {
    println!("cargo:rerun-if-changed=../.env");
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_SECRET");

    let dotenv = std::fs::read_to_string("../.env").unwrap_or_default();
    for key in ["GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET"] {
        let from_env = std::env::var(key).ok().filter(|v| !v.trim().is_empty());
        let from_file = dotenv.lines().find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        });
        if let Some(value) = from_env.or(from_file).filter(|v| !v.is_empty()) {
            println!("cargo:rustc-env={key}={value}");
        }
    }

    tauri_build::build()
}
