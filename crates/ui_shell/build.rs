fn main() {
    glib_build_tools::compile_resources(
        &[".", "../.."],
        "resources.gresource.xml",
        "phototux-ui.gresource",
    );
}
