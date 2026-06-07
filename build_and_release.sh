#!/bin/bash

# Ensure we are in the script's directory
cd "$(dirname "$0")"

# Color definitions for premium output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color
BOLD='\033[1m'

echo -e "${CYAN}================================================================${NC}"
echo -e "${CYAN}             SUPRAH AUTOMATED BUILD & RELEASE PIPELINE          ${NC}"
echo -e "${CYAN}================================================================${NC}"

# Step 1: Run all tests
echo -e "\n${YELLOW}[1/6] Running tests...${NC}"
cargo test
if [ $? -ne 0 ]; then
    echo -e "\n${RED}Error: Tests failed! Aborting build and release process.${NC}"
    exit 1
fi
echo -e "${GREEN}Success: All tests passed!${NC}"

# Step 2: Backup configuration files
echo -e "\n${YELLOW}[2/6] Creating backups for safe rollback...${NC}"
if [ ! -f "Cargo.toml" ] || [ ! -f "CHANGELOG.md" ]; then
    echo -e "${RED}Error: Cargo.toml or CHANGELOG.md not found!${NC}"
    exit 1
fi

cp Cargo.toml Cargo.toml.bak
cp CHANGELOG.md CHANGELOG.md.bak

rollback() {
    echo -e "\n${RED}Rolling back configuration files...${NC}"
    [ -f Cargo.toml.bak ] && cp Cargo.toml.bak Cargo.toml && rm Cargo.toml.bak
    [ -f CHANGELOG.md.bak ] && cp CHANGELOG.md.bak CHANGELOG.md && rm CHANGELOG.md.bak
}

# Step 3: Parse and bump version in Cargo.toml & update CHANGELOG.md
echo -e "\n${YELLOW}[3/6] Bumping version and updating CHANGELOG.md...${NC}"

# Read current version
VERSION_LINE=$(grep -E '^version\s*=' Cargo.toml | head -n 1)
CURRENT_VERSION=$(echo "$VERSION_LINE" | sed -E 's/version\s*=\s*"([^"]+)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    echo -e "${RED}Error: Could not parse current version from Cargo.toml!${NC}"
    rollback
    exit 1
fi

# Determine new version
if [ -n "$OVERRIDE_VERSION" ]; then
    NEW_VERSION="$OVERRIDE_VERSION"
    echo -e "Overriding version with environment variable: ${GREEN}${BOLD}$NEW_VERSION${NC}"
else
    # Parse major, minor, patch
    IFS='.' read -r major minor patch <<< "$CURRENT_VERSION"

    # Bump the patch version
    NEW_PATCH=$((patch + 1))
    NEW_VERSION="$major.$minor.$NEW_PATCH"
fi

echo -e "Current Version: ${BOLD}$CURRENT_VERSION${NC}"
echo -e "New Version:     ${GREEN}${BOLD}$NEW_VERSION${NC}"

# Replace version in Cargo.toml
sed -i "s/version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
echo -e "${GREEN}Success: Cargo.toml updated to version $NEW_VERSION!${NC}"

# Update CHANGELOG.md
DATE=$(date +%Y-%m-%d)
CHANGES=""

# Use the first command-line argument as the changelog message if provided
if [ -n "$1" ]; then
    echo -e "${CYAN}Using manual changelog message from arguments...${NC}"
    # Format the input message properly as bullet points
    if [[ "$1" =~ ^- ]]; then
        CHANGES="$1"
    else
        CHANGES="- $1"
    fi
else
    echo -e "${CYAN}No manual changelog message provided. Fetching recent git commits for changelog...${NC}"
    COMMITS=$(git log -n 5 --oneline | cut -d' ' -f2-)
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        if [[ ! "$line" =~ "finalize" && ! "$line" =~ "bump" && ! "$line" =~ "release" ]]; then
            CHANGES="$CHANGES\n- $line"
        fi
    done <<< "$COMMITS"
fi

if [ -z "$CHANGES" ]; then
    CHANGES="\n- General improvements and updates"
fi

echo -e "Changelog Changes to be added:\n${CYAN}$CHANGES${NC}"

# Insert into CHANGELOG.md using a robust python command
python3 -c "
import sys
version = sys.argv[1]
date = sys.argv[2]
changes = sys.argv[3].replace('\\\\n', '\\n')

with open('CHANGELOG.md', 'r') as f:
    content = f.read()

entry = f'## [V{version}] - {date}\n\n### Added\n{changes}\n\n### Fixed\n\n'

idx = content.find('## [')
if idx != -1:
    new_content = content[:idx] + entry + '\n\n' + content[idx:]
    with open('CHANGELOG.md', 'w') as f:
        f.write(new_content)
else:
    with open('CHANGELOG.md', 'a') as f:
        f.write('\n\n' + entry)
" "$NEW_VERSION" "$DATE" "$CHANGES"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Success: CHANGELOG.md updated successfully!${NC}"
else
    echo -e "${RED}Error: Failed to update CHANGELOG.md!${NC}"
    rollback
    exit 1
fi

# Step 4: Build release binary
echo -e "\n${YELLOW}[4/6] Compiling release binary...${NC}"
cargo build --release
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Compilation failed!${NC}"
    rollback
    exit 1
fi
echo -e "${GREEN}Success: Release binary compiled successfully!${NC}"

# Step 5: Copy to Matt-Magie engines folder
echo -e "\n${YELLOW}[5/6] Deploying release to Matt-Magie engines directory...${NC}"
TARGET_DIR="../matt-magie/engines"
mkdir -p "$TARGET_DIR"

COPY_TARGET="$TARGET_DIR/suprah-$NEW_VERSION"
cp "target/release/suprah" "$COPY_TARGET"
chmod +x "$COPY_TARGET"

if [ $? -eq 0 ]; then
    # Clean up backups since build and deploy succeeded
    rm -f Cargo.toml.bak CHANGELOG.md.bak
    echo -e "${GREEN}Success: Deployed to $COPY_TARGET!${NC}"
    
    # Step 6: Remote ARM Server Compilation & Deployment
    if [ -z "$EODSERVERIP" ]; then
        echo -e "\n${YELLOW}Warning: Environment variable EODSERVERIP is not set.${NC}"
        echo -e "Skipping native compilation and deployment on remote ARM server."
        echo -e "To deploy to the ARM server, please run: export EODSERVERIP=\"<IP>\" before releasing."
    else
        echo -e "\n${YELLOW}[6/6] Starting remote ARM compilation and deployment to ${EODSERVERIP}...${NC}"
        REMOTE_USER="root"
        REMOTE_DIR="/root/mattmagie"
        REMOTE_TMP_DIR="${REMOTE_DIR}/tmp_suprah_build"

        # A. Package, upload, compile and deploy in a single SSH connection to avoid rate limiting / disconnections
        echo -e "${YELLOW}Packaging, uploading, and compiling suprah on remote server natively...${NC}"
        tar -cf - Cargo.toml src | ssh ${REMOTE_USER}@${EODSERVERIP} "mkdir -p ${REMOTE_TMP_DIR} && tar -xf - -C ${REMOTE_TMP_DIR} && source \$HOME/.cargo/env && cd ${REMOTE_TMP_DIR} && rm -f Cargo.lock && cargo build --release && mkdir -p ${REMOTE_DIR}/engines && cp target/release/suprah ${REMOTE_DIR}/engines/suprah-${NEW_VERSION} && chmod +x ${REMOTE_DIR}/engines/suprah-${NEW_VERSION} && cd / && rm -rf ${REMOTE_TMP_DIR}"
        if [ $? -ne 0 ]; then
            echo -e "${RED}Error: Remote compilation and deployment failed!${NC}"
            rollback
            exit 1
        fi
        echo -e "${GREEN}Success: Remote compilation and deployment completed successfully!${NC}"
    fi

    # Git commit and tagging
    echo -e "\n${YELLOW}Creating git commit and tag for version v$NEW_VERSION...${NC}"
    # Stage all modified tracked files so the tag/release includes all codebase modifications
    git add -u
    git commit -m "Release v$NEW_VERSION"
    if [ $? -eq 0 ]; then
        git tag -a "v$NEW_VERSION" -m "Release version v$NEW_VERSION"
        echo -e "${GREEN}Success: Created git commit and tag v$NEW_VERSION!${NC}"
    else
        echo -e "${RED}Warning: Git commit failed. Skipping tagging.${NC}"
    fi

    echo -e "\n${CYAN}================================================================${NC}"
    echo -e "${GREEN}${BOLD}RELEASE PROCESS COMPLETED SUCCESSFULLY!${NC}"
    echo -e "Engine ${BOLD}suprah-$NEW_VERSION${NC} is now ready for matchups."
    if [ -n "$EODSERVERIP" ]; then
        echo -e "ARM build deployed to remote server at ${EODSERVERIP}:/root/mattmagie/engines/suprah-$NEW_VERSION"
    fi
    echo -e "\n${YELLOW}Reminder: Please open CHANGELOG.md and manually enrich the release notes with detailed explanations of changes.${NC}"
    echo -e "${CYAN}================================================================${NC}"
else
    echo -e "${RED}Error: Failed to copy binary to $COPY_TARGET!${NC}"
    rollback
    exit 1
fi
