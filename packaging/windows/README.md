# Running fh6-tel-serve as a Windows background service

## Option A — Scheduled Task at logon (simplest)
1. Open Task Scheduler -> Create Task.
2. Trigger: "At log on".
3. Action: Start a program -> `C:\path\to\fh6-tel-serve.exe`
   Arguments: `--ip 0.0.0.0 --port 8080 --auth-token CHANGE_ME`
4. Check "Run whether user is logged on or not" if you want it headless.

## Option B — True service via NSSM
A console exe needs a wrapper to be a real Windows service.
1. Download NSSM (https://nssm.cc).
2. `nssm install fh6-tel "C:\path\to\fh6-tel-serve.exe" --ip 0.0.0.0 --port 8080 --auth-token CHANGE_ME`
3. `nssm start fh6-tel`
