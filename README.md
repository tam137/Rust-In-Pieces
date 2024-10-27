### initial build for Ubuntu:  
```
nightly-x86_64-unknown-linux-gnu unchanged - rustc 1.83.0-nightly (eb4e23467 2024-10-09)
```

### for Windows build:  

```
cargo build --target x86_64-pc-windows-gnu --release
```

### for ARM64 Build:  

```
rustup target add aarch64-unknown-linux-gnu2
sudo apt install gcc-aarch64-linux-gnu
```


Create .cargo/config.toml  
```
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

compile std for ARM64:  
```
cargo build -Z build-std=std,panic_abort --target=aarch64-unknown-linux-gnu --release
```

Compile Project for ARM64:  
```
cargo build --target=aarch64-unknown-linux-gnu --release
```