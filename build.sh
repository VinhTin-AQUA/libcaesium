# export JAVA_HOME=/home/newtun/.local/apps/android-studio/jbr
# export ANDROID_HOME=$HOME/Android/Sdk
# export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/27.3.13750724

# export NDK_TOOLCHAIN=$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin

# export CC_aarch64_linux_android=$NDK_TOOLCHAIN/aarch64-linux-android21-clang
# export CXX_aarch64_linux_android=$NDK_TOOLCHAIN/aarch64-linux-android21-clang++
# export AR_aarch64_linux_android=$NDK_TOOLCHAIN/llvm-ar
# export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$NDK_TOOLCHAIN/aarch64-linux-android21-clang

# rustup target add aarch64-linux-android

cargo clean

cargo build --release --target x86_64-pc-windows-gnu
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-linux-android

# target/aarch64-linux-android/release/libcaesium.so
# target/x86_64-pc-windows-gnu/release/caesium.dll
# target/x86_64-unknown-linux-gnu/release/libcaesium.so
