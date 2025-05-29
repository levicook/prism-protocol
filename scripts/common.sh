git_root() {
    git rev-parse --show-toplevel 2>/dev/null || {
        echo "Error: not in a git repository" >&2
        exit 1
    }
}

export PROJECT_NAME="prism-protocol"
export PROJECT_ROOT="$(git_root)"
