#!/bin/bash
set -euo pipefail

# Prime Installer for macOS and Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/alyghaly2020-ux/prime/master/install.sh | bash

REPO="alyghaly2020-ux/prime"
GH="https://github.com/$REPO"

BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

detect_platform() {
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  ARCH="$(uname -m)"

  case "$OS" in
    darwin) OS="macos" ;;
    linux)  OS="linux" ;;
    *)
      echo -e "${RED}Unsupported OS: $OS${NC}"
      echo "For Windows, use:"
      echo '  powershell -c "irm https://raw.githubusercontent.com/alyghaly2020-ux/prime/master/install.ps1 | iex"'
      exit 1
      ;;
  esac

  case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *)
      echo -e "${RED}Unsupported architecture: $ARCH${NC}"
      exit 1
      ;;
  esac
}

detect_linux_distro() {
  if command -v dpkg &>/dev/null; then
    echo "debian"
  elif command -v rpm &>/dev/null; then
    echo "rpm"
  else
    echo "appimage"
  fi
}

get_latest_version() {
  if command -v curl &>/dev/null; then
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | cut -d'"' -f4 | sed 's/^v//'
  elif command -v wget &>/dev/null; then
    wget -qO- "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | cut -d'"' -f4 | sed 's/^v//'
  else
    echo -e "${RED}Need curl or wget${NC}"
    exit 1
  fi
}

download() {
  local url="$1" out="$2"
  if command -v curl &>/dev/null; then
    curl -fsSL "$url" -o "$out"
  else
    wget -qO "$out" "$url"
  fi
}

install_linux() {
  local distro
  distro=$(detect_linux_distro)

  case "$distro" in
    debian)
      local tmp="/tmp/prime.deb"
      echo -e "${CYAN}Downloading Prime ${VERSION} (Debian/Ubuntu)...${NC}"
      download "$GH/releases/download/v${VERSION}/prime_${VERSION}_amd64.deb" "$tmp"
      echo -e "${CYAN}Installing...${NC}"
      sudo dpkg -i "$tmp" 2>/dev/null || sudo apt-get install -f -y -qq
      rm -f "$tmp"
      ;;
    rpm)
      local tmp="/tmp/prime.rpm"
      echo -e "${CYAN}Downloading Prime ${VERSION} (Fedora/RHEL)...${NC}"
      download "$GH/releases/download/v${VERSION}/prime-${VERSION}-1.x86_64.rpm" "$tmp"
      echo -e "${CYAN}Installing...${NC}"
      sudo rpm -i "$tmp" 2>/dev/null || sudo dnf install -y "$tmp" 2>/dev/null || sudo yum install -y "$tmp"
      rm -f "$tmp"
      ;;
    appimage)
      local dest="/usr/local/bin/prime"
      echo -e "${CYAN}Downloading Prime ${VERSION} (AppImage)...${NC}"
      download "$GH/releases/download/v${VERSION}/Prime_${VERSION}_x86_64.AppImage" "/tmp/prime.AppImage"
      echo -e "${CYAN}Installing...${NC}"
      sudo mv /tmp/prime.AppImage "$dest"
      sudo chmod +x "$dest"
      ;;
  esac

  echo ""
  echo -e "${GREEN}✓ Prime ${VERSION} installed!${NC}"
  echo "  Run:  ${BOLD}prime${NC}"
  echo "  Headless:  ${BOLD}prime headless --port 9876${NC}"
}

install_macos() {
  local asset arch_label
  if [ "$ARCH" = "x86_64" ]; then
    asset="Prime_${VERSION}_x64.dmg"
    arch_label="Intel"
  else
    asset="Prime_${VERSION}_aarch64.dmg"
    arch_label="Silicon"
  fi

  local tmp="/tmp/prime.dmg"
  echo -e "${CYAN}Downloading Prime ${VERSION} for macOS ${arch_label}...${NC}"
  download "$GH/releases/download/v${VERSION}/$asset" "$tmp"

  echo -e "${CYAN}Installing...${NC}"
  hdiutil attach "$tmp" -nobrowse -quiet 2>/dev/null
  sudo cp -R "/Volumes/Prime/Prime.app" /Applications/
  hdiutil detach "/Volumes/Prime" -quiet 2>/dev/null
  rm -f "$tmp"

  echo ""
  echo -e "${GREEN}✓ Prime ${VERSION} installed!${NC}"
  echo "  Open from Applications or run:  ${BOLD}open -a Prime${NC}"
  echo "  Headless:  ${BOLD}/Applications/Prime.app/Contents/MacOS/prime headless --port 9876${NC}"
}

echo ""
echo -e "${BOLD}  Prime Installer${NC}"
echo ""

detect_platform
VERSION=$(get_latest_version)

echo -e "  ${CYAN}Detected:${NC} $OS ($ARCH)"
echo -e "  ${CYAN}Version:${NC}  $VERSION"
echo ""

if [ "$OS" = "linux" ]; then
  install_linux
else
  install_macos
fi

echo ""
echo -e "  ${YELLOW}Other platforms or versions?${NC}"
echo "  → $GH/releases"
echo ""