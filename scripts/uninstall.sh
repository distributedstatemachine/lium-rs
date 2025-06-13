#!/bin/bash
# Lium CLI Uninstaller
# Usage: curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/uninstall.sh | sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="lium"
INSTALL_LOCATIONS=(
    "$HOME/.local/bin"
    "/usr/local/bin"
    "/opt/homebrew/bin"
    "/usr/bin"
)

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
    echo "🗑️  Lium CLI Uninstaller"
    echo "======================"
    echo -e "${NC}"
    echo "This script will remove lium-cli from your system."
    echo ""
}

# Find installed binaries
find_installations() {
    FOUND_INSTALLATIONS=()
    
    for location in "${INSTALL_LOCATIONS[@]}"; do
        if [ -f "$location/$BINARY_NAME" ]; then
            FOUND_INSTALLATIONS+=("$location/$BINARY_NAME")
        fi
    done
    
    # Also check what's in PATH
    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        BINARY_PATH=$(which "$BINARY_NAME" 2>/dev/null || true)
        if [ -n "$BINARY_PATH" ] && [ -f "$BINARY_PATH" ]; then
            # Check if it's not already in our list
            local already_found=false
            for installation in "${FOUND_INSTALLATIONS[@]}"; do
                if [ "$installation" = "$BINARY_PATH" ]; then
                    already_found=true
                    break
                fi
            done
            
            if [ "$already_found" = false ]; then
                FOUND_INSTALLATIONS+=("$BINARY_PATH")
            fi
        fi
    fi
}

# Display found installations
display_installations() {
    if [ ${#FOUND_INSTALLATIONS[@]} -eq 0 ]; then
        warn "No lium-cli installations found."
        return 1
    fi
    
    echo "Found lium-cli installations:"
    for i in "${!FOUND_INSTALLATIONS[@]}"; do
        echo "  $((i+1)). ${FOUND_INSTALLATIONS[i]}"
        
        # Try to get version if possible
        if [ -x "${FOUND_INSTALLATIONS[i]}" ]; then
            local version=$("${FOUND_INSTALLATIONS[i]}" --version 2>/dev/null | head -n1 || echo "unknown version")
            echo "     Version: $version"
        fi
    done
    echo ""
}

# Remove binary
remove_binary() {
    local binary_path="$1"
    
    if [ ! -f "$binary_path" ]; then
        warn "Binary not found: $binary_path"
        return 1
    fi
    
    log "Removing: $binary_path"
    
    # Check if we need sudo
    local dir=$(dirname "$binary_path")
    if [ ! -w "$dir" ]; then
        if command -v sudo >/dev/null 2>&1; then
            log "Requires sudo to remove from: $dir"
            if sudo rm -f "$binary_path"; then
                success "Removed: $binary_path"
            else
                error "Failed to remove: $binary_path"
                return 1
            fi
        else
            error "Cannot remove $binary_path - no write permission and sudo not available"
            return 1
        fi
    else
        if rm -f "$binary_path"; then
            success "Removed: $binary_path"
        else
            error "Failed to remove: $binary_path"
            return 1
        fi
    fi
}

# Clean up PATH entries
clean_path_entries() {
    local shell_profiles=(
        "$HOME/.bashrc"
        "$HOME/.bash_profile"
        "$HOME/.zshrc"
        "$HOME/.profile"
        "$HOME/.config/fish/config.fish"
    )
    
    local cleaned_any=false
    
    for profile in "${shell_profiles[@]}"; do
        if [ -f "$profile" ]; then
            # Check if the profile contains lium-cli installer PATH entries
            if grep -q "# Added by lium-cli installer" "$profile" 2>/dev/null; then
                log "Cleaning PATH entries from: $profile"
                
                # Create backup
                cp "$profile" "$profile.lium-backup"
                
                # Remove lium-cli installer lines
                # This removes the comment and the next line (export PATH=...)
                sed -i.tmp '/# Added by lium-cli installer/,+1d' "$profile" 2>/dev/null || {
                    # Fallback for systems where sed -i behaves differently
                    sed '/# Added by lium-cli installer/,+1d' "$profile" > "$profile.tmp" && mv "$profile.tmp" "$profile"
                }
                
                success "Cleaned PATH entries from: $profile"
                log "Backup saved as: $profile.lium-backup"
                cleaned_any=true
            fi
        fi
    done
    
    if [ "$cleaned_any" = true ]; then
        warn "PATH changes will take effect in new shell sessions"
        warn "Or run: source <your-shell-profile>"
    fi
}

# Remove configuration (optional)
remove_config() {
    local config_locations=(
        "$HOME/.config/lium"
        "$HOME/.lium"
    )
    
    local found_config=false
    for config_dir in "${config_locations[@]}"; do
        if [ -d "$config_dir" ]; then
            found_config=true
            break
        fi
    done
    
    if [ "$found_config" = false ]; then
        log "No configuration directories found"
        return 0
    fi
    
    echo "Configuration directories found:"
    for config_dir in "${config_locations[@]}"; do
        if [ -d "$config_dir" ]; then
            echo "  - $config_dir"
        fi
    done
    echo ""
    
    read -p "Remove configuration directories? [y/N]: " -r
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        for config_dir in "${config_locations[@]}"; do
            if [ -d "$config_dir" ]; then
                log "Removing configuration: $config_dir"
                rm -rf "$config_dir"
                success "Removed: $config_dir"
            fi
        done
    else
        log "Skipping configuration removal"
    fi
}

# Verify removal
verify_removal() {
    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        warn "lium-cli is still available in PATH"
        local remaining_path=$(which "$BINARY_NAME" 2>/dev/null || echo "unknown")
        warn "Remaining installation: $remaining_path"
        return 1
    else
        success "lium-cli successfully removed from PATH"
        return 0
    fi
}

# Main uninstall flow
main() {
    print_banner
    
    # Handle command line arguments
    local skip_confirm=false
    while [[ $# -gt 0 ]]; do
        case $1 in
            --yes|-y)
                skip_confirm=true
                shift
                ;;
            --help|-h)
                echo "Lium CLI Uninstaller"
                echo ""
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --yes, -y      Skip confirmation prompts"
                echo "  --help, -h     Show this help message"
                echo ""
                exit 0
                ;;
            *)
                warn "Unknown option: $1"
                shift
                ;;
        esac
    done
    
    # Find installations
    find_installations
    
    if [ ${#FOUND_INSTALLATIONS[@]} -eq 0 ]; then
        warn "No lium-cli installations found on this system."
        exit 0
    fi
    
    # Display what will be removed
    display_installations
    
    # Confirm removal
    if [ "$skip_confirm" = false ]; then
        read -p "Remove all lium-cli installations? [y/N]: " -r
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log "Uninstall cancelled"
            exit 0
        fi
    fi
    
    echo ""
    log "Starting uninstall process..."
    
    # Remove each installation
    local removed_count=0
    for installation in "${FOUND_INSTALLATIONS[@]}"; do
        if remove_binary "$installation"; then
            ((removed_count++))
        fi
    done
    
    # Clean up PATH entries
    clean_path_entries
    
    # Optionally remove configuration
    echo ""
    remove_config
    
    # Verify removal
    echo ""
    if verify_removal; then
        echo ""
        success "🎉 Lium CLI completely removed!"
        success "Removed $removed_count installation(s)"
        
        echo ""
        echo "To reinstall lium-cli, run:"
        echo "curl -sSL https://raw.githubusercontent.com/distributedstatemachine/lium-rs/main/scripts/install.sh | sh"
        
    else
        echo ""
        warn "Uninstall completed but some installations may remain"
        warn "You may need to manually remove remaining installations"
    fi
}

# Run main function
main "$@" 