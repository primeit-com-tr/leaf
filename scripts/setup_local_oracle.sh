#!/bin/bash

set -e

PROJECT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
ORACLE_DIR="${PROJECT_ROOT}/.uncommitted/instantclient_21_19"

echo "Setting up local Oracle Instant Client in: ${ORACLE_DIR}"

# Create directory
mkdir -p "${ORACLE_DIR}"

cd "${ORACLE_DIR}"

# Download Oracle Instant Client if not already present
if [ ! -f "instantclient-basic-linux.x64-21.19.0.0.0dbru.zip" ]; then
    echo "Downloading Oracle Instant Client Basic..."
    wget https://download.oracle.com/otn_software/linux/instantclient/2119000/instantclient-basic-linux.x64-21.19.0.0.0dbru.zip
fi

# Extract if not already extracted
if [ ! -d "instantclient_21_19" ]; then
    echo "Extracting Oracle Instant Client..."
    unzip -q instantclient-basic-linux.x64-21.19.0.0.0dbru.zip
fi

# Download libaio locally (extract from deb package)
echo "Downloading libaio library..."
cd /tmp
apt-get download libaio1t64

# Extract the .deb file
dpkg-deb -x libaio1t64_*.deb "${ORACLE_DIR}/libaio_extract"

# Copy the library to instant client directory
cp "${ORACLE_DIR}/libaio_extract/usr/lib/x86_64-linux-gnu/libaio.so.1t64"* "${ORACLE_DIR}/instantclient_21_19/"

# Create symlink
cd "${ORACLE_DIR}/instantclient_21_19"
ln -sf libaio.so.1t64 libaio.so.1

# Clean up
rm -rf "${ORACLE_DIR}/libaio_extract"
rm -f /tmp/libaio1t64_*.deb

echo ""
echo "✅ Oracle Instant Client setup complete!"
echo ""
echo "Add this to your shell configuration (~/.bashrc or ~/.zshrc):"
echo ""
echo "export LD_LIBRARY_PATH=${ORACLE_DIR}/instantclient_21_19:\$LD_LIBRARY_PATH"
echo ""
echo "Or run for this session only:"
echo "export LD_LIBRARY_PATH=${ORACLE_DIR}/instantclient_21_19:\$LD_LIBRARY_PATH"
