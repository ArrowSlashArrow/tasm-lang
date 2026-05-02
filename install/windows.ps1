param (
    [Parameter(Position = 0)]
    [string]$action,

    [Parameter(Position = 1)]
    [string]$version

)

$appname = "tasm";
$tasmdir = Join-Path $env:APPDATA $appname; 

$help = @"
This is the install script for the TASM language.
One of these actions must be specified as the first argument:
- "remove": to remove the current installed version of tasm if it exists
- "install <version>": install the specified version. overwrites (with permission) any other version 
"@

$installprompt = @"
Which version would you like to install? 
[0] Using MSVC toolchain (requires MSVC)
[1] Using GNU toolchiain 
"@

function install ($version) {
    # check if there is an installed version already
    $old = [System.Environment]::GetEnvironmentVariable("PATH", "User") -split ";"
    if ($tasmdir -in $old) {
        Write-Host "Detected an already installed version of tasm" -ForegroundColor Yellow
        try {
            # try to print the installed version
            & (Join-Path $tasmdir "tasmc.exe") --version
        } 
        catch {
            # do nothing
            # since if the program failed the user shouldn't learn of our mistakes
        }

        # ask user for permission to remove old version
        $confirm = Read-Host "> Confirm removing this version to install $appname $version (y/n)"
        if ($confirm -ne "y") {
            return;
        }

        # then, remove it
        remove;
    }

    if (-not $version) {
        Write-Host "Specify a version: .\windows.ps1 install <version>";
        return;
    }

    Write-Host $installprompt
    $choice = Read-Host "> Option number"
    switch ($choice) {
        "0" {
            $triple = "x86_64-pc-windows-msvc"
        }
        "1" {
            $triple = "x86_64-pc-windows-gnu"
        }
        default {
            Write-Host "Invalid option. Aborting..."
            return
        }
    }
    # ok now install the new version
    $url = "https://github.com/ArrowSlashArrow/tasm-lang/releases/download/$version/tasm-$version-$triple.zip"
    $outdir = Join-Path $env:TEMP "tasm-$version"
    $outfile_zip = Join-Path $env:TEMP "tasm-$version.zip"
    try {
        Invoke-WebRequest -Uri $url -OutFile $outfile_zip
    }
    catch {
        Write-Host "Unable to download file.";
        return;
    }

    # remove stale temp dir
    if (Test-Path $outdir) {
        Remove-Item $outdir -Recurse -Force -ErrorAction SilentlyContinue
    }
    # then update the system
    Expand-Archive -Path $outfile_zip -DestinationPath $outdir -Force
    if (-not(Test-Path $tasmdir)) {
        mkdir $tasmdir | Out-Null
    }
    Copy-Item -Path (Join-Path $outdir "*") -Destination $tasmdir -Recurse -Force
    $newpath = "$env:PATH;$tasmdir"
    [System.Environment]::SetEnvironmentVariable("PATH", $newpath, "User")
    Write-Host "Added executable path to PATH"

    Remove-Item $outdir -Recurse -Force
    Remove-Item $outfile_zip -Force

    Write-Host "$appname $version [$triple] has been installed."
    Write-Host "Restart your shell for PATH changes to take effect."
}

function remove {
    if (Test-Path $tasmdir) {
        # update $PATH
        $old = [System.Environment]::GetEnvironmentVariable("PATH", "User")
        $new = ($old -split ";" | Where-Object { $_ -ne $tasmdir }) -join ';'
        [System.Environment]::SetEnvironmentVariable("PATH", $new, "User")
        # remove install dir
        Remove-Item -Recurse $tasmdir
        Write-Host "$appname was removed." -ForegroundColor Green
    }
    else {
        Write-Host "$appname is already removed."
    }
}

switch ($action) {
    "install" { install $version }
    "remove" { remove }
    default {
        Write-Host $help
    }
}