use std::path::PathBuf;

fn main() {
  let src = PathBuf::from("src");
  let includes = &[src.clone()];

  let mut config = prost_build::Config::new();
  config.btree_map(["."]);

  config.compile_protos(&[src.join("discord.proto")], includes).unwrap();
}
