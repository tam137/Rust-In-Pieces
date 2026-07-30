# Script Directory Guidelines (`scripts/`)

This directory contains utility, testing, diagnostic, and maintenance scripts for the project. All scripts stored here must be committed to version control for team-wide reusability and automated workflows.

---

## Mandatory Script Rules

### 1. Scope & Version Control
- **Purpose:** Helper scripts stored in `scripts/` are strictly designated for testing, benchmarking, tuning, diagnostic evaluation, and engine maintenance.
- **Version Control Requirement:** All scripts must be tracked in version control (Git) to guarantee reusability, consistency, and reproducibility across environments.

### 2. Path Handling & Portability
- **No Hardcoded Absolute Paths:** Hardcoded absolute file paths (e.g., `/home/username/...` or `C:\Users\...`) are strictly forbidden.
- **Dynamic Path Resolution:** All file and directory references must be resolved dynamically relative to the script location (e.g., `os.path.dirname(__file__)` in Python or `$(dirname "$0")` in Bash) or the project root.

### 3. Security, Confidentiality & Privacy
- **No Exposed Credentials or Secrets:** Never hardcode passwords, secret keys, tokens, or authentication credentials.
- **No Internal System or Infrastructure Details:** Do not hardcode internal server IP addresses, hostnames, internal network paths, or private infrastructure structures.
- **No Personal or Environment Data Dumps:** Scripts must not output or expose OS environment variable contents, personal directory names, or sensitive user data.
- **Parameterization:** Any variable external configuration (such as remote server endpoints) must be passed dynamically via command-line parameters or environment variables.
