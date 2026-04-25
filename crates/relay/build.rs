fn main() {
    println!("cargo:rerun-if-changed=../../proto/messages.proto");
    let fds = protox::compile(["../../proto/messages.proto"], ["../../proto/"])
        .expect("protox compile");
    prost_build::Config::new()
        .compile_fds(fds)
        .expect("prost compile");
}
