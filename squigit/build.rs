fn main() {
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("icons/icon.ico")
        .set("ProductName", "Squigit")
        .set("FileDescription", "Squigit")
        .compile()
        .expect("Could not embed the Squigit application icon");
}
