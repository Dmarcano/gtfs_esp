use dotenvy_macro::dotenv;

fn main() {
    let ssid = dotenv!("SSID");
    let password = dotenv!("PASSWORD");

    println!("cargo:rustc-env=SSID={ssid}");
    println!("cargo:rustc-env=PASSWORD={password}");

    println!("cargo:rustc-link-arg=-Tlinkall.x");
}
