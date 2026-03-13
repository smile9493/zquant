param(
  [string]$Image = "postgres:16",
  [string]$ContainerName = "zquant-pg-test",
  [int]$HostPort = 55432,
  [string]$DbName = "webquant_test",
  [string]$User = "postgres",
  [string]$Password = "postgres",
  [switch]$KeepContainer
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -Scope Global -ErrorAction SilentlyContinue) {
  $global:PSNativeCommandUseErrorActionPreference = $false
}

function Invoke-Native {
  param(
    [Parameter(Mandatory = $true)]
    [string[]]$CommandLine
  )

  if ($CommandLine.Length -lt 1) { throw "CommandLine cannot be empty" }

  $exe = $CommandLine[0]
  $args = @()
  if ($CommandLine.Length -gt 1) { $args = $CommandLine[1..($CommandLine.Length - 1)] }

  & $exe @args
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed ($LASTEXITCODE): $exe $($args -join ' ')"
  }
}

function Cleanup {
  param([switch]$Force)
  if ($KeepContainer -and -not $Force) { return }
  try { & docker rm -f $ContainerName 2>$null | Out-Null } catch { }
}

try {
  $repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
  $migrations = (Resolve-Path (Join-Path $repoRoot "migrations")).Path

  Cleanup -Force

  Invoke-Native @(
    "docker", "run", "-d",
    "--name", $ContainerName,
    "-e", "POSTGRES_PASSWORD=$Password",
    "-e", "POSTGRES_DB=$DbName",
    "-e", "POSTGRES_INITDB_ARGS=--encoding=UTF8 --locale=C",
    "-p", "$HostPort`:5432",
    "-v", "${migrations}:/migrations",
    $Image
  ) | Out-Null

  $deadline = (Get-Date).AddSeconds(60)
  while ($true) {
    cmd /c "docker exec $ContainerName pg_isready -U $User -d $DbName >NUL 2>NUL"
    if ($LASTEXITCODE -eq 0) { break }
    if ((Get-Date) -gt $deadline) { throw "Timed out waiting for postgres to become ready" }
    Start-Sleep -Milliseconds 250
  }

  $deadline = (Get-Date).AddSeconds(60)
  while ($true) {
    cmd /c "docker exec $ContainerName psql -U $User -d $DbName -c ""SELECT 1"" >NUL 2>NUL"
    if ($LASTEXITCODE -eq 0) { break }
    if ((Get-Date) -gt $deadline) { throw "Timed out waiting for postgres to accept queries" }
    Start-Sleep -Milliseconds 250
  }

  $deadline = (Get-Date).AddSeconds(60)
  while ($true) {
    try {
      Invoke-Native @(
        "docker", "exec", $ContainerName,
        "psql", "-v", "ON_ERROR_STOP=1", "-U", $User, "-d", $DbName, "-f", "/migrations/0001_jobs.sql"
      ) | Out-Null
      Invoke-Native @(
        "docker", "exec", $ContainerName,
        "psql", "-v", "ON_ERROR_STOP=1", "-U", $User, "-d", $DbName, "-f", "/migrations/0002_phase1.sql"
      ) | Out-Null
      break
    } catch {
      if ((Get-Date) -gt $deadline) { throw }
      Start-Sleep -Milliseconds 250
    }
  }

  $env:DATABASE_URL = "postgres://$User`:$Password@localhost`:$HostPort/$DbName"
  Push-Location $repoRoot
  try {
    Invoke-Native @("cargo", "test", "-p", "job-store-pg")
  } catch {
    Write-Host "`n=== Test container logs (last 50 lines) ===" -ForegroundColor Yellow
    & docker logs --tail 50 $ContainerName 2>&1
    Write-Host "=== End of container logs ===`n" -ForegroundColor Yellow
    throw
  } finally {
    Pop-Location
  }
} finally {
  Cleanup
}
