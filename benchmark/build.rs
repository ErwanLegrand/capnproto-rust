fn main() {
    ::capnpc::CompilerCommand::new()
        .file("eval.capnp")
        .file("catrank.capnp")
        .file("carsales.capnp")
        .import_path("../capnp")
        .src_prefix("../capnp")
        .run()
        .expect("compiling schemas");
}
