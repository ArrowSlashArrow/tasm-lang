# profiler script for this program
cargo build --release;
samply record target/release/tasmc.exe ../tests/big_malloc.tasm -e