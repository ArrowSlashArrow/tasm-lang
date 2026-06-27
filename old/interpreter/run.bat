:: This script is used to test the program in the enviuronment of main (NOT release mode)
cargo build
if "%1" == "" goto err

:run
cd ..
python main.py interpreter\%1 --interpret --runner
cd interpreter
goto end

:err
echo Supply a file to run after .\run.bat: .\run.bat <file>
goto end

:end