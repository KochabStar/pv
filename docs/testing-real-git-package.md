# Testing the real Git package

This fixture uses the official Git for Windows MinGit ZIP asset so `pv install git`
exercises real network download, SHA-256 verification, ZIP extraction, activation,
and shim creation.

```powershell
$env:PV_HOME = "$PWD\.tmp\pv-real-git"
New-Item -ItemType Directory -Force "$env:PV_HOME\buckets\main" | Out-Null
Copy-Item ".\tests\fixtures\bucket\main\git.toml" "$env:PV_HOME\buckets\main\git.toml"
@"
[[buckets]]
name = "main"
url = "local"
"@ | Set-Content "$env:PV_HOME\config.toml"

.\target\debug\pv.exe install git
.\target\debug\pv.exe list git
& "$env:PV_HOME\shims\git.exe" --version
```
