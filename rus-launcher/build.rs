fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource.set("ProductName", "Rus Launcher");
        resource.set("FileDescription", "Rus Launcher");
        resource.set("CompanyName", "Obsifox Studio");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=could not add the icon: {error}");
        }
    }
}
