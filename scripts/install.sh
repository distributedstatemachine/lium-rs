#!/bin/bash
# Lium CLI Installer
# Usage: curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/install.sh | sh
# Or: wget -qO- https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/install.sh | sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
GITHUB_REPO="distributedstatemachine/lium-rs"
BINARY_NAME="lium"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_API_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"

# Functions
log() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Print banner
print_banner() {
    echo -e "${BOLD}${BLUE}"
    echo "🍄 Lium CLI Installer"
    echo "===================="
    echo -e "${NC}"
    echo "This script will download and install the latest version of lium-cli."
    echo ""
}

# Detect platform and architecture
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)

    case "$os" in
        linux*)
            OS="linux"
            ;;
        darwin*)
            OS="macos"
            ;;
        *)
            error "Unsupported operating system: $os"
            error "Only Linux and macOS are supported."
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            error "Only x86_64 and aarch64 are supported."
            exit 1
            ;;
    esac

    log "Detected platform: ${OS}-${ARCH}"
}

# Get the latest release info
get_latest_release() {
    log "Fetching latest release information..."
    
    if command -v curl >/dev/null 2>&1; then
        RELEASE_DATA=$(curl -s "$GITHUB_API_URL")
    elif command -v wget >/dev/null 2>&1; then
        RELEASE_DATA=$(wget -qO- "$GITHUB_API_URL")
    else
        error "Neither curl nor wget is available. Please install one of them."
        exit 1
    fi

    if [ -z "$RELEASE_DATA" ]; then
        error "Failed to fetch release information"
        exit 1
    fi

    # Extract tag name (version)
    VERSION=$(echo "$RELEASE_DATA" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
    
    if [ -z "$VERSION" ]; then
        error "Could not determine latest version"
        exit 1
    fi

    log "Latest version: $VERSION"
}

# Construct download URL and filename
construct_download_info() {
    # Construct the binary filename based on platform
    case "$OS" in
        linux)
            if [ "$ARCH" = "x86_64" ]; then
                BINARY_FILE="lium-cli-x86_64-unknown-linux-gnu"
            elif [ "$ARCH" = "aarch64" ]; then
                BINARY_FILE="lium-cli-aarch64-unknown-linux-gnu"
            else
                error "Unsupported Linux architecture: $ARCH"
                exit 1
            fi
            ;;
        macos)
            if [ "$ARCH" = "x86_64" ]; then
                BINARY_FILE="lium-cli-x86_64-apple-darwin"
            elif [ "$ARCH" = "aarch64" ]; then
                BINARY_FILE="lium-cli-aarch64-apple-darwin"
            else
                error "Unsupported macOS architecture: $ARCH"
                exit 1
            fi
            ;;
    esac

    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${BINARY_FILE}"
    log "Download URL: $DOWNLOAD_URL"
}

# Check if install directory is in PATH
check_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# Add directory to PATH in shell profile
add_to_path() {
    local shell_profile=""
    
    # Detect shell and set appropriate profile file
    case "$SHELL" in
        */bash)
            shell_profile="$HOME/.bashrc"
            [ -f "$HOME/.bash_profile" ] && shell_profile="$HOME/.bash_profile"
            ;;
        */zsh)
            shell_profile="$HOME/.zshrc"
            ;;
        */fish)
            shell_profile="$HOME/.config/fish/config.fish"
            ;;
        *)
            shell_profile="$HOME/.profile"
            ;;
    esac

    if [ -n "$shell_profile" ]; then
        echo "" >> "$shell_profile"
        echo "# Added by lium-cli installer" >> "$shell_profile"
        echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$shell_profile"
        success "Added $INSTALL_DIR to PATH in $shell_profile"
        warn "Please restart your shell or run: source $shell_profile"
    else
        warn "Could not determine shell profile. Please manually add $INSTALL_DIR to your PATH."
    fi
}

# Create install directory
create_install_dir() {
    if [ ! -d "$INSTALL_DIR" ]; then
        log "Creating install directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
}

# Download and install binary
download_and_install() {
    log "Downloading lium-cli..."
    
    local temp_file=$(mktemp)
    
    if command -v curl >/dev/null 2>&1; then
        if ! curl -L --fail --progress-bar "$DOWNLOAD_URL" -o "$temp_file"; then
            error "Failed to download lium-cli"
            rm -f "$temp_file"
            exit 1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! wget --progress=bar:force "$DOWNLOAD_URL" -O "$temp_file"; then
            error "Failed to download lium-cli"
            rm -f "$temp_file"
            exit 1
        fi
    fi

    # Install the binary
    local install_path="$INSTALL_DIR/$BINARY_NAME"
    log "Installing to: $install_path"
    
    mv "$temp_file" "$install_path"
    chmod +x "$install_path"
    
    success "lium-cli installed successfully!"
}

# Verify installation
verify_installation() {
    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        log "Verifying installation..."
        
        # Check if it's in PATH or run directly
        if check_path; then
            VERSION_OUTPUT=$("$BINARY_NAME" --version 2>/dev/null || echo "")
        else
            VERSION_OUTPUT=$("$INSTALL_DIR/$BINARY_NAME" --version 2>/dev/null || echo "")
        fi
        
        if [ -n "$VERSION_OUTPUT" ]; then
            success "Installation verified: $VERSION_OUTPUT"
        else
            warn "Installation completed but version check failed"
        fi
    else
        error "Installation verification failed"
        exit 1
    fi
}

# Check for existing installation
check_existing() {
    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        local existing_version=$("$BINARY_NAME" --version 2>/dev/null | head -n1 || echo "unknown")
        warn "Existing lium-cli installation found: $existing_version"
        echo "This installer will replace it with the latest version."
        echo ""
    fi
}

# Post-installation setup
post_install() {
    echo ""
    success "🎉 Lium CLI installation complete!"
    echo ""
    echo "Next steps:"
    echo "1. Run 'lium init' to set up your configuration"
    echo "2. Use 'lium ls' to see available GPU executors"
    echo "3. Create your first pod with 'lium up'"
    echo ""
    
    if ! check_path; then
        echo "Note: $INSTALL_DIR is not in your PATH."
        echo "You can either:"
        echo "  - Run commands with full path: $INSTALL_DIR/$BINARY_NAME"
        echo "  - Add to PATH manually: export PATH=\"$INSTALL_DIR:\$PATH\""
        echo "  - Let this installer add it for you"
        echo ""
        
        read -p "Add $INSTALL_DIR to your PATH? [y/N]: " -r
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            add_to_path
        fi
    else
        echo "✅ lium-cli is ready to use!"
    fi
    
    echo ""
    echo "For help, run: lium --help"
    echo "Documentation: https://github.com/distributedstatemachine/lium-rs"
}

# Handle cleanup on exit
cleanup() {
    if [ -n "$temp_file" ] && [ -f "$temp_file" ]; then
        rm -f "$temp_file"
    fi
}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Main installation flow
main() {
    print_banner
    
    # Check for required tools
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        error "Neither curl nor wget is available. Please install one of them first."
        exit 1
    fi
    
    # Handle command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --help|-h)
                echo "Lium CLI Installer"
                echo ""
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --install-dir DIR    Install to specific directory (default: ~/.local/bin)"
                echo "  --help, -h          Show this help message"
                echo ""
                echo "Environment Variables:"
                echo "  INSTALL_DIR         Override default install directory"
                echo ""
                echo "Supported Platforms:"
                echo "  - Linux (x86_64, aarch64)"
                echo "  - macOS (x86_64, aarch64/Apple Silicon)"
                echo ""
                exit 0
                ;;
            *)
                warn "Unknown option: $1"
                shift
                ;;
        esac
    done
    
    check_existing
    detect_platform
    get_latest_release
    construct_download_info
    create_install_dir
    download_and_install
    verify_installation
    post_install
}

# Run main function
main "$@" 