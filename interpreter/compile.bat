:: This script is used to automatically move the executable into example_programs
cargo build --release
move /Y ".\target\debug\interpreter.exe" "..\interpreter.exe"
