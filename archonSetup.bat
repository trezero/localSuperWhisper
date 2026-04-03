@echo off
chcp 65001 >nul 2>&1
setlocal EnableDelayedExpansion

set "ARCHON_API_URL=http://localhost:8181"
set "ARCHON_MCP_URL=http://localhost:8051"

echo.
echo  =============================================
echo    Archon Setup
echo    Server: %ARCHON_MCP_URL%
echo  =============================================
echo.

:: If placeholders were not substituted, ask for the Archon host
echo %ARCHON_API_URL% | findstr /C:"{{" >nul 2>&1 && (
  echo  URLs not pre-configured. Please enter your Archon server address.
  echo.
  set /p "ARCHON_HOST=  Archon host (e.g. 192.168.1.10 or localhost): "
  set "ARCHON_API_URL=http://!ARCHON_HOST!:8181"
  set "ARCHON_MCP_URL=http://!ARCHON_HOST!:8051"
  echo.
  echo  Using API: !ARCHON_API_URL!
  echo  Using MCP: !ARCHON_MCP_URL!
  echo.
)

:: Check dependencies
where curl >nul 2>&1 || (echo Error: curl is required. Install from https://curl.se & exit /b 1)
where claude >nul 2>&1 || (echo Error: claude CLI not found. Install Claude Code first. & exit /b 1)
where powershell >nul 2>&1 || (echo Error: PowerShell is required. & exit /b 1)

:: Check Python availability and version (3.10+ required)
set "PYTHON_CMD="
where python3 >nul 2>&1 && set "PYTHON_CMD=python3"
if not defined PYTHON_CMD (
    where python >nul 2>&1 && set "PYTHON_CMD=python"
)
if not defined PYTHON_CMD (
    echo.
    echo  Error: Python is not installed.
    echo    Install Python 3.10 from: https://www.python.org/downloads/release/python-31011/
    echo    Or run: winget install Python.Python.3.10
    exit /b 1
)

set "PY_VERSION="
set "PY_OK="
for /f "delims=" %%V in ('!PYTHON_CMD! -c "import sys; v=sys.version_info; print(f'{v.major}.{v.minor}')" 2^>nul') do set "PY_VERSION=%%V"
for /f "delims=" %%V in ('!PYTHON_CMD! -c "import sys; print(sys.version_info >= (3,10))" 2^>nul') do set "PY_OK=%%V"

if not "!PY_OK!"=="True" (
    echo.
    echo  ! Python !PY_VERSION! detected -- Python 3.10+ is required.
    echo    The scanner and plugin system need Python 3.10+.
    echo.
    echo    Install with:  winget install Python.Python.3.10
    echo    Or download:   https://www.python.org/downloads/release/python-31011/
    echo.
    set /p "PY_CONTINUE=  Continue anyway? (y/N): "
    if /i not "!PY_CONTINUE!"=="y" (
        echo  Install Python 3.10+ and re-run this script.
        exit /b 0
    )
    echo.
)

:: -- Step 1/4: System name --------------------------------------------------
echo [1/4] System name
set "SYSTEM_NAME=%COMPUTERNAME%"
echo       Using: %SYSTEM_NAME%
echo.

:: -- Step 2/4: Project -----------------------------------------------------
echo [2/4] Project

for %%F in (.) do set "DIR_NAME=%%~nxF"
set "PROJECT_ID="
set "PROJECT_TITLE="

:: Try auto-match on current directory name
set "ENCODED_DIR="
for /f "delims=" %%E in ('powershell -Command "[uri]::EscapeDataString('!DIR_NAME!')"') do set "ENCODED_DIR=%%E"
set "MATCH_FILE=%TEMP%\archon_match.json"
curl -sf "%ARCHON_API_URL%/api/projects?include_content=false&q=!ENCODED_DIR!" -o "%MATCH_FILE%" 2>nul

set "MATCH_COUNT=0"
for /f "delims=" %%C in ('powershell -Command "$d = Get-Content '%MATCH_FILE%' | ConvertFrom-Json; $d.projects.Count" 2^>nul') do set "MATCH_COUNT=%%C"

if "!MATCH_COUNT!"=="1" (
  set "MATCHED_TITLE="
  set "MATCHED_ID="
  for /f "delims=" %%T in ('powershell -Command "$d = Get-Content '%MATCH_FILE%' | ConvertFrom-Json; $d.projects[0].title" 2^>nul') do set "MATCHED_TITLE=%%T"
  for /f "delims=" %%I in ('powershell -Command "$d = Get-Content '%MATCH_FILE%' | ConvertFrom-Json; $d.projects[0].id" 2^>nul') do set "MATCHED_ID=%%I"
  echo       Matched in Archon: !MATCHED_TITLE!
  set /p "CONFIRM=      Press Enter to accept or type to search: "
  if "!CONFIRM!"=="" (
    set "PROJECT_ID=!MATCHED_ID!"
    set "PROJECT_TITLE=!MATCHED_TITLE!"
    goto :project_done
  ) else (
    set "SEARCH_TERM=!CONFIRM!"
    goto :search_loop_body
  )
)

:search_loop
set /p "SEARCH_TERM=      Search projects (or Enter to list all): "

:search_loop_body
set "ENCODED_TERM="
for /f "delims=" %%E in ('powershell -Command "[uri]::EscapeDataString('%SEARCH_TERM%')"') do set "ENCODED_TERM=%%E"

set "RESULTS_FILE=%TEMP%\archon_projects.json"
curl -sf "%ARCHON_API_URL%/api/projects?include_content=false&q=!ENCODED_TERM!" -o "%RESULTS_FILE%" 2>nul

powershell -Command ^
  "$data = Get-Content '%RESULTS_FILE%' | ConvertFrom-Json; " ^
  "$projects = $data.projects | Select-Object -First 10; " ^
  "$i = 1; foreach ($p in $projects) { Write-Host ('        ' + $i + '. ' + $p.title); $i++ }"

echo         C. Create new project in Archon
echo.
set /p "SELECTION=      Enter number, new search, or C to create: "

if /i "%SELECTION%"=="C" goto :create_project

:: Check if numeric
echo %SELECTION%| findstr /r "^[0-9][0-9]*$" >nul
if %errorlevel%==0 (
  for /f "delims=" %%R in ('powershell -Command ^
    "$data = Get-Content '%RESULTS_FILE%' | ConvertFrom-Json; " ^
    "$projects = $data.projects; " ^
    "$idx = %SELECTION% - 1; " ^
    "if ($idx -lt $projects.Count) { $projects[$idx].id + '|' + $projects[$idx].title }"') do (
    for /f "tokens=1,2 delims=|" %%A in ("%%R") do (
      set "PROJECT_ID=%%A"
      set "PROJECT_TITLE=%%B"
    )
  )
  if defined PROJECT_ID goto :project_done
  echo       Invalid selection.
)

set "SEARCH_TERM=%SELECTION%"
goto :search_loop

:create_project
set /p "NEW_NAME=      New project name [%DIR_NAME%]: "
if "%NEW_NAME%"=="" set "NEW_NAME=%DIR_NAME%"
set /p "NEW_DESC=      Description (optional): "
echo       Creating project...
set "NAME_FILE=%TEMP%\archon_name.txt"
set "DESC_FILE=%TEMP%\archon_desc.txt"
set "BODY_FILE=%TEMP%\archon_body.json"
set "CREATE_FILE=%TEMP%\archon_create.json"

:: Write user inputs to temp files so they are never interpolated into PowerShell command strings
:: Use echo( syntax to avoid "ECHO is off." when variable is empty
(echo(!NEW_NAME!)>"%NAME_FILE%"
(echo(!NEW_DESC!)>"%DESC_FILE%"

:: Build JSON body by reading temp files inside PowerShell — no user input in the -Command string
powershell -Command ^
  "$n = (Get-Content '%NAME_FILE%').Trim(); " ^
  "$d = (Get-Content '%DESC_FILE%').Trim(); " ^
  "@{ title = $n; description = $d } | ConvertTo-Json | Set-Content '%BODY_FILE%'"

powershell -Command ^
  "try { $r = Invoke-RestMethod -Uri '%ARCHON_API_URL%/api/projects' -Method POST -Body (Get-Content '%BODY_FILE%' -Raw) -ContentType 'application/json'; $r.id } catch { '' }" ^
  > "%CREATE_FILE%" 2>nul

set "PROJECT_ID="
for /f "delims=" %%I in (%CREATE_FILE%) do set "PROJECT_ID=%%I"
set "PROJECT_TITLE=!NEW_NAME!"
if defined PROJECT_ID (
  echo       Created "!NEW_NAME!"
) else (
  echo       Error creating project. Continuing without project link.
)

:project_done
echo.

:: -- Step 3/4: Add MCP -----------------------------------------------------
echo [3/4] Setting up Claude Code MCP...
claude mcp add --transport http archon "%ARCHON_MCP_URL%/mcp" 2>nul || echo       (Already configured)
echo       Added archon MCP server
echo.

:: -- Step 3.5: Install scope ------------------------------------------------
echo.
echo Where should Archon tools be installed?
echo.
echo   [1] This project only (recommended)
echo       Installed to .claude\ in your project root.
echo       Customize per-project, changes stay isolated.
echo.
echo   [2] Global (all projects)
echo       Installed to %USERPROFILE%\.claude\ in your home directory.
echo       Same setup shared across all projects.
echo.
set /p "INSTALL_SCOPE=Choice [1]: "
if "!INSTALL_SCOPE!"=="" set "INSTALL_SCOPE=1"

if "!INSTALL_SCOPE!"=="2" (
    set "INSTALL_DIR=%USERPROFILE%\.claude"
) else (
    set "INSTALL_DIR=.claude"
)
echo.

:: -- Check for existing claude-mem plugin -----------------------------------
set "SKIP_PLUGIN_INSTALL=false"
if exist "%USERPROFILE%\.claude\plugins\cache\thedotmack\claude-mem" goto :claude_mem_found
if exist ".claude\plugins\claude-mem" goto :claude_mem_found
goto :plugin_install

:claude_mem_found
echo Detected existing plugin: claude-mem
echo The archon-memory plugin replaces claude-mem with enhanced
echo features and Archon integration.
echo.
echo   [1] Remove claude-mem and install archon-memory (recommended)
echo   [2] Keep both (not recommended - duplicate hooks and tools)
echo   [3] Skip plugin installation
echo.
set /p "CLAUDE_MEM_CHOICE=Choice [1]: "
if "!CLAUDE_MEM_CHOICE!"=="" set "CLAUDE_MEM_CHOICE=1"

if "!CLAUDE_MEM_CHOICE!"=="1" (
    if exist "%USERPROFILE%\.claude\plugins\cache\thedotmack\claude-mem" rmdir /s /q "%USERPROFILE%\.claude\plugins\cache\thedotmack\claude-mem"
    if exist ".claude\plugins\claude-mem" rmdir /s /q ".claude\plugins\claude-mem"
    echo ^✓ Removed claude-mem
)
if "!CLAUDE_MEM_CHOICE!"=="3" set "SKIP_PLUGIN_INSTALL=true"
echo.

:plugin_install
:: -- Install archon-memory plugin -------------------------------------------
if "!SKIP_PLUGIN_INSTALL!"=="true" goto :plugin_done

set "PLUGIN_DIR=!INSTALL_DIR!\plugins\archon-memory"
echo Installing archon-memory plugin...
if not exist "!PLUGIN_DIR!" mkdir "!PLUGIN_DIR!"
set "PLUGIN_TMP=%TEMP%\archon-memory.tar.gz"
curl -sf "%ARCHON_MCP_URL%/archon-setup/plugin/archon-memory.tar.gz" -o "%PLUGIN_TMP%" 2>nul
if %errorlevel%==0 (
    powershell -Command "tar -xzf '%PLUGIN_TMP%' -C '!INSTALL_DIR!\plugins\'" 2>nul
    echo       ^✓ Plugin installed to !PLUGIN_DIR!\
    :: Remove stale venv from previous install (different Python, broken state, etc.)
    if exist "!PLUGIN_DIR!\.venv" rmdir /s /q "!PLUGIN_DIR!\.venv" 2>nul
    :: Create fresh venv and install Python dependencies
    if exist "!PLUGIN_DIR!\requirements.txt" (
        echo Creating plugin virtual environment...
        !PYTHON_CMD! -m venv "!PLUGIN_DIR!\.venv" 2>nul
        if exist "!PLUGIN_DIR!\.venv\Scripts\python.exe" (
            echo       ^✓ Created venv
            "!PLUGIN_DIR!\.venv\Scripts\pip.exe" install -q --upgrade pip 2>nul
            echo Installing plugin dependencies...
            "!PLUGIN_DIR!\.venv\Scripts\pip.exe" install -q -r "!PLUGIN_DIR!\requirements.txt" 2>nul
            if %errorlevel%==0 (
                echo       ^✓ Plugin dependencies installed in venv
            ) else (
                echo       ^! pip install failed. Run manually:
                echo         !PLUGIN_DIR!\.venv\Scripts\pip install -r !PLUGIN_DIR!\requirements.txt
            )
            :: Verify the venv works
            "!PLUGIN_DIR!\.venv\Scripts\python.exe" -c "import httpx" 2>nul
            if not %errorlevel%==0 (
                echo       ^! Verification failed -- httpx not importable
            )
        ) else (
            echo       ^! Could not create venv. Falling back to system pip...
            !PYTHON_CMD! -m pip install -q -r "!PLUGIN_DIR!\requirements.txt" 2>nul
            if %errorlevel%==0 (
                echo       ^✓ Plugin dependencies installed ^(system-wide^)
            ) else (
                echo       ^! Could not install plugin dependencies.
            )
        )
    )
) else (
    echo       ^! Plugin download failed -- install manually from Archon
)
del "%PLUGIN_TMP%" 2>nul
echo.

:plugin_done
:: -- Register hooks in Claude Code settings ---------------------------------
:: SessionStart and Stop hooks only work in global ~/.claude/settings.json.
:: PostToolUse works in project settings.local.json.
if "!SKIP_PLUGIN_INSTALL!"=="true" goto :hooks_done

set "GLOBAL_SETTINGS=%USERPROFILE%\.claude\settings.json"

:: Determine Python executable and script paths for hooks
:: Use forward slashes in hook paths — Claude Code on Windows uses Git Bash, not cmd.exe
:: The PowerShell .Replace('\','/') normalizes any remaining backslashes before writing JSON.
if "!INSTALL_SCOPE!"=="2" (
    set "LC_PYTHON=!PLUGIN_DIR!/.venv/Scripts/python"
    set "LC_SCRIPTS=!PLUGIN_DIR!/scripts"
    set "PTU_PYTHON=!PLUGIN_DIR!/.venv/Scripts/python"
    set "PTU_SCRIPTS=!PLUGIN_DIR!/scripts"
    set "PTU_SETTINGS=!GLOBAL_SETTINGS!"
) else (
    set "LC_PYTHON=$CLAUDE_PROJECT_DIR/.claude/plugins/archon-memory/.venv/Scripts/python"
    set "LC_SCRIPTS=$CLAUDE_PROJECT_DIR/.claude/plugins/archon-memory/scripts"
    set "PTU_PYTHON=.claude/plugins/archon-memory/.venv/Scripts/python"
    set "PTU_SCRIPTS=.claude/plugins/archon-memory/scripts"
    set "PTU_SETTINGS=!INSTALL_DIR!\settings.local.json"
)

:: Fall back to detected system python if venv doesn't exist (global scope only)
if "!INSTALL_SCOPE!"=="2" (
    if not exist "!PLUGIN_DIR!\.venv\Scripts\python.exe" (
        set "LC_PYTHON=!PYTHON_CMD!"
        set "PTU_PYTHON=!PYTHON_CMD!"
    )
)

echo Registering lifecycle hooks in global settings...
set "LCPY_FILE=%TEMP%\archon_lcpy.txt"
set "LCSC_FILE=%TEMP%\archon_lcsc.txt"
(echo(!LC_PYTHON!)>"%LCPY_FILE%"
(echo(!LC_SCRIPTS!)>"%LCSC_FILE%"

:: Hook commands use bash syntax (test -f / &&) because Claude Code on Windows
:: executes hooks via Git Bash, not cmd.exe.  Paths use forward slashes.
powershell -Command ^
  "$settingsPath = '!GLOBAL_SETTINGS!'; " ^
  "$pyPath = (Get-Content '%LCPY_FILE%').Trim().Replace('\','/'); " ^
  "$scPath = (Get-Content '%LCSC_FILE%').Trim().Replace('\','/'); " ^
  "if (Test-Path $settingsPath) { $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json } else { $settings = @{} }; " ^
  "if (-not $settings.hooks) { $settings | Add-Member -Force -NotePropertyName 'hooks' -NotePropertyValue @{} }; " ^
  "$hooks = $settings.hooks; " ^
  "$ssHook = @(@{ matcher=''; hooks=@(@{ type='command'; command=('test -f \"' + $scPath + '/session_start_hook.py\" && \"' + $pyPath + '\" \"' + $scPath + '/session_start_hook.py\" || true'); timeout=10 }) }); " ^
  "$stopHook = @(@{ matcher=''; hooks=@(@{ type='command'; command=('test -f \"' + $scPath + '/session_end_hook.py\" && \"' + $pyPath + '\" \"' + $scPath + '/session_end_hook.py\" || true'); timeout=30 }) }); " ^
  "$hooks | Add-Member -Force -NotePropertyName 'SessionStart' -NotePropertyValue $ssHook; " ^
  "$hooks | Add-Member -Force -NotePropertyName 'Stop' -NotePropertyValue $stopHook; " ^
  "$settings | ConvertTo-Json -Depth 10 | Set-Content $settingsPath"

echo       ^✓ SessionStart + Stop hooks registered globally

echo Registering PostToolUse hook...
set "PTUPY_FILE=%TEMP%\archon_ptupy.txt"
set "PTUSC_FILE=%TEMP%\archon_ptusc.txt"
(echo(!PTU_PYTHON!)>"%PTUPY_FILE%"
(echo(!PTU_SCRIPTS!)>"%PTUSC_FILE%"

powershell -Command ^
  "$settingsPath = '!PTU_SETTINGS!'; " ^
  "$pyPath = (Get-Content '%PTUPY_FILE%').Trim().Replace('\','/'); " ^
  "$scPath = (Get-Content '%PTUSC_FILE%').Trim().Replace('\','/'); " ^
  "if (Test-Path $settingsPath) { $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json } else { $settings = @{} }; " ^
  "if (-not $settings.hooks) { $settings | Add-Member -Force -NotePropertyName 'hooks' -NotePropertyValue @{} }; " ^
  "$hooks = $settings.hooks; " ^
  "$ptuHook = @(@{ matcher=''; hooks=@(@{ type='command'; command=('test -f \"' + $scPath + '/observation_hook.py\" && \"' + $pyPath + '\" \"' + $scPath + '/observation_hook.py\" || true'); timeout=5 }) }); " ^
  "$hooks | Add-Member -Force -NotePropertyName 'PostToolUse' -NotePropertyValue $ptuHook; " ^
  "$settings | ConvertTo-Json -Depth 10 | Set-Content $settingsPath"

del "%LCPY_FILE%" "%LCSC_FILE%" "%PTUPY_FILE%" "%PTUSC_FILE%" 2>nul
echo       ^✓ PostToolUse hook registered
echo.

:hooks_done
:: -- Download and install extensions ----------------------------------------
echo Installing extensions...
if not exist "!INSTALL_DIR!\skills" mkdir "!INSTALL_DIR!\skills"
set "EXT_TMP=%TEMP%\archon-extensions.tar.gz"
curl -sf "%ARCHON_MCP_URL%/archon-setup/extensions.tar.gz" -o "%EXT_TMP%" 2>nul
if %errorlevel%==0 (
    powershell -Command "tar -xzf '%EXT_TMP%' -C '!INSTALL_DIR!\skills\'" 2>nul
    for /f %%C in ('dir /b /s "!INSTALL_DIR!\skills\SKILL.md" 2^>nul ^| find /c /v ""') do echo       ^✓ Installed %%C extension^(s^) to !INSTALL_DIR!\skills\
) else (
    echo       ^! Extension download failed -- /archon-setup will handle installation
)
del "%EXT_TMP%" 2>nul
echo.

:: -- Write archon-config.json -----------------------------------------------
set "FINGERPRINT_FILE=%TEMP%\archon_fp.txt"
powershell -Command ^
  "$h = [System.Security.Cryptography.MD5]::Create(); " ^
  "$b = [System.Text.Encoding]::UTF8.GetBytes($env:COMPUTERNAME + [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value); " ^
  "$hash = ($h.ComputeHash($b) | ForEach-Object { $_.ToString('x2') }) -join ''; " ^
  "$hash.Substring(0,16)" > "%FINGERPRINT_FILE%" 2>nul
set "MACHINE_FINGERPRINT="
for /f "delims=" %%F in (%FINGERPRINT_FILE%) do set "MACHINE_FINGERPRINT=%%F"
del "%FINGERPRINT_FILE%" 2>nul

set "CONFIG_FILE=%TEMP%\archon_config_vals.txt"
(echo(!ARCHON_API_URL!)>"%TEMP%\archon_apiurl.txt"
(echo(!ARCHON_MCP_URL!)>"%TEMP%\archon_mcpurl.txt"
(echo(!PROJECT_ID!)>"%TEMP%\archon_pid.txt"
(echo(!PROJECT_TITLE!)>"%TEMP%\archon_ptitle.txt"
(echo(!MACHINE_FINGERPRINT!)>"%TEMP%\archon_mfp.txt"
(echo(!INSTALL_SCOPE!)>"%TEMP%\archon_scope.txt"

powershell -Command ^
  "$apiUrl  = (Get-Content '%TEMP%\archon_apiurl.txt').Trim(); " ^
  "$mcpUrl  = (Get-Content '%TEMP%\archon_mcpurl.txt').Trim(); " ^
  "$projId  = (Get-Content '%TEMP%\archon_pid.txt').Trim(); " ^
  "$projTitle = (Get-Content '%TEMP%\archon_ptitle.txt').Trim(); " ^
  "$mfp     = (Get-Content '%TEMP%\archon_mfp.txt').Trim(); " ^
  "$scope   = (Get-Content '%TEMP%\archon_scope.txt').Trim(); " ^
  "$ts      = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'); " ^
  "$cfg = [ordered]@{ archon_api_url=$apiUrl; archon_mcp_url=$mcpUrl; project_id=$projId; project_title=$projTitle; machine_id=$mfp; install_scope=$scope; installed_at=$ts }; " ^
  "$cfg | ConvertTo-Json | Set-Content '!INSTALL_DIR!\archon-config.json'"

echo       ^✓ Wrote !INSTALL_DIR!\archon-config.json
echo.

:: -- Update .gitignore ------------------------------------------------------
for %%G in (".claude/plugins/" ".claude/skills/" ".claude/archon-config.json" ".claude/archon-state.json" ".claude/archon-memory-buffer.jsonl" ".archon/") do (
    findstr /x /c:"%%~G" .gitignore >nul 2>&1 || echo %%~G>>.gitignore
)

:: -- Inject Archon rules into CLAUDE.md ------------------------------------
echo Configuring CLAUDE.md project rules...
set "SNIPPET_FILE=%TEMP%\archon_claude_md_snippet.md"
curl -sf "%ARCHON_MCP_URL%/archon-setup/claude-md-snippet.md" -o "%SNIPPET_FILE%" 2>nul

if %errorlevel%==0 if exist "%SNIPPET_FILE%" (
    set "MARKER_START=<!-- archon-rules-start -->"
    set "MARKER_END=<!-- archon-rules-end -->"

    if not exist "CLAUDE.md" (
        :: No CLAUDE.md — create with Archon rules
        powershell -Command ^
            "$ms = '<!-- archon-rules-start -->'; " ^
            "$me = '<!-- archon-rules-end -->'; " ^
            "$snippet = Get-Content '%SNIPPET_FILE%' -Raw; " ^
            "($ms + [Environment]::NewLine + $snippet.TrimEnd() + [Environment]::NewLine + $me + [Environment]::NewLine) | Set-Content 'CLAUDE.md' -NoNewline"
        echo       ^✓ Created CLAUDE.md with Archon rules
    ) else (
        :: CLAUDE.md exists — check if markers already present
        findstr /c:"<!-- archon-rules-start -->" "CLAUDE.md" >nul 2>&1
        if !errorlevel!==0 (
            :: Markers found — replace section between markers
            powershell -Command ^
                "$ms = '<!-- archon-rules-start -->'; " ^
                "$me = '<!-- archon-rules-end -->'; " ^
                "$snippet = Get-Content '%SNIPPET_FILE%' -Raw; " ^
                "$content = Get-Content 'CLAUDE.md' -Raw; " ^
                "$si = $content.IndexOf($ms); " ^
                "$ei = $content.IndexOf($me) + $me.Length; " ^
                "if ($ei -lt $content.Length -and $content[$ei] -eq [char]10) { $ei++ }; " ^
                "if ($ei -lt $content.Length -and $content[$ei] -eq [char]13) { $ei++ }; " ^
                "$updated = $content.Substring(0, $si) + $ms + [Environment]::NewLine + $snippet.TrimEnd() + [Environment]::NewLine + $me + [Environment]::NewLine + $content.Substring($ei); " ^
                "$updated | Set-Content 'CLAUDE.md' -NoNewline"
            echo       ^✓ Updated Archon rules in CLAUDE.md
        ) else (
            :: No markers — append with markers
            powershell -Command ^
                "$ms = '<!-- archon-rules-start -->'; " ^
                "$me = '<!-- archon-rules-end -->'; " ^
                "$snippet = Get-Content '%SNIPPET_FILE%' -Raw; " ^
                "$append = [Environment]::NewLine + [Environment]::NewLine + $ms + [Environment]::NewLine + $snippet.TrimEnd() + [Environment]::NewLine + $me + [Environment]::NewLine; " ^
                "$append | Add-Content 'CLAUDE.md' -NoNewline"
            echo       ^✓ Appended Archon rules to CLAUDE.md
            echo         Run /archon-setup in Claude Code for intelligent merge with existing rules.
        )
    )
) else (
    echo       ^! Could not download CLAUDE.md snippet. Skipping rules injection.
    echo         Run /archon-setup in Claude Code to configure project rules.
)
del "%SNIPPET_FILE%" 2>nul
echo.

:: -- Step 4/4: Install slash commands ---------------------------------------
echo [4/4] Installing slash commands...
if not exist "%USERPROFILE%\.claude\commands" mkdir "%USERPROFILE%\.claude\commands"
curl -sf "%ARCHON_MCP_URL%/archon-setup.md" -o "%USERPROFILE%\.claude\commands\archon-setup.md"
echo       Installed /archon-setup to %USERPROFILE%\.claude\commands\archon-setup.md
curl -sf "%ARCHON_MCP_URL%/scan-projects.md" -o "%USERPROFILE%\.claude\commands\scan-projects.md"
echo       Installed /scan-projects to %USERPROFILE%\.claude\commands\scan-projects.md
echo.

:: -- Write initial state ----------------------------------------------------
if not exist ".claude" mkdir ".claude"
set "SYSNAME_FILE=%TEMP%\archon_sysname.txt"
set "PROJID_FILE=%TEMP%\archon_projid.txt"

:: Write user-supplied values to temp files to avoid interpolation into PowerShell command strings
(echo(!SYSTEM_NAME!)>"%SYSNAME_FILE%"
(echo(!PROJECT_ID!)>"%PROJID_FILE%"

powershell -Command ^
  "$sysName = (Get-Content '%SYSNAME_FILE%').Trim(); " ^
  "$projId = (Get-Content '%PROJID_FILE%').Trim(); " ^
  "$state = if (Test-Path '.claude\archon-state.json') { Get-Content '.claude\archon-state.json' | ConvertFrom-Json } else { @{} }; " ^
  "$state | Add-Member -Force -NotePropertyName 'system_name' -NotePropertyValue $sysName; " ^
  "if ($projId) { $state | Add-Member -Force -NotePropertyName 'archon_project_id' -NotePropertyValue $projId }; " ^
  "$state | ConvertTo-Json | Set-Content '.claude\archon-state.json'"

:: -- Done ------------------------------------------------------------------
echo =============================================
echo  Setup complete!
echo.
echo  Open Claude Code and run:
echo.
echo    /archon-setup
echo.
echo  This will sync extensions and project context.
echo =============================================
echo.
