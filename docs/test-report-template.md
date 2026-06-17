# memguard Stress Test Report

## Environment

- Hardware: ____________________
- CPU/RAM: ____________________
- OS: ____________________
- memguard version: ____________________
- Date: ____________________

## Procedure

1. Install `memguard` and enable the service:
   ```bash
   sudo dnf install memguard
   sudo systemctl enable --now memguard
   ```
2. Open a realistic workload, for example:
   - 50 Chrome tabs
   - VS Code with a medium project
   - `stress-ng --vm 2 --vm-bytes 1G` or a memory-heavy Docker container
3. Observe the system for at least 10 minutes while memory pressure rises.

## Observations

| Time | Pressure Level | Action Taken | Active App | Shell PID Alive? | Notes |
|------|----------------|--------------|------------|------------------|-------|
|      |                |              |            |                  |       |

## Pass Criteria

- [ ] System slows but does not crash.
- [ ] Desktop shell survives 100% of pressure events.
- [ ] Active window is never killed.
- [ ] Background apps are frozen before they are killed.

## Result

- Pass / Fail: ____________________
- Notes: ____________________
