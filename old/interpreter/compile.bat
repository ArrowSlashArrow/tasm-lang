:: This script is used to automatically move the executable into example_programs
cargo build --release
move /Y ".\target\release\interpreter.exe" "..\interpreter.exe"
