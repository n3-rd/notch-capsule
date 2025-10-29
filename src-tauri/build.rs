fn main() {
    // Build Swift framework on macOS
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        println!("cargo:rerun-if-changed=swift/Sources");
        println!("cargo:rerun-if-changed=swift/Package.swift");
        
        // Build Swift package
        let status = Command::new("swift")
            .args(&["build", "-c", "release"])
            .current_dir("swift")
            .status();
            
        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=Swift framework built successfully");
                // Link the Swift framework
                println!("cargo:rustc-link-search=native=swift/.build/release");
                println!("cargo:rustc-link-lib=dylib=NotchCapsuleKit");
            }
            Ok(s) => {
                println!("cargo:warning=Swift build failed with status: {:?}", s);
            }
            Err(e) => {
                println!("cargo:warning=Failed to run swift build: {:?}", e);
            }
        }
    }
    
    tauri_build::build()
}
