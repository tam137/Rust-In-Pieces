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
echo -e "\n${YELLOW}[1/4] Running tests...${NC}"
cargo test
if [ $? -ne 0 ]; then
    echo -e "\n${RED}Error: Tests failed! Aborting build and release process.${NC}"
    exit 1
fi
echo -e "${GREEN}Success: All tests passed!${NC}"

# Step 2: Parse and bump version in Cargo.toml
echo -e "\n${YELLOW}[2/4] Bumping version in Cargo.toml...${NC}"
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Cargo.toml not found!${NC}"
    exit 1
fi

# Read current version
VERSION_LINE=$(grep -E '^version\s*=' Cargo.toml | head -n 1)
CURRENT_VERSION=$(echo "$VERSION_LINE" | sed -E 's/version\s*=\s*"([^"]+)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    echo -e "${RED}Error: Could not parse current version from Cargo.toml!${NC}"
    exit 1
fi

# Parse major, minor, patch
IFS='.' read -r major minor patch <<< "$CURRENT_VERSION"

# Bump the patch version
NEW_PATCH=$((patch + 1))
NEW_VERSION="$major.$minor.$NEW_PATCH"

echo -e "Current Version: ${BOLD}$CURRENT_VERSION${NC}"
echo -e "New Version:     ${GREEN}${BOLD}$NEW_VERSION${NC}"

# Replace version in Cargo.toml
sed -i "s/version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
echo -e "${GREEN}Success: Cargo.toml updated to version $NEW_VERSION!${NC}"

# Step 3: Build release binary
echo -e "\n${YELLOW}[3/4] Compiling release binary...${NC}"
cargo build --release
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Compilation failed! Rolling back version in Cargo.toml...${NC}"
    sed -i "s/version = \"$NEW_VERSION\"/version = \"$CURRENT_VERSION\"/" Cargo.toml
    exit 1
fi
echo -e "${GREEN}Success: Release binary compiled successfully!${NC}"

# Step 4: Copy to Matt-Magie engines folder
echo -e "\n${YELLOW}[4/4] Deploying release to Matt-Magie engines directory...${NC}"
TARGET_DIR="../matt-magie/engines"
mkdir -p "$TARGET_DIR"

COPY_TARGET="$TARGET_DIR/suprah-$NEW_VERSION"
cp "target/release/suprah" "$COPY_TARGET"
chmod +x "$COPY_TARGET"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Success: Deployed to $COPY_TARGET!${NC}"
    echo -e "\n${CYAN}================================================================${NC}"
    echo -e "${GREEN}${BOLD}RELEASE PROCESS COMPLETED SUCCESSFULLY!${NC}"
    echo -e "Engine ${BOLD}suprah-$NEW_VERSION${NC} is now ready for matchups."
    echo -e "${CYAN}================================================================${NC}"
else
    echo -e "${RED}Error: Failed to copy binary to $COPY_TARGET!${NC}"
    exit 1
fi
