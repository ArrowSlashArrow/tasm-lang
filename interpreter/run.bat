:: This script is used to test the program in the enviuronment of main (NOT release mode)
cargo build
cd ..
python main.py example_programs/fib_in_memory.tasm --interpret
cd interpreter
:: example_programs/fib_in_memory.tasm is an example file, it can be any .tasm file