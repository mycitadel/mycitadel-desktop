fn main() {
    #[cfg(windows)]
    embed_resource::compile("mycitadel.rc", embed_resource::NONE);
}
