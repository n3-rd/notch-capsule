fn main() {
    // Build Swift framework on macOS
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        use std::env;
        use std::fs;
        use std::path::PathBuf;
        
        println!("cargo:rerun-if-changed=swift/Sources");
        println!("cargo:rerun-if-changed=swift/Package.swift");
        
        // Determine build profile (debug or release)
        let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
        let swift_config = if profile == "release" { "release" } else { "debug" };
        
        // Build Swift package
        let status = Command::new("swift")
            .args(&["build", "-c", swift_config])
            .current_dir("swift")
            .status();
            
        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=Swift framework built successfully");
                
                // Get the target directory
                let out_dir = env::var("OUT_DIR").unwrap();
                let target_dir = PathBuf::from(&out_dir)
                    .ancestors()
                    .nth(3)
                    .unwrap()
                    .to_path_buf();
                
                // Copy the dylib to target directory so runtime linker can find it
                let swift_lib = format!("swift/.build/{}/libNotchCapsuleKit.dylib", swift_config);
                let target_lib = target_dir.join("libNotchCapsuleKit.dylib");
                
                if let Err(e) = fs::copy(&swift_lib, &target_lib) {
                    println!("cargo:warning=Failed to copy Swift dylib: {:?}", e);
                } else {
                    println!("cargo:warning=Copied Swift dylib to target directory");
                }
                
                // Link the Swift framework
                println!("cargo:rustc-link-search=native={}", target_dir.display());
                println!("cargo:rustc-link-lib=dylib=NotchCapsuleKit");
                
                // Set rpath for the binary to find the dylib
                println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
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
